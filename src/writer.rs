use std::env;
use std::io::{Write, Error};
use std::net::{UdpSocket, SocketAddr};
use std::sync::mpsc::{Sender, Receiver};
use std::thread::{self, JoinHandle};

use anyhow;

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
    pub fn new(write_destination: SocketAddr, queue_tx: Sender<WriteRequest>) -> anyhow::Result<Self, anyhow::Error> {
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
                        },
                        WriteRequest::Done => {
                            break;
                        }
                    }
                },
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
        Some(dest.parse().map_err(Into::into))
    } else if let Ok(dest) = env::var("PYSPY_STREAM_DEST") {
        // then look in the environment
        Some(dest.parse().map_err(Into::into))
    } else {
        None
    }
}
