//! codepulse local daemon.

mod api;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use codepulse_controller::Controller;
use codepulse_indexer::Indexer;
use codepulse_ingest::Ingestor;
use codepulse_store::Store;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "codepulse", about = "codepulse local daemon")]
struct Args {
    /// Workspace root to index
    #[arg(long, env = "CODEPULSE_ROOT", default_value = ".")]
    root: PathBuf,

    /// SQLite database path
    #[arg(long, env = "CODEPULSE_DB", default_value = ".codepulse/codepulse.db")]
    db: PathBuf,

    /// Listen address
    #[arg(long, env = "CODEPULSE_LISTEN", default_value = "127.0.0.1:7420")]
    listen: SocketAddr,

    /// Skip indexing on startup
    #[arg(long, default_value_t = false)]
    no_index: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub ingest: Arc<Ingestor>,
    pub controller: Arc<Controller>,
    pub indexer: Arc<std::sync::Mutex<Indexer>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let args = Args::parse();
    let root = args.root.canonicalize().unwrap_or(args.root.clone());

    if let Some(parent) = args.db.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let store = Store::open(&args.db).context("open store")?;
    let ingest = Arc::new(Ingestor::new(store.clone()));
    let controller = Arc::new(Controller::new(store.clone()));
    let indexer = Indexer::new(store.clone(), root.clone());

    if !args.no_index {
        match indexer.index_root() {
            Ok(n) => tracing::info!(symbols = n, root = %root.display(), "indexed workspace"),
            Err(e) => tracing::warn!(error = %e, "initial index failed"),
        }
    }

    let state = AppState {
        store,
        ingest,
        controller,
        indexer: Arc::new(std::sync::Mutex::new(indexer)),
    };

    let app = api::router(state);
    tracing::info!(%args.listen, root = %root.display(), "codepulse daemon listening");
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
