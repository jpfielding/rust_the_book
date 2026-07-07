use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::sleep;

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

#[derive(Clone)]
struct AppState {
    tx: mpsc::Sender<Message>,
    rx: Arc<Mutex<mpsc::Receiver<Message>>>,
}
#[derive(Serialize, Deserialize)]
struct Message {
    text: String,
}

async fn run(cli: Cli) -> Result<(), String> {
    let port: u16 = cli.port.parse().expect("invalid port number");

    let bind = format!("0.0.0.0:{}", port);

    let listener = TcpListener::bind(&bind)
        .await
        .map_err(|e| format!("bind {}: {}", bind, e))?;

    eprintln!("web-service listening on http://{}", bind);

    let (tx, rx) = mpsc::channel::<Message>(32);
    let state = AppState {
        tx,
        rx: Arc::new(Mutex::new(rx)),
    };

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| format!("serve: {}", e))
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

fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/ping", post(ping))
        .route("/pong", get(pong))
        .with_state(state)
}

pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub async fn readyz() -> StatusCode {
    StatusCode::OK
}

async fn ping(State(state): State<AppState>, Json(msg): Json<Message>) -> StatusCode {
    match state.tx.send(msg).await {
        Ok(_) => StatusCode::ACCEPTED,             // 202: queued
        Err(_) => StatusCode::SERVICE_UNAVAILABLE, // receiver gone
    }
}

async fn pong(State(state): State<AppState>) -> Result<Json<Message>, StatusCode> {
    let mut rx = state.rx.lock().await;
    tokio::select! {
        Some(msg) = rx.recv() => Ok(Json(msg)),
        _ = sleep(Duration::from_secs(30)) => Err(StatusCode::REQUEST_TIMEOUT), // 408: timeout
    }
}
