use tokio::{io::{AsyncReadExt, AsyncWriteExt, Result, copy}, net::{TcpListener, TcpStream}};

use crate::parser::{self, State, extract_url_parts};

// test with curl -v -x http://localhost:2345 http://example.com

const LISTEN_PORT: &str = "2345";

pub async fn forwarder() -> std::io::Result<()> {
    let listen_addr = format!("127.0.0.1:{}", LISTEN_PORT);
    // let server_addr = format!("127.0.0.1:{}", TARGET_PORT);

    // receive from the client and communicate with actual server
    let client_x = TcpListener::bind(listen_addr.as_str()).await?;
    println!("Listening on {}", listen_addr);

    loop {
        let stream = client_x.accept().await;
        match stream {
            Ok(stream) => {
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream.0).await {
                        eprintln!("connection error: {e}");
                    }
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


async fn handle_connection(client: TcpStream) -> Result<()> {
    let (mut client_reader, mut client_writer) = client.into_split();

    // get server address from request 

    let mut buffer = [0u8; 4096];

    let mut parser = parser::Parser::new();
    let loop_res: String = loop {
        let n = client_reader
            .read(&mut buffer)
            .await?;
        
        if n == 0 {
            return Ok(());
        }
        
        parser.feed(&buffer[..n]).unwrap();
        if let State::Complete { method: _, ref path, headers: _, body: _  } = parser.state {
            break path.to_string();
        }
    };

    let path = loop_res;

    let proxy_buffer = parser.rewrite_request().unwrap();

    let (domain, port, _) = extract_url_parts(&path.as_str()).unwrap();

    let server_addr = format!("{}:{}", domain, port);
    let server = TcpStream::connect(&server_addr).await?;
    
    println!("Connected to upstream {:?}", server_addr);

    let (mut  server_reader, mut server_writer) = server.into_split();

    server_writer.write_all(&proxy_buffer).await?;



    let client_to_server_thread = tokio::spawn(async move{
        copy(&mut client_reader, &mut server_writer).await?;
        Result::<()>::Ok(())
    });

    let server_to_client_thread = tokio::spawn(async move {
        copy(&mut server_reader, &mut client_writer).await?;
        Result::<()>::Ok(())
    });

    let (a, b) = tokio::join!(
        client_to_server_thread,
        server_to_client_thread,
    );

    a??;
    b??;


    Ok(())
}