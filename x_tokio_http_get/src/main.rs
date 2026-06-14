use tokio::join;

#[tokio::main(worker_threads = 2)]
pub async fn main() {
    concurrents().await;
}

/// Retrieves three URLs concurrently.
async fn concurrents() {
    let a = http_get_await("https://www.rust-lang.org/");
    let b = http_get_await("https://www.google.com/");
    let c = http_get_await("https://www.github.com/");
    join!(a, b, c);
}

pub async fn http_get_await(url: &str) {
    println!("Retrieving URL: {}", url);
    let response = reqwest::get(url).await.unwrap();
    println!("Completed: ({}): {}", response.status(), url);
}
