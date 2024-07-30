use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::process;

const K_MAX_MSG: usize = 4096;

fn msg(message: &str) {
    eprintln!("{}", message);
}

fn die(message: &str) {
    let err = std::io::Error::last_os_error();
    eprintln!("[{}] {}", err.raw_os_error().unwrap_or(0), message);
    process::abort()
}

fn read_full(stream: &mut TcpStream, buf: &mut [u8]) -> std::io::Result<()> {
    let mut bytes_read = 0;
    while bytes_read < buf.len() {
        match stream.read(&mut buf[bytes_read..]) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "EOF",
                ))
            }
            Ok(n) => bytes_read += n,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn write_all(stream: &mut TcpStream, buf: &[u8]) -> std::io::Result<()> {
    stream.write_all(buf)
}

fn query(stream: &mut TcpStream, text: &str) -> std::io::Result<()> {
    let len = text.len() as u32;
    if len > K_MAX_MSG as u32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Message too long",
        ));
    }

    let mut wbuf = vec![0u8; 4 + text.len()];
    wbuf[0..4].copy_from_slice(&len.to_le_bytes());
    wbuf[4..].copy_from_slice(text.as_bytes());
    write_all(stream, &wbuf)?;

    let mut rbuf = vec![0u8; 4 + K_MAX_MSG + 1];
    read_full(stream, &mut rbuf[0..4])?;

    let len = u32::from_le_bytes(rbuf[0..4].try_into().unwrap()) as usize;
    if len > K_MAX_MSG {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Response too long",
        ));
    }

    read_full(stream, &mut rbuf[4..4 + len])?;

    let response = String::from_utf8_lossy(&rbuf[4..4 + len]);
    println!("server says: {}", response);
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, 1234))?;

    // multiple requests
    query(&mut stream, "hello1")?;
    query(&mut stream, "hello2")?;
    query(&mut stream, "hello3")?;

    Ok(())
}
