use anyhow::{anyhow, Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use colored::Colorize;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use prettytable::{row, Table};
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::io::{stdout, Write};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const API_BASE_URL: &str = "https://cloud.lambdalabs.com/api/v1";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Error, Debug)]
pub enum LambdaError {
    #[error("API key not set. Please set LAMBDA_API_KEY environment variable")]
    ApiKeyNotSet,
    #[error("Instance type '{0}' not found")]
    InstanceTypeNotFound(String),
    #[error("No regions available for instance type '{0}'")]
    NoRegionsAvailable(String),
    #[error("No instance IDs returned from launch request")]
    NoInstanceIds,
    #[error("API request failed: {0}")]
    ApiError(String),
    #[error("SSH key is required for this operation")]
    SshKeyRequired,
}

/// A command-line tool for Lambda Labs cloud GPU API
#[derive(Parser)]
#[command(name = "lambda")]
#[command(version = "0.2.0")]
#[command(about = "A command-line tool for Lambda Labs cloud GPU API", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available GPU instances
    List,
    /// Start a GPU instance with the specified SSH key
    Start {
        /// GPU instance type (e.g., gpu_1x_a100)
        #[arg(short, long)]
        gpu: String,
        /// SSH key name to use for the instance
        #[arg(short, long)]
        ssh: String,
        /// Optional name for the instance
        #[arg(short, long)]
        name: Option<String>,
        /// Region to launch in (auto-selects first available if not specified)
        #[arg(short, long)]
        region: Option<String>,
    },
    /// Stop a specified GPU instance
    Stop {
        /// Instance ID to terminate
        #[arg(short = 'i', long)]
        instance_id: String,
    },
    /// List all running GPU instances
    Running,
    /// Continuously find and start a GPU instance when it becomes available
    Find {
        /// GPU instance type to find
        #[arg(short, long)]
        gpu: String,
        /// SSH key name to use when launching
        #[arg(short, long)]
        ssh: String,
        /// Polling interval in seconds
        #[arg(long, default_value_t = 10)]
        interval: u64,
        /// Optional name for the instance when launched
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Rename an existing instance
    Rename {
        /// Instance ID to rename
        #[arg(short = 'i', long)]
        instance_id: String,
        /// New name for the instance
        #[arg(short, long)]
        name: String,
    },
}

#[derive(Deserialize, Debug)]
struct ApiResponse<T> {
    data: T,
}

#[derive(Deserialize, Debug)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Deserialize, Debug)]
struct ApiErrorDetail {
    message: String,
}

#[derive(Deserialize, Debug)]
struct Instance {
    id: Option<String>,
    name: Option<String>,
    status: Option<String>,
    ip: Option<String>,
    ssh_key_names: Option<Vec<String>>,
    instance_type: Option<InstanceTypeInfo>,
    region: Option<RegionInfo>,
}

