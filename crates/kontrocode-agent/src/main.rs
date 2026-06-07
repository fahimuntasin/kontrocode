use kontrocode_agent::acp;
use kontrocode_agent::{Agent, AgentConfig};
use kontrocode_memory::{FileMemoryStore, MemoryStore};
use kontrocode_research::{NullSource, ResearchRunner, ResearchRunnerConfig};
use kontrocode_router::{MockProvider, ProviderRegistry, Router};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match mode {
        "acp" => run_acp().await,
        _ => {
            eprintln!("Usage: kontrocode-agent [acp]");
            anyhow::bail!("unknown mode: {mode}")
        }
    }
}

async fn run_acp() -> anyhow::Result<()> {
    let memory: Arc<dyn MemoryStore> = Arc::new(FileMemoryStore::default_location());
    let mut registry = ProviderRegistry::new();
    registry.register(MockProvider::new());
    let router = Router::with_default_config(registry);
    let runner = ResearchRunner::new(
        vec![Arc::new(NullSource)],
        ResearchRunnerConfig::default(),
    );
    let config = AgentConfig::new(std::env::current_dir().unwrap_or_default());
    let tools = kontrocode_agent::tools::default_tools();
    let agent = Agent::new(config, router, runner, memory, tools);

    tracing::info!("KontroCode ACP agent starting on stdin/stdout");
    acp::run_acp_agent(agent).await?;

    Ok(())
}
