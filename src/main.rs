use anyhow::{anyhow, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use colored::Colorize;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use lambda_cli::api::{LambdaClient, LambdaError};
use prettytable::{row, Table};
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

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
        /// Filesystem name to attach (must be in same region)
        #[arg(short, long)]
        filesystem: Option<String>,
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
        /// Filesystem name to attach when launched (must be in same region)
        #[arg(short, long)]
        filesystem: Option<String>,
    },
    /// List all filesystems (persistent storage)
    Filesystems,
    /// Create a new filesystem
    CreateFilesystem {
        /// Name for the filesystem
        #[arg(short, long)]
        name: String,
        /// Region to create the filesystem in
        #[arg(short, long)]
        region: String,
    },
    /// Delete a filesystem
    DeleteFilesystem {
        /// Filesystem ID to delete
        #[arg(short = 'i', long)]
        filesystem_id: String,
    },
}

fn main() {
    dotenv::dotenv().ok();

    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let rt = Runtime::new()?;
    let client = LambdaClient::from_env()?;

    match &cli.command {
        Some(Commands::List) => list_instances(&rt, &client),
        Some(Commands::Start {
            gpu,
            ssh,
            name,
            region,
            filesystem,
        }) => start_instance(
            &rt,
            &client,
            gpu,
            ssh,
            name.as_deref(),
            region.as_deref(),
            filesystem.as_deref(),
        ),
        Some(Commands::Stop { instance_id }) => stop_instance(&rt, &client, instance_id),
        Some(Commands::Running) => list_running_instances(&rt, &client),
        Some(Commands::Find {
            gpu,
            ssh,
            interval,
            name,
            filesystem,
        }) => find_and_start_instance(
            &rt,
            &client,
            gpu,
            ssh,
            *interval,
            name.as_deref(),
            filesystem.as_deref(),
        ),
        Some(Commands::Filesystems) => list_filesystems(&rt, &client),
        Some(Commands::CreateFilesystem { name, region }) => {
            create_filesystem(&rt, &client, name, region)
        }
        Some(Commands::DeleteFilesystem { filesystem_id }) => {
            delete_filesystem(&rt, &client, filesystem_id)
        }
        None => validate_api_key(&rt, &client),
    }
}

fn validate_api_key(rt: &Runtime, client: &LambdaClient) -> Result<()> {
    rt.block_on(client.validate_api_key())?;
    println!("{}", "API key is valid".green());
    Ok(())
}

fn list_instances(rt: &Runtime, client: &LambdaClient) -> Result<()> {
    let types = rt.block_on(client.list_instance_types())?;

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

    for t in types {
        let availability = if t.regions_available.is_empty() {
            "None".red().to_string()
        } else {
            t.regions_available.join(", ").blue().to_string()
        };

        let price = format!("${:.2}", t.price_cents_per_hour as f64 / 100.0);

        table.add_row(row![
            if t.regions_available.is_empty() {
                t.name.dimmed().to_string()
            } else {
                t.name.green().to_string()
            },
            t.description,
            price.yellow(),
            t.vcpus,
            t.memory_gib,
            t.storage_gib,
            availability
        ]);
    }

    table.printstd();
    Ok(())
}

fn start_instance(
    rt: &Runtime,
    client: &LambdaClient,
    gpu: &str,
    ssh: &str,
    name: Option<&str>,
    region: Option<&str>,
    filesystem: Option<&str>,
) -> Result<()> {
    let fs_info = filesystem
        .map(|f| format!(" with filesystem '{}'", f.magenta()))
        .unwrap_or_default();
    println!(
        "Launching {} {}{}...",
        gpu.green(),
        name.map(|n| format!("as '{}'", n.cyan()))
            .unwrap_or_default(),
        fs_info
    );

    let result = rt.block_on(client.launch_instance_with_filesystem(
        gpu, ssh, name, region, filesystem,
    ))?;

    println!(
        "{} Instance {} launched in region {}",
        "Success!".green().bold(),
        result.instance_id.cyan(),
        result.region.blue()
    );
    println!("Waiting for instance to become active...");

    // Poll for instance to become ready
    let start_time = Instant::now();
    let max_wait = Duration::from_secs(300);
    let poll_interval = Duration::from_secs(10);

    loop {
        if start_time.elapsed() > max_wait {
            println!(
                "{} Instance may still be starting. Check status with: lambda running",
                "Timeout:".yellow()
            );
            break;
        }

        std::thread::sleep(poll_interval);

        match rt.block_on(client.get_instance(&result.instance_id)) {
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
                print!("\r{} Waiting for instance...    ", "Polling...".dimmed());
                stdout().flush().ok();
                eprintln!("\nWarning: {}", e);
            }
        }
    }

    Ok(())
}

