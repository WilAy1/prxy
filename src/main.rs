mod server;
mod client;
mod forwarder;

use forwarder::forwarder::forwarder;

// use std::thread;

// use server::handle_b_server;
// use client::handle_b_client;

#[tokio::main]
async fn main() {
    let _ = forwarder().await;
}