#[derive(Deserialize, Debug)]
struct InstanceTypeInfo {
    name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RegionInfo {
    name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct LaunchResponse {
    instance_ids: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct InstanceTypeResponse {
    instance_type: InstanceType,
    regions_with_capacity_available: Vec<Region>,
}

#[derive(Deserialize, Debug, Clone)]
struct InstanceType {
    description: String,
    price_cents_per_hour: i32,
    specs: InstanceSpecs,
}

#[derive(Deserialize, Debug, Clone)]
struct InstanceSpecs {
    vcpus: u32,
    memory_gib: u32,
    storage_gib: u32,
}

#[derive(Deserialize, Debug, Clone)]
struct Region {
    name: String,
    #[allow(dead_code)]
    description: String,
}

fn create_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")
}

fn get_api_key() -> Result<String> {
    env::var("LAMBDA_API_KEY").map_err(|_| LambdaError::ApiKeyNotSet.into())
}

fn main() {
    dotenv::dotenv().ok();

    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Parse CLI first so --help works without API key
    let cli = Cli::parse();

    // Now get API key and client (needed for all commands)
    let api_key = get_api_key()?;
    let client = create_client()?;

    match &cli.command {
        Some(Commands::List) => list_instances(&client, &api_key),
        Some(Commands::Start {
            gpu,
            ssh,
            name,
            region,
        }) => start_instance(
            &client,
            &api_key,
            gpu,
            ssh,
            name.as_deref(),
            region.as_deref(),
        ),
        Some(Commands::Stop { instance_id }) => stop_instance(&client, &api_key, instance_id),
        Some(Commands::Running) => list_running_instances(&client, &api_key),
        Some(Commands::Find {
            gpu,
            ssh,
            interval,
            name,
        }) => find_and_start_instance(&client, &api_key, gpu, ssh, *interval, name.as_deref()),
        Some(Commands::Rename { instance_id, name }) => {
            rename_instance(&client, &api_key, instance_id, name)
        }
        None => validate_api_key(&client, &api_key),
    }
}

fn validate_api_key(client: &Client, api_key: &str) -> Result<()> {
    let url = format!("{}/instances", API_BASE_URL);
    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .context("Failed to connect to Lambda Labs API")?;

    if response.status().is_success() {
        println!("{}", "API key is valid".green());
        Ok(())
    } else {
        let status = response.status();
        let error_msg = parse_error_response(response);
        Err(anyhow!(
            "API key validation failed ({}): {}",
            status,
            error_msg
        ))
    }
}

fn parse_error_response(response: reqwest::blocking::Response) -> String {
    response
        .json::<ApiErrorResponse>()
        .map(|e| e.error.message)
        .unwrap_or_else(|_| "Unknown error".to_string())
}

fn list_instances(client: &Client, api_key: &str) -> Result<()> {
    let url = format!("{}/instance-types", API_BASE_URL);
    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .context("Failed to fetch instance types")?;

    if !response.status().is_success() {
        let error_msg = parse_error_response(response);
        return Err(anyhow!("Failed to list instances: {}", error_msg));
    }

    let response: ApiResponse<HashMap<String, InstanceTypeResponse>> = response
        .json()
        .context("Failed to parse instance types response")?;

    let mut table = Table::new();
    table.add_row(row![
        "Instance Type",
        "Description",
        "Price ($/hr)",
        "vCPUs",
        "Memory (GiB)",
        "Storage (GiB)",
        "Available Regions"
    ]);

    let mut entries: Vec<_> = response.data.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (key, instance_type_response) in entries {
        let regions: Vec<String> = instance_type_response
            .regions_with_capacity_available
            .iter()
            .map(|region| region.name.clone())
            .collect();

        let availability = if regions.is_empty() {
            "None".red().to_string()
        } else {
            regions.join(", ").blue().to_string()
        };

        let price = format!(
            "${:.2}",
            instance_type_response.instance_type.price_cents_per_hour as f64 / 100.0
        );

        table.add_row(row![
            if regions.is_empty() {
                key.dimmed().to_string()
            } else {
                key.green().to_string()
            },
            instance_type_response.instance_type.description,
            price.yellow(),
            instance_type_response.instance_type.specs.vcpus,
            instance_type_response.instance_type.specs.memory_gib,
            instance_type_response.instance_type.specs.storage_gib,
            availability
        ]);
    }

    table.printstd();
    Ok(())
}

fn start_instance(
    client: &Client,
    api_key: &str,
    gpu: &str,
    ssh: &str,
    name: Option<&str>,
    region: Option<&str>,
) -> Result<()> {
    let instance_type_response = get_instance_type_response(client, api_key, gpu)?
        .ok_or_else(|| LambdaError::InstanceTypeNotFound(gpu.to_string()))?;

    let region_name = if let Some(r) = region {
        // Validate the specified region is available
        if !instance_type_response
            .regions_with_capacity_available
            .iter()
            .any(|reg| reg.name == r)
        {
            return Err(anyhow!(
                "Region '{}' is not available for instance type '{}'. Available regions: {}",
                r,
                gpu,
                instance_type_response
                    .regions_with_capacity_available
                    .iter()
                    .map(|reg| reg.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        r.to_string()
    } else {
        // Auto-select first available region
        instance_type_response
            .regions_with_capacity_available
            .first()
            .ok_or_else(|| LambdaError::NoRegionsAvailable(gpu.to_string()))?
            .name
            .clone()
    };

    let url = format!("{}/instance-operations/launch", API_BASE_URL);

    // Build payload - include name if provided
    let mut payload = serde_json::json!({
        "region_name": region_name,
        "instance_type_name": gpu,
        "ssh_key_names": [ssh],
        "quantity": 1
    });

    if let Some(instance_name) = name {
        payload["name"] = serde_json::Value::String(instance_name.to_string());
    }

    println!(
        "Launching {} in {}{}...",
        gpu.green(),
        region_name.blue(),
        name.map(|n| format!(" as '{}'", n.cyan()))
            .unwrap_or_default()
    );

    let response = client
        .post(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .context("Failed to send launch request")?;

    if !response.status().is_success() {
        let error_msg = parse_error_response(response);
        return Err(anyhow!("Failed to launch instance: {}", error_msg));
    }

    let response_text = response.text().context("Failed to read response")?;
    let parsed_response: ApiResponse<LaunchResponse> =
        serde_json::from_str(&response_text).context("Failed to parse launch response")?;

    let instance_id = parsed_response
        .data
        .instance_ids
        .first()
        .ok_or(LambdaError::NoInstanceIds)?;

    println!(
        "{} Instance {} launched in region {}",
        "Success!".green().bold(),
        instance_id.cyan(),
        region_name.blue()
    );
    println!("Waiting for instance to become active...");

    // Poll for instance to become ready instead of fixed sleep
    let start_time = Instant::now();
    let max_wait = Duration::from_secs(300); // 5 minutes max
    let poll_interval = Duration::from_secs(10);

    loop {
        if start_time.elapsed() > max_wait {
            println!(
                "{} Instance may still be starting. Check status with: lambda running",
                "Timeout:".yellow()
            );
            break;
        }

        thread::sleep(poll_interval);

        match get_instance_details(client, api_key, instance_id) {
            Ok(instance) => {
                let status = instance.status.as_deref().unwrap_or("unknown");
                print!(
                    "\r{} Status: {}    ",
                    "Polling...".dimmed(),
                    status.yellow()
                );
                stdout().flush().ok();

                if status == "active" {
                    println!();
                    if let Some(ip) = instance.ip {
                        println!(
                            "{} Instance is active! SSH: {}",
                            "Ready!".green().bold(),
                            format!("ssh ubuntu@{}", ip).cyan()
                        );
                    } else {
                        println!(
                            "{} Instance is active but IP not yet assigned",
                            "Ready!".green().bold()
                        );
                    }
                    break;
                } else if status == "terminated" || status == "unhealthy" {
                    println!();
                    return Err(anyhow!("Instance entered {} state", status));
                }
            }
            Err(e) => {
                // Instance might not be queryable yet, continue polling
                print!("\r{} Waiting for instance...    ", "Polling...".dimmed());
                stdout().flush().ok();
                eprintln!("\nWarning: {}", e);
            }
        }
    }

    Ok(())
}

fn get_instance_type_response(
    client: &Client,
    api_key: &str,
    gpu: &str,
) -> Result<Option<InstanceTypeResponse>> {
    let url = format!("{}/instance-types", API_BASE_URL);
    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .context("Failed to fetch instance types")?;

    if !response.status().is_success() {
        let error_msg = parse_error_response(response);
        return Err(anyhow!("Failed to get instance types: {}", error_msg));
    }

    let response: ApiResponse<HashMap<String, InstanceTypeResponse>> =
        response.json().context("Failed to parse instance types")?;

    Ok(response.data.get(gpu).cloned())
}

fn stop_instance(client: &Client, api_key: &str, instance_id: &str) -> Result<()> {
    let url = format!("{}/instance-operations/terminate", API_BASE_URL);
    let payload = serde_json::json!({
        "instance_ids": [instance_id]
    });

    println!("Terminating instance {}...", instance_id.cyan());

    let response = client
        .post(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .context("Failed to send terminate request")?;

    if !response.status().is_success() {
        let error_msg = parse_error_response(response);
        return Err(anyhow!("Failed to terminate instance: {}", error_msg));
    }

    println!(
        "{} Instance {} terminated",
        "Success!".green().bold(),
        instance_id.cyan()
    );
    Ok(())
}

fn list_running_instances(client: &Client, api_key: &str) -> Result<()> {
    let url = format!("{}/instances", API_BASE_URL);
    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .context("Failed to fetch running instances")?;

    if !response.status().is_success() {
        let error_msg = parse_error_response(response);
        return Err(anyhow!("Failed to list running instances: {}", error_msg));
    }

    let response: ApiResponse<Vec<Instance>> = response
        .json()
        .context("Failed to parse running instances response")?;

    if response.data.is_empty() {
        println!("{}", "No running instances".yellow());
        return Ok(());
    }

    let mut table = Table::new();
    table.add_row(row![
        "Instance ID",
        "Name",
        "Type",
        "Region",
        "Status",
        "IP Address",
        "SSH Keys"
    ]);

    for instance in response.data {
        let status = instance.status.as_deref().unwrap_or("N/A");
        let status_colored = match status {
            "active" => status.green().to_string(),
            "booting" => status.yellow().to_string(),
            "unhealthy" | "terminated" => status.red().to_string(),
            _ => status.to_string(),
        };

        table.add_row(row![
            instance.id.unwrap_or_else(|| "N/A".to_string()).cyan(),
            instance.name.unwrap_or_else(|| "-".to_string()).white(),
            instance
                .instance_type
                .and_then(|t| t.name)
                .unwrap_or_else(|| "N/A".to_string()),
            instance
                .region
                .and_then(|r| r.name)
                .unwrap_or_else(|| "N/A".to_string()),
            status_colored,
            instance.ip.unwrap_or_else(|| "N/A".to_string()).blue(),
            instance
                .ssh_key_names
                .map(|keys| keys.join(", "))
                .unwrap_or_else(|| "N/A".to_string())
        ]);
    }

    table.printstd();
    Ok(())
}

fn find_and_start_instance(
    client: &Client,
    api_key: &str,
    gpu: &str,
    ssh: &str,
    interval: u64,
    name: Option<&str>,
) -> Result<()> {
    if ssh.is_empty() {
        return Err(LambdaError::SshKeyRequired.into());
    }

    println!(
        "Looking for available {} instances (polling every {}s)...",
        gpu.green(),
        interval
    );
    println!("Press Ctrl+C to stop\n");

    // Do first check immediately
    let mut first_check = true;

    loop {
        if !first_check {
            thread::sleep(Duration::from_secs(interval));
        }
        first_check = false;

        let check_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        match get_instance_type_response(client, api_key, gpu) {
            Ok(Some(instance_type_response)) => {
                if !instance_type_response
                    .regions_with_capacity_available
                    .is_empty()
                {
                    let regions: Vec<String> = instance_type_response
                        .regions_with_capacity_available
                        .iter()
                        .map(|region| region.name.clone())
                        .collect();

                    // Clear screen and show success
                    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).ok();
                    println!(
                        "{} Found {} available in: {}",
                        "SUCCESS!".green().bold(),
                        gpu.green(),
                        regions.join(", ").blue()
                    );

                    return start_instance(client, api_key, gpu, ssh, name, None);
                }
            }
            Ok(None) => {
                return Err(LambdaError::InstanceTypeNotFound(gpu.to_string()).into());
            }
            Err(e) => {
                eprintln!(
                    "{} Failed to check availability: {}",
                    "Warning:".yellow(),
                    e
                );
                continue;
            }
        }

        // Update status display
        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).ok();
        let mut table = Table::new();
        table.add_row(row!["Instance Type", "Last Checked", "Status"]);
        table.add_row(row![gpu.green(), check_time, "No availability".red()]);
        table.printstd();
        println!("\nNext check in {} seconds... (Ctrl+C to stop)", interval);
    }
}

fn get_instance_details(client: &Client, api_key: &str, instance_id: &str) -> Result<Instance> {
    let url = format!("{}/instances/{}", API_BASE_URL, instance_id);
    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .context("Failed to fetch instance details")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_msg = parse_error_response(response);
        return Err(anyhow!(
            "Failed to get instance details ({}): {}",
            status,
            error_msg
        ));
    }

    let response: ApiResponse<Instance> = response
        .json()
        .context("Failed to parse instance details")?;

    Ok(response.data)
}

fn rename_instance(client: &Client, api_key: &str, instance_id: &str, name: &str) -> Result<()> {
    // Note: This endpoint may not exist in Lambda Labs API
    // If it doesn't work, we'll get a clear error message
    let url = format!("{}/instances/{}", API_BASE_URL, instance_id);
    let payload = serde_json::json!({
        "name": name
    });

    println!(
        "Renaming instance {} to '{}'...",
        instance_id.cyan(),
        name.green()
    );

    let response = client
        .patch(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .context("Failed to send rename request")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_msg = parse_error_response(response);

        if status.as_u16() == 404 || status.as_u16() == 405 {
            return Err(anyhow!(
                "Instance renaming is not supported by the Lambda Labs API. \
                You can set a name when launching with: lambda start --gpu <type> --ssh <key> --name <name>"
            ));
        }

        return Err(anyhow!("Failed to rename instance: {}", error_msg));
    }

    println!(
        "{} Instance {} renamed to '{}'",
        "Success!".green().bold(),
        instance_id.cyan(),
        name.green()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test API response parsing
    #[test]
    fn test_parse_instance_response() {
        let json = r#"{
            "data": {
                "id": "inst-123",
                "name": "my-instance",
                "status": "active",
                "ip": "192.168.1.1",
                "ssh_key_names": ["key1", "key2"],
                "instance_type": {"name": "gpu_1x_a100"},
                "region": {"name": "us-west-1"}
            }
        }"#;

        let response: ApiResponse<Instance> = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.id, Some("inst-123".to_string()));
        assert_eq!(response.data.name, Some("my-instance".to_string()));
        assert_eq!(response.data.status, Some("active".to_string()));
        assert_eq!(response.data.ip, Some("192.168.1.1".to_string()));
        assert_eq!(
            response.data.ssh_key_names,
            Some(vec!["key1".to_string(), "key2".to_string()])
        );
    }

    #[test]
    fn test_parse_instance_response_with_nulls() {
        let json = r#"{
            "data": {
                "id": "inst-123",
                "status": "booting"
            }
        }"#;

        let response: ApiResponse<Instance> = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.id, Some("inst-123".to_string()));
        assert_eq!(response.data.name, None);
        assert_eq!(response.data.ip, None);
        assert_eq!(response.data.ssh_key_names, None);
    }

    #[test]
    fn test_parse_launch_response() {
        let json = r#"{
            "data": {
                "instance_ids": ["inst-abc", "inst-def"]
            }
        }"#;

        let response: ApiResponse<LaunchResponse> = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.instance_ids.len(), 2);
        assert_eq!(response.data.instance_ids[0], "inst-abc");
        assert_eq!(response.data.instance_ids[1], "inst-def");
    }

