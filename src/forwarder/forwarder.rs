// use std::{io::{Error, Result, copy}, net::{TcpListener, TcpStream}, thread};
use tokio::{io::{AsyncReadExt, AsyncWriteExt, Result, copy}, net::{TcpListener, TcpStream}};

use crate::parser;

const LISTEN_PORT: &str = "2345";
const TARGET_PORT: &str = "8080";

pub async fn forwarder() -> std::io::Result<()> {
    let listen_addr = format!("127.0.0.1:{}", LISTEN_PORT);
    let server_addr = format!("127.0.0.1:{}", TARGET_PORT);

    // receive from the client and communicate with actual server
    let client_x = TcpListener::bind(listen_addr.as_str()).await?;
    println!("Listening on {}", listen_addr);

    loop {
        let stream = client_x.accept().await;
        match stream {
            Ok(stream) => {
                let server_addr = server_addr.clone();

                tokio::spawn(async move {
                    let _ = handle_connection(stream.0, &server_addr).await;
                });
            },
            Err(err) => {
                eprintln!("Failed woefully {}", err);
                break;
            }
        }
    }
    Ok(())
}


async fn handle_connection(client: TcpStream, server_addr: &str) -> Result<()> {
    let server = TcpStream::connect(server_addr).await?;

    println!("Connected to upstream {}", server_addr);

    let (mut client_reader, mut client_writer) = client.into_split();
    let (mut  server_reader, mut server_writer) = server.into_split();

    let client_to_server_thread = tokio::spawn(async move {
        // parse HTTP request
        let mut buffer = [0u8; 4096];

        let mut parser = parser::Parser::new();
        loop {
            let n = client_reader.read(&mut buffer).await.unwrap();
            
            if n == 0 {
                break;
            }
            
            parser.feed(&buffer[..n]).unwrap();

            server_writer.write_all(&mut buffer[..n]).await?;
        }
        Result::<()>::Ok(())
        // copy(&mut client_reader, &mut server_writer).await
    });

    let server_to_client_thread = tokio::spawn(async move {
        copy(&mut server_reader, &mut client_writer).await?;
        Result::<()>::Ok(())
    });

    tokio::select! {
        res = client_to_server_thread => res? ,
        res = server_to_client_thread => res? ,
    }?;

    Ok(())
}