mod server;
mod client;
mod forwarder;
pub mod parser;

use forwarder::forwarder::forwarder;

// use std::thread;

// use server::handle_b_server;
// use client::handle_b_client;

#[tokio::main]
async fn main() {
    // parser::parse_request("".into());
    let _ = forwarder().await;
}
