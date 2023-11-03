use std::env;
use std::io::{Error, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use anyhow::Context;

use crate::config::Config;

pub enum WriteRequest {
    Content(Vec<u8>),
    Done,
}

pub struct StreamingWriter {
    pub socket_addr: SocketAddr,
    queue_tx: Sender<WriteRequest>,
}

impl StreamingWriter {
    pub fn new(
        write_destination: SocketAddr,
        queue_tx: Sender<WriteRequest>,
    ) -> anyhow::Result<Self, anyhow::Error> {
        Ok(StreamingWriter {
            socket_addr: write_destination,
            queue_tx,
        })
    }

    pub fn stop_rx(&self) {
        self.queue_tx.send(WriteRequest::Done).unwrap();
    }
}

impl Write for StreamingWriter {
    fn write(&mut self, buf: &[u8]) -> anyhow::Result<usize, Error> {
        match self.queue_tx.send(WriteRequest::Content(buf.to_vec())) {
            Ok(_) => Ok(buf.len()),
            Err(e) => Err(Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    fn flush(&mut self) -> anyhow::Result<(), Error> {
        Ok(())
    }
}

pub fn start_rx(rx: Receiver<WriteRequest>, write_destination: SocketAddr) -> JoinHandle<()> {
    thread::spawn(move || {
        println!("streaming samples to {}", write_destination);
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        socket.set_nonblocking(true).unwrap();
        loop {
            match rx.recv() {
                Ok(write_request) => {
                    match write_request {
                        WriteRequest::Content(msg) => {
                            // out_file.write(&msg).unwrap();
                            socket.send_to(&msg, write_destination).unwrap();
                        }
                        WriteRequest::Done => {
                            break;
                        }
                    }
                }
                Err(e) => {
                    println!("Got error: {}", e);
                    break;
                }
            }
        }
    })
}

pub fn find_dest(config: &Config) -> Option<anyhow::Result<SocketAddr>> {
    if let Some(dest) = &config.stream_destination {
        // first look in the supplied args
        Some(parse_to_socket_addr(dest))
    } else if let Ok(dest) = env::var("PYSPY_STREAM_DEST") {
        // then look in the environment
        Some(parse_to_socket_addr(&dest))
    } else {
        None
    }
}

fn parse_to_socket_addr(addr_str: &str) -> anyhow::Result<SocketAddr> {
    let mut addrs_iter = addr_str
        .to_socket_addrs()
        .with_context(|| format!("Could not resolve the address: {}", addr_str))?;

    addrs_iter
        .next()
        .with_context(|| format!("No addresses found for {}", addr_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_hostname_and_port() {
        let addr_str = "example.com:80";
        assert!(parse_to_socket_addr(addr_str).is_ok());
    }

    #[test]
    fn test_invalid_hostname() {
        let addr_str = "invalid_hostname:80";
        assert!(parse_to_socket_addr(addr_str).is_err());
    }

    #[test]
    fn test_invalid_port() {
        let addr_str = "example.com:99999";
        assert!(parse_to_socket_addr(addr_str).is_err());
    }

    #[test]
    fn test_missing_port() {
        let addr_str = "example.com";
        assert!(parse_to_socket_addr(addr_str).is_err());
    }

    #[test]
    fn test_empty_string() {
        let addr_str = "";
        assert!(parse_to_socket_addr(addr_str).is_err());
    }

    #[test]
    fn test_valid_ip_and_port() {
        let addr_str = "93.184.216.34:80"; // example.com IP
        assert!(parse_to_socket_addr(addr_str).is_ok());
    }

    #[test]
    fn test_ipv6_address_and_port() {
        let addr_str = "[2606:2800:220:1:248:1893:25c8:1946]:80"; // example.com IPv6
        assert!(parse_to_socket_addr(addr_str).is_ok());
    }
}
