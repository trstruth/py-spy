use std::io::{Write, Error};
use std::net::UdpSocket;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use anyhow;


pub enum WriteRequest {
    Content(Vec<u8>),
    Done,
}

pub struct StreamingWriter {
    pub out_file: Arc<Mutex<std::fs::File>>,
    queue_tx: Sender<WriteRequest>,
}

impl StreamingWriter {
    pub fn new(filename: &str, queue_tx: Sender<WriteRequest>) -> Result<Self, anyhow::Error> {
        Ok(StreamingWriter {
            out_file: Arc::new(Mutex::new(std::fs::File::create(&filename)?)),
            queue_tx,
        })
    }

    pub fn stop_rx(&self) {
        self.queue_tx.send(WriteRequest::Done).unwrap();
    }
}

impl Write for StreamingWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        match self.queue_tx.send(WriteRequest::Content(buf.to_vec())) {
            Ok(_) => Ok(buf.len()),
            Err(e) => Err(Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

pub fn start_rx(rx: Receiver<WriteRequest>, out_file: Arc<Mutex<std::fs::File>>) -> JoinHandle<()> {
    thread::spawn(move || {
        // let mut out_file = out_file.lock().unwrap();
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        loop {
            match rx.recv() {
                Ok(write_request) => {
                    match write_request {
                        WriteRequest::Content(msg) => {
                            // out_file.write(&msg).unwrap();
                            socket.send_to(&msg, "go_kusto_exporter:8888").unwrap();
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
