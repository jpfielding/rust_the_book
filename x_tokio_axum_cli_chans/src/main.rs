use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use clap::Parser;
use tokio::net::TcpListener;

#[derive(Parser)]
#[command(name = "web-service", about = "Web Service Daemon")]
struct Cli {
    #[arg(short, long, default_value = "8080", env = "DEFAULT_PORT")]
    port: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), String> {
    let port: u16 = cli.port.parse().expect("invalid port number");

    let bind = format!("0.0.0.0:{port}");

    let listener = TcpListener::bind(&bind)
        .await
        .map_err(|e| format!("bind {bind}: {e}"))?;

    eprintln!("web-service listening on http://{bind}");

    axum::serve(listener, router())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| format!("serve: {e}"))
}

/// Resolve when a termination signal arrives, then drain source tasks.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut term) = signal(SignalKind::terminate()) {
            term.recv().await;
        }
    };

    tokio::select! {
        _ = ctrl_c => {
            println!("SIGINT received, relaying shutdown...");
        }
        _ = terminate => {
            println!("SIGTERM received, relaying shutdown...");
        }
    }
}

pub async fn healthz() -> &'static str {
    "ok"
}

/// Readiness in a push model is *can we accept events*, not *do we have data*.
/// The manager is constructed before serving, so once we answer at all we're ready.
pub async fn readyz() -> StatusCode {
    StatusCode::OK
}

/// Build the full router. Bodies reuse `Event`/`Insight`/`SourceStatus`/
/// `SourceSummary` directly; see [`dto`] for the thin response envelopes.
pub fn router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}
