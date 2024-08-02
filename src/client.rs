use std::env;
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
    process::abort();
}

fn read_full(stream: &mut TcpStream, buf: &mut [u8]) -> std::io::Result<()> {
    stream.read_exact(buf)
}

fn write_all(stream: &mut TcpStream, buf: &[u8]) -> std::io::Result<()> {
    stream.write_all(buf)
}

fn send_req(stream: &mut TcpStream, cmd: &[String]) -> std::io::Result<()> {
    let mut len = 4;
    for s in cmd {
        len += 4 + s.len();
    }
    if len > K_MAX_MSG {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Message too long",
        ));
    }

    let mut wbuf = Vec::with_capacity(4 + K_MAX_MSG);
    wbuf.extend_from_slice(&(len as u32).to_le_bytes());
    wbuf.extend_from_slice(&(cmd.len() as u32).to_le_bytes());

    for s in cmd {
        wbuf.extend_from_slice(&(s.len() as u32).to_le_bytes());
        wbuf.extend_from_slice(s.as_bytes());
    }

    write_all(stream, &wbuf)
}

fn read_res(stream: &mut TcpStream) -> std::io::Result<()> {
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

    if len < 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Bad response",
        ));
    }

    let rescode = u32::from_le_bytes(rbuf[4..8].try_into().unwrap());
    println!(
        "server says: [{}] {}",
        rescode,
        String::from_utf8_lossy(&rbuf[8..4 + len])
    );

    Ok(())
}

pub fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, 1234))?;

    let args: Vec<String> = env::args().skip(1).collect();

    send_req(&mut stream, &args)?;
    read_res(&mut stream)?;

    Ok(())
}
