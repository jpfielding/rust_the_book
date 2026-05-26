use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main] // only for async, std::thread::spawn
async fn main() {
    // no passing values into the closure
    let closed = std::thread::spawn(|| {
        println!("closed worker");
        return 42;
    });
    let res = closed.join().unwrap();
    println!("answer to closed {}", res);

    // move  values into the closure
    let moved = 42;
    let mover = std::thread::spawn(move || {
        println!("moved worker");
        return moved;
    });
    let res = mover.join().unwrap();
    println!("answer to moved {}", res);
    // move closures and shared vars
    let v1 = Arc::new(10); // Rc is single threaded
    let v2 = v1.clone();
    std::thread::spawn(move ||{
        println!("v2: {}", v2);
    });
    println!("v1: {}", v1);

    // sync trait
    let v1 = Arc::new(std::sync::Mutex::new(10));
    let v2 = v1.clone();
    std::thread::spawn(move ||{
        *v2.lock().unwrap() = 22;
    });
    // parallelism / concurrency
    async fn hello() -> &'static str {
        "hello from async"
    }
    println!("{}", hello().await); // starts/blocking
    // blocking pools (cpu heavy backpressure)
    let blocking = tokio::task::spawn_blocking(|| {
        return "hello from blocking";
    }).await.unwrap();
    println!("{}", blocking);

    // share memory by communicating 
    let (tx, mut rx) = mpsc::channel(32);
    let tx1 = tx.clone();
    tokio::spawn(async move { // spawn task 1
        tx1.send("hello from tx1").await.unwrap();
    });
    let tx2 = tx.clone();
    tokio::spawn(async move { // spawn task 2
        tx2.send("hello from tx2").await.unwrap();
    });
    drop(tx); // close the original; channel ends once all clones drop
    while let Some(message) = rx.recv().await {
        println!("received: {}", message);
    }
    // the end
    println!("done");
}
