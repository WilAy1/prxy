use std::{io::{Read, Write}, net::TcpStream};

pub fn handle_b_client() -> std::io::Result<()> {
    let mut strean = TcpStream::connect("0.0.0.0:2345")?;
    
    strean.write_all(b"Hello there!")?;

    // read response

    let mut buffer = [0; 512];
    let bytes_read = strean.read(&mut buffer)?;

    println!("Received back: {}", String::from_utf8_lossy(&buffer[..bytes_read]));

    Ok(())
}