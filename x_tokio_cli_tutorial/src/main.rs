mod lessons;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tokio-toor", about = "A guided tour of Tokio")]
struct Cli {
    #[command(subcommand)]
    lesson: Lesson,
}

#[derive(Subcommand)]
enum Lesson {
    /// Spawn concurrent tasks and await their results
    Spawn {
        #[arg(default_value_t = 5)]
        tasks: u64,
    },
    /// sleep, internal and timeout
    Timers,
    // /// Race futures conccurently with `select!`
    // Select,
    // /// Run futures concurretly with join!
    // Join,
    // /// Many-produce, single-consumer channel
    // Mpsc {
    //     #[arg(default_value_t = 3)]
    //     producers: u64,
    // },
    // /// Single Request -> response
    // Oneshot,
    // /// Fan-out to every subscriber
    // Broadcast,
    // /// Propagate the latst state to watchers
    // Watch,
    // /// Share a counter across tasks with Arc<Mutex>
    // SharedState {
    //     #[arg(default_value_t = 8)]
    //     tasks: u64,
    // },
    // // Offload blocking work with spawn_blocking
    // Blocking,
    // /// Graceful cancellation with CancellationToken
    // Cancel,
    // /// Async streams
    // Streams,
    // /// TCP echo server
    // Echo {
    //     #[arg(default_value = "127.0.0.1:8080")]
    //     addr: String,
    // },
    // /// Graceful shutdown on Ctrl-C
    // Signal,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().lesson {
        Lesson::Spawn { tasks } => lessons::spawn::run(tasks).await,
        Lesson::Timers => lessons::timers::run().await,
        // Lesson::Select => lessons::select::run().await,
        // Lesson::Join => lessons::join::run().await,
        // Lesson::Mpsc { producers } => lessons::mpsc::run(producers).await,
        // Lesson::Oneshot => lessons::oneshot::run().await,
        // Lesson::Broadcast => lessons::broadcast::run().await,
        // Lesson::Watch => lessons::watch::run().await,
        // Lesson::SharedState { tasks } => lessons::shared_state::run(tasks).await,
        // Lesson::Blocking => lessons::blocking::run().await,
        // Lesson::Cancel => lessons::cancel::run().await,
        // Lesson::Streams => lessons::streams::run().await,
        // Lesson::Echo { addr } => lessons::echo::run(addr).await,
        // Lesson::Signal => lessons::signal::run().await,
    }
}
