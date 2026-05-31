use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
// use clap::Parser;
use tokio::net::TcpListener;

static DEFAULT_PORT: &str = "8080";

#[tokio::main]
async fn main() -> Result<(), String> {
    let port: u16 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_PORT.to_string())
        .parse()
        .expect("invalid port number");

    let bind = format!("0.0.0.0:{port}");

    let listener = TcpListener::bind(&bind)
        .await
        .map_err(|e| format!("bind {bind}: {e}"))?;
    eprintln!("agentstate-service listening on http://{bind}");

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
            println!("SIGHUP received, reloading config...");
        }
        _ = terminate => {
            println!("SIGINT received, exiting...");
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
