/// Blocking TCP server

use std::io::{Read, Write};
use std::net::{TcpListener};
use std::thread;

pub fn handle_b_server() {
    let listener = TcpListener::bind("0.0.0.0:2345").unwrap();
    println!("Server listening on port 2345");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("New client connected");
                thread::spawn(move || {
                    let mut buffer = [0; 512]; // Buffer to hold incoming data
                    
                    match stream.read(&mut buffer) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 { return; }
                            // print received message
                            println!("Received: {}", String::from_utf8_lossy(&buffer[..bytes_read]));

                            // write back
                            if let Err(e) = stream.write_all(b"Message received") {
                                eprintln!("Error occured while sending {}", e);
                            }
                        },
                        Err(e ) => eprintln!("Failed to read from stream {}", e)
                    };
                });
            },
            Err(_) => {}
        }
    }

}