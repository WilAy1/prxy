use std::{io::{Error, Result, copy}, net::{TcpListener, TcpStream}, thread};

const LISTEN_PORT: &str = "2345";
const TARGET_PORT: &str = "5173";

pub fn forwarder() -> std::io::Result<()> {
    let listen_addr = format!("127.0.0.1:{}", LISTEN_PORT);
    let server_addr = format!("127.0.0.1:{}", TARGET_PORT);

    // receive from the client and communicate with actual server
    let client_x = TcpListener::bind(listen_addr.as_str())?;
    println!("Listening on {}", listen_addr);

    for stream in client_x.incoming() {
        match stream {
            Ok(stream) => {
                let server_addr = server_addr.clone();

                thread::spawn(move || {
                    let _ = handle_connection(stream, &server_addr);
                });
            },
            Err(_) => eprintln!("Failed woefully"),
        }        
    }
    Ok(())
}


fn handle_connection(mut client: TcpStream, server_addr: &str) -> Result<()> {
    let mut server = TcpStream::connect(server_addr)?;

    println!("Connected to upstream {}", server_addr);

    let mut client_to_server = client.try_clone()?;
    let mut server_to_client = server.try_clone()?;

    let client_to_server_thread = thread::spawn(move || {
        copy(&mut client, &mut server)
    });

    let server_to_client_thread = thread::spawn(move || {
        copy(&mut server_to_client, &mut client_to_server)
    });

    client_to_server_thread
        .join()
        .map_err(|_| Error::other("client -> server thread panicked"))??;

    server_to_client_thread
        .join()
        .map_err(|_| Error::other("server -> client thread panicked"))??;

    Ok(())
}