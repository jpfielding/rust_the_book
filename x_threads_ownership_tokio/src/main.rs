use std::sync::Arc;
use tokio;

#[tokio::main]
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
    async fn hello() {
        println!("hello from async");
    }

    hello().await; // starts/blocking
    println!("done");
}
