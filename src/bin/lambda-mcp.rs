use anyhow::Result;
use lambda_cli::api::{Instance, InstanceTypeData, LambdaClient};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::schemars::JsonSchema;
use rmcp::serde::Deserialize;
use rmcp::{tool, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use std::sync::Arc;

/// Lambda Labs MCP Server
#[derive(Clone)]
struct LambdaService {
    client: Arc<LambdaClient>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl LambdaService {
    fn new() -> Result<Self> {
        dotenv::dotenv().ok();
        let client = LambdaClient::from_env()?;
        Ok(Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        })
    }

    fn format_instance_types(types: &[InstanceTypeData]) -> String {
        let mut output = String::from("Available GPU Instance Types:\n\n");
        for t in types {
            let price = t.price_cents_per_hour as f64 / 100.0;
            let availability = if t.regions_available.is_empty() {
                "No availability".to_string()
            } else {
                t.regions_available.join(", ")
            };
            output.push_str(&format!(
                "• {} - {}\n  Price: ${:.2}/hr | vCPUs: {} | RAM: {} GiB | Storage: {} GiB\n  Regions: {}\n\n",
                t.name, t.description, price, t.vcpus, t.memory_gib, t.storage_gib, availability
            ));
        }
        output
    }

    fn format_instances(instances: &[Instance]) -> String {
        if instances.is_empty() {
            return "No running instances.".to_string();
        }
        let mut output = String::from("Running Instances:\n\n");
        for inst in instances {
            let id = inst.id.as_deref().unwrap_or("N/A");
            let name = inst.name.as_deref().unwrap_or("-");
            let status = inst.status.as_deref().unwrap_or("unknown");
            let ip = inst.ip.as_deref().unwrap_or("N/A");
            let inst_type = inst
                .instance_type
                .as_ref()
                .and_then(|t| t.name.as_deref())
                .unwrap_or("N/A");
            let region = inst
                .region
                .as_ref()
                .and_then(|r| r.name.as_deref())
                .unwrap_or("N/A");
            let ssh_keys = inst
                .ssh_key_names
                .as_ref()
                .map(|k| k.join(", "))
                .unwrap_or_else(|| "N/A".to_string());

            output.push_str(&format!(
                "• ID: {}\n  Name: {} | Type: {} | Region: {}\n  Status: {} | IP: {}\n  SSH Keys: {}\n\n",
                id, name, inst_type, region, status, ip, ssh_keys
            ));
        }
        output
    }
}

// Tool parameter types
#[derive(Debug, Deserialize, JsonSchema)]
struct StartInstanceParams {
    /// GPU instance type (e.g., gpu_1x_h100, gpu_1x_a100)
    gpu: String,
    /// SSH key name to use for the instance
    ssh_key: String,
    /// Optional name for the instance
    name: Option<String>,
    /// Optional region to launch in (auto-selects if not specified)
    region: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct StopInstanceParams {
    /// Instance ID to terminate
    instance_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CheckAvailabilityParams {
    /// GPU instance type to check (e.g., gpu_1x_h100)
    gpu: String,
}

#[tool_router]
impl LambdaService {
    #[tool(
        description = "List all available GPU instance types with pricing, specs, and current availability"
    )]
    async fn list_gpu_types(&self) -> Result<CallToolResult, McpError> {
        let types = self
            .client
            .list_instance_types()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            Self::format_instance_types(&types),
        )]))
    }

    #[tool(description = "Launch a new GPU instance. Returns instance ID and connection details.")]
    async fn start_instance(
        &self,
        Parameters(params): Parameters<StartInstanceParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .client
            .launch_instance(
                &params.gpu,
                &params.ssh_key,
                params.name.as_deref(),
                params.region.as_deref(),
            )
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Instance launched successfully!\n\nInstance ID: {}\nRegion: {}\n\nNote: Instance may take a few minutes to become active. Use 'list_running_instances' to check status.",
            result.instance_id, result.region
        ))]))
    }

    #[tool(description = "Terminate a running GPU instance")]
    async fn stop_instance(
        &self,
        Parameters(params): Parameters<StopInstanceParams>,
    ) -> Result<CallToolResult, McpError> {
        self.client
            .terminate_instance(&params.instance_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Instance {} terminated successfully.",
            params.instance_id
        ))]))
    }

    #[tool(
        description = "List all currently running GPU instances with their status and connection details"
    )]
    async fn list_running_instances(&self) -> Result<CallToolResult, McpError> {
        let instances = self
            .client
            .list_running_instances()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            Self::format_instances(&instances),
        )]))
    }

    #[tool(
        description = "Check if a specific GPU type is currently available. Returns list of regions with availability."
    )]
    async fn check_availability(
        &self,
        Parameters(params): Parameters<CheckAvailabilityParams>,
    ) -> Result<CallToolResult, McpError> {
        let regions = self
            .client
            .check_availability(&params.gpu)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let message = if regions.is_empty() {
            format!(
                "GPU type '{}' is currently NOT available in any region.",
                params.gpu
            )
        } else {
            format!(
                "GPU type '{}' is available in: {}",
                params.gpu,
                regions.join(", ")
            )
        };

        Ok(CallToolResult::success(vec![Content::text(message)]))
    }
}

impl ServerHandler for LambdaService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Lambda Labs GPU instance management. Use these tools to list available GPU types, \
                 launch instances, check running instances, and terminate instances. \
                 Requires LAMBDA_API_KEY environment variable to be set."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the service
    let service = match LambdaService::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize Lambda service: {}", e);
            eprintln!("Make sure LAMBDA_API_KEY environment variable is set.");
            std::process::exit(1);
        }
    };

    // Run the MCP server over stdio
    let server = service.serve(rmcp::transport::io::stdio()).await?;

    // Wait for the server to finish
    server.waiting().await?;

    Ok(())
}
