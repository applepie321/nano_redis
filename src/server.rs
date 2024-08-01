use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::time::Duration;

const K_MAX_MSG: usize = 4096;
const SERVER: Token = Token(0);

// Print a message to stderr
fn msg(message: &str) {
    eprintln!("{}", message);
}

// Print an error message and abort the program
fn die(message: &str) {
    let err = std::io::Error::last_os_error();
    eprintln!("[{}] {}", err.raw_os_error().unwrap_or(0), message);
    std::process::abort();
}

#[derive(PartialEq)]
enum State {
    Req,
    Res,
    End,
}

struct Conn {
    stream: TcpStream,
    state: State,
    rbuf: Vec<u8>,
    wbuf: Vec<u8>,
    wbuf_sent: usize,
}

impl Conn {
    fn new(stream: TcpStream) -> Self {
        Conn {
            stream,
            state: State::Req,
            rbuf: Vec::with_capacity(4 + K_MAX_MSG),
            wbuf: Vec::with_capacity(4 + K_MAX_MSG),
            wbuf_sent: 0,
        }
    }
}

fn try_one_request(conn: &mut Conn) -> bool {
    if conn.rbuf.len() < 4 {
        return false;
    }
    let len = u32::from_le_bytes([conn.rbuf[0], conn.rbuf[1], conn.rbuf[2], conn.rbuf[3]]) as usize;
    if len > K_MAX_MSG {
        msg("too long");
        conn.state = State::End;
        return false;
    }
    if 4 + len > conn.rbuf.len() {
        return false;
    }

    println!(
        "client says: {}",
        String::from_utf8_lossy(&conn.rbuf[4..4 + len])
    );

    conn.wbuf.clear();
    conn.wbuf.extend_from_slice(&(len as u32).to_le_bytes());
    conn.wbuf.extend_from_slice(&conn.rbuf[4..4 + len]);

    conn.rbuf.drain(..4 + len);

    conn.state = State::Res;
    conn.wbuf_sent = 0;
    true
}

fn try_fill_buffer(conn: &mut Conn) -> bool {
    let mut buf = [0u8; K_MAX_MSG];
    loop {
        match conn.stream.read(&mut buf) {
            Ok(0) => {
                if conn.rbuf.is_empty() {
                    msg("EOF");
                } else {
                    msg("unexpected EOF");
                }
                conn.state = State::End;
                return false;
            }
            Ok(n) => {
                conn.rbuf.extend_from_slice(&buf[..n]);
                while try_one_request(conn) {}
                return conn.state == State::Req;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return false;
            }
            Err(_) => {
                msg("read() error");
                conn.state = State::End;
                return false;
            }
        }
    }
}

fn try_flush_buffer(conn: &mut Conn) -> bool {
    loop {
        let remain = conn.wbuf.len() - conn.wbuf_sent;
        match conn.stream.write(&conn.wbuf[conn.wbuf_sent..]) {
            Ok(0) => {
                conn.state = State::End;
                return false;
            }
            Ok(n) => {
                conn.wbuf_sent += n;
                if conn.wbuf_sent == conn.wbuf.len() {
                    conn.state = State::Req;
                    conn.wbuf_sent = 0;
                    conn.wbuf.clear();
                    return false;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return false;
            }
            Err(_) => {
                msg("write() error");
                conn.state = State::End;
                return false;
            }
        }
    }
}

fn connection_io(conn: &mut Conn) {
    match conn.state {
        State::Req => while try_fill_buffer(conn) {},
        State::Res => while try_flush_buffer(conn) {},
        State::End => {}
    }
}

pub fn main() -> std::io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let addr = "0.0.0.0:1234".parse::<SocketAddr>().unwrap();
    let mut server = TcpListener::bind(addr)?;

    poll.registry()
        .register(&mut server, SERVER, Interest::READABLE)?;

    let mut connections = HashMap::new();
    let mut unique_token = Token(SERVER.0 + 1);

    loop {
        poll.poll(&mut events, Some(Duration::from_millis(1000)))?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    match server.accept() {
                        Ok((mut stream, _)) => {
                            let token = Token(unique_token.0);
                            unique_token.0 += 1;

                            poll.registry().register(
                                &mut stream,
                                token,
                                Interest::READABLE | Interest::WRITABLE,
                            )?;

                            connections.insert(token, Conn::new(stream));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                },
                token => {
                    if let Some(conn) = connections.get_mut(&token) {
                        connection_io(conn);
                        if conn.state == State::End {
                            poll.registry().deregister(&mut conn.stream)?;
                            connections.remove(&token);
                        }
                    }
                }
            }
        }
    }
}