    #[test]
    fn test_parse_instance_types_response() {
        let json = r#"{
            "data": {
                "gpu_1x_a100": {
                    "instance_type": {
                        "description": "1x A100 GPU",
                        "price_cents_per_hour": 110,
                        "specs": {
                            "vcpus": 24,
                            "memory_gib": 200,
                            "storage_gib": 512
                        }
                    },
                    "regions_with_capacity_available": [
                        {"name": "us-west-1", "description": "US West"},
                        {"name": "us-east-1", "description": "US East"}
                    ]
                }
            }
        }"#;

        let response: ApiResponse<HashMap<String, InstanceTypeResponse>> =
            serde_json::from_str(json).unwrap();

        assert!(response.data.contains_key("gpu_1x_a100"));
        let instance_type = response.data.get("gpu_1x_a100").unwrap();
        assert_eq!(instance_type.instance_type.description, "1x A100 GPU");
        assert_eq!(instance_type.instance_type.price_cents_per_hour, 110);
        assert_eq!(instance_type.instance_type.specs.vcpus, 24);
        assert_eq!(instance_type.instance_type.specs.memory_gib, 200);
        assert_eq!(instance_type.regions_with_capacity_available.len(), 2);
        assert_eq!(
            instance_type.regions_with_capacity_available[0].name,
            "us-west-1"
        );
    }

    #[test]
    fn test_parse_empty_regions() {
        let json = r#"{
            "data": {
                "gpu_8x_h100": {
                    "instance_type": {
                        "description": "8x H100 GPU",
                        "price_cents_per_hour": 2400,
                        "specs": {
                            "vcpus": 208,
                            "memory_gib": 1800,
                            "storage_gib": 20000
                        }
                    },
                    "regions_with_capacity_available": []
                }
            }
        }"#;

        let response: ApiResponse<HashMap<String, InstanceTypeResponse>> =
            serde_json::from_str(json).unwrap();

        let instance_type = response.data.get("gpu_8x_h100").unwrap();
        assert!(instance_type.regions_with_capacity_available.is_empty());
    }

    #[test]
    fn test_parse_api_error_response() {
        let json = r#"{
            "error": {
                "message": "Invalid API key"
            }
        }"#;

        let response: ApiErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.error.message, "Invalid API key");
    }

    // Test error types
    #[test]
    fn test_lambda_error_messages() {
        assert_eq!(
            LambdaError::ApiKeyNotSet.to_string(),
            "API key not set. Please set LAMBDA_API_KEY environment variable"
        );
        assert_eq!(
            LambdaError::InstanceTypeNotFound("gpu_1x_a100".to_string()).to_string(),
            "Instance type 'gpu_1x_a100' not found"
        );
        assert_eq!(
            LambdaError::NoRegionsAvailable("gpu_1x_a100".to_string()).to_string(),
            "No regions available for instance type 'gpu_1x_a100'"
        );
        assert_eq!(
            LambdaError::NoInstanceIds.to_string(),
            "No instance IDs returned from launch request"
        );
        assert_eq!(
            LambdaError::SshKeyRequired.to_string(),
            "SSH key is required for this operation"
        );
    }

    // Test URL construction
    #[test]
    fn test_api_base_url() {
        assert_eq!(API_BASE_URL, "https://cloud.lambdalabs.com/api/v1");
    }

    #[test]
    fn test_url_construction() {
        let instance_id = "inst-123";
        let url = format!("{}/instances/{}", API_BASE_URL, instance_id);
        assert_eq!(
            url,
            "https://cloud.lambdalabs.com/api/v1/instances/inst-123"
        );
    }

    // Test client creation
    #[test]
    fn test_create_client() {
        let client = create_client();
        assert!(client.is_ok());
    }

    // Test JSON payload construction
    #[test]
    fn test_launch_payload_without_name() {
        let payload = serde_json::json!({
            "region_name": "us-west-1",
            "instance_type_name": "gpu_1x_a100",
            "ssh_key_names": ["my-key"],
            "quantity": 1
        });

        assert_eq!(payload["region_name"], "us-west-1");
        assert_eq!(payload["instance_type_name"], "gpu_1x_a100");
        assert_eq!(payload["quantity"], 1);
        assert!(payload.get("name").is_none());
    }

    #[test]
    fn test_launch_payload_with_name() {
        let mut payload = serde_json::json!({
            "region_name": "us-west-1",
            "instance_type_name": "gpu_1x_a100",
            "ssh_key_names": ["my-key"],
            "quantity": 1
        });

        let instance_name = "my-training-job";
        payload["name"] = serde_json::Value::String(instance_name.to_string());

        assert_eq!(payload["name"], "my-training-job");
    }

    #[test]
    fn test_terminate_payload() {
        let instance_id = "inst-123";
        let payload = serde_json::json!({
            "instance_ids": [instance_id]
        });

        let ids = payload["instance_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "inst-123");
    }

    // Test InstanceTypeResponse cloning (needed for the lookup function)
    #[test]
    fn test_instance_type_response_clone() {
        let response = InstanceTypeResponse {
            instance_type: InstanceType {
                description: "Test GPU".to_string(),
                price_cents_per_hour: 100,
                specs: InstanceSpecs {
                    vcpus: 8,
                    memory_gib: 64,
                    storage_gib: 256,
                },
            },
            regions_with_capacity_available: vec![Region {
                name: "us-west-1".to_string(),
                description: "US West".to_string(),
            }],
        };

        let cloned = response.clone();
        assert_eq!(cloned.instance_type.description, "Test GPU");
        assert_eq!(cloned.regions_with_capacity_available.len(), 1);
    }

    // Test safe array access patterns
    #[test]
    fn test_first_on_empty_vec() {
        let empty: Vec<String> = vec![];
        assert!(empty.first().is_none());
    }

    #[test]
    fn test_first_on_non_empty_vec() {
        let vec = vec!["first".to_string(), "second".to_string()];
        assert_eq!(vec.first(), Some(&"first".to_string()));
    }

    // Test default timeout value
    #[test]
    fn test_default_timeout() {
        assert_eq!(DEFAULT_TIMEOUT_SECS, 30);
    }
}
