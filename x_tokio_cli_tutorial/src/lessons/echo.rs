// src/lessons/echo.rs
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn run(addr: String) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    println!(
        "echo server on {addr} (connect with: nc {})",
        addr.replace(":", " ")
    );

    loop {
        let (mut socket, peer) = listener.accept().await?;
        println!("accepted connection from {peer}");

        // one task per connection, the accept loop is never blocked by a client
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => {
                        println!("connection closed by {peer}");
                        break;
                    }
                    Ok(n) => {
                        if let Err(e) = socket.write_all(&buf[..n]).await {
                            eprintln!("failed to write to {peer}: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("failed to read from {peer}: {e}");
                        break;
                    }
                }
            }
        });
    }
}