fn stop_instance(rt: &Runtime, client: &LambdaClient, instance_id: &str) -> Result<()> {
    println!("Terminating instance {}...", instance_id.cyan());

    rt.block_on(client.terminate_instance(instance_id))?;

    println!(
        "{} Instance {} terminated",
        "Success!".green().bold(),
        instance_id.cyan()
    );
    Ok(())
}

fn list_running_instances(rt: &Runtime, client: &LambdaClient) -> Result<()> {
    let instances = rt.block_on(client.list_running_instances())?;

    if instances.is_empty() {
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

    for instance in instances {
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
    rt: &Runtime,
    client: &LambdaClient,
    gpu: &str,
    ssh: &str,
    interval: u64,
    name: Option<&str>,
    filesystem: Option<&str>,
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

    let mut first_check = true;

    loop {
        if !first_check {
            std::thread::sleep(Duration::from_secs(interval));
        }
        first_check = false;

        let check_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        match rt.block_on(client.check_availability(gpu)) {
            Ok(regions) if !regions.is_empty() => {
                execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).ok();
                println!(
                    "{} Found {} available in: {}",
                    "SUCCESS!".green().bold(),
                    gpu.green(),
                    regions.join(", ").blue()
                );

                return start_instance(rt, client, gpu, ssh, name, None, filesystem);
            }
            Ok(_) => {
                // No availability
            }
            Err(e) => {
                if e.to_string().contains("not found") {
                    return Err(LambdaError::InstanceTypeNotFound(gpu.to_string()).into());
                }
                eprintln!(
                    "{} Failed to check availability: {}",
                    "Warning:".yellow(),
                    e
                );
                continue;
            }
        }

        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).ok();
        let mut table = Table::new();
        table.add_row(row!["Instance Type", "Last Checked", "Status"]);
        table.add_row(row![gpu.green(), check_time, "No availability".red()]);
        table.printstd();
        println!("\nNext check in {} seconds... (Ctrl+C to stop)", interval);
    }
}

fn list_filesystems(rt: &Runtime, client: &LambdaClient) -> Result<()> {
    let filesystems = rt.block_on(client.list_filesystems())?;

    if filesystems.is_empty() {
        println!("{}", "No filesystems".yellow());
        return Ok(());
    }

    let mut table = Table::new();
    table.add_row(row![
        "ID",
        "Name",
        "Region",
        "Mount Point",
        "In Use",
        "Bytes Used",
        "Created"
    ]);

    for fs in filesystems {
        let in_use = if fs.is_in_use {
            "Yes".green().to_string()
        } else {
            "No".dimmed().to_string()
        };

        let bytes_str = if fs.bytes_used > 1_000_000_000 {
            format!("{:.2} GB", fs.bytes_used as f64 / 1_000_000_000.0)
        } else if fs.bytes_used > 1_000_000 {
            format!("{:.2} MB", fs.bytes_used as f64 / 1_000_000.0)
        } else if fs.bytes_used > 1_000 {
            format!("{:.2} KB", fs.bytes_used as f64 / 1_000.0)
        } else {
            format!("{} B", fs.bytes_used)
        };

        table.add_row(row![
            fs.id.cyan(),
            fs.name.green(),
            fs.region.name.blue(),
            fs.mount_point,
            in_use,
            bytes_str,
            fs.created
        ]);
    }

    table.printstd();
    Ok(())
}

fn create_filesystem(rt: &Runtime, client: &LambdaClient, name: &str, region: &str) -> Result<()> {
    println!(
        "Creating filesystem '{}' in region {}...",
        name.green(),
        region.blue()
    );

    let fs = rt.block_on(client.create_filesystem(name, region))?;

    println!(
        "{} Filesystem '{}' created",
        "Success!".green().bold(),
        fs.name.green()
    );
    println!("  ID: {}", fs.id.cyan());
    println!("  Mount point: {}", fs.mount_point);
    println!("  Region: {}", fs.region.name.blue());

    Ok(())
}

fn delete_filesystem(rt: &Runtime, client: &LambdaClient, filesystem_id: &str) -> Result<()> {
    println!("Deleting filesystem {}...", filesystem_id.cyan());

    rt.block_on(client.delete_filesystem(filesystem_id))?;

    println!(
        "{} Filesystem {} deleted",
        "Success!".green().bold(),
        filesystem_id.cyan()
    );
    Ok(())
}
