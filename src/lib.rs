use std::fmt::Debug;
use std::thread;
use std::io::{Read, Write, ErrorKind};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

extern crate rayon;
use rayon::prelude::*;

#[derive(Clone)]
struct Clients {
    writers: Arc<Mutex<Vec<TcpStream>>>,
    is_parallel: bool,
    is_blocking: bool,
}

impl Clients {
    pub fn send_to_one(writer: &mut (impl Write + Debug), buffer: &[u8]) -> bool {
        match writer.write(buffer) {
            Ok(_) => { true }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                eprintln!("{:?} would block", writer);
                true
            }
            Err(e) => {
                eprintln!("{:?} unable to send data: {}",
                          writer, e);
                false
            }
        }
    }

    pub fn send_to_all(&mut self, buffer: &[u8]) -> bool {
        let mut ws = self.writers.lock().unwrap();
        let status: Vec<bool>;

        if self.is_parallel {
            status = ws.par_iter().map(
                |mut w| Clients::send_to_one(&mut w, buffer)).collect();
        } else {
            status = ws.iter().map(
                |mut w| Clients::send_to_one(&mut w, buffer)).collect();
        }
        let mut i = 0;

        ws.retain(|_| (status[i], i += 1).0);

        ws.is_empty()
    }

    pub fn add_client(&mut self, stream: std::net::TcpStream) -> bool {
        stream.set_nonblocking(!self.is_blocking).expect("set_nonblocking fails");

        let mut ws = self.writers.lock().unwrap();

        ws.push(stream);
        ws.len() == 1
    }
}

fn transfer_data(mut input: impl Read,
                 writers: &Arc<Mutex<Vec<TcpStream>>>) -> bool {
    let mut buffer = [0; 4096];

    loop {
        match input.read(&mut buffer) {
            Ok(0) => {
                return false;
            }
            Ok(n) => {
                let mut ws = writers.lock().unwrap();

                let status: Vec<bool> = ws.par_iter().map(|mut writer| {
                    let res = writer.write(&buffer[..n]);
                    match res {
                        Ok(_) => { true }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            eprintln!("{:?} would block", writer);
                            true
                        }
                        Err(e) => {
                            eprintln!("{:?} unable to send data: {}",
                                      writer, e);
                            false
                        }
                    }
                }).collect();

                for i in (0..status.len()).rev() {
                    if !status[i] {
                        ws.remove(i);
                    }
                }

                if ws.is_empty() {
                    return true;
                }
            }
            Err(e) => {
                eprintln!("read fails: {}", e);
                return false;
            }
        }
    }
}

fn accept_client(stream: TcpStream, writers: &Arc<Mutex<Vec<TcpStream>>>,
                 tx: &Option<Sender<i32>>, block: bool) {
    eprintln!("new client {:?}", stream);

    stream.set_nonblocking(!block).expect("set_nonblocking fails");

    let mut ws = writers.lock().unwrap();

    if ws.is_empty() {
        if let Some(sender) = &tx {
            sender.send(0).unwrap();
        }
    }

    ws.push(stream);
}

fn listen_client(listener: TcpListener, writers: Arc<Mutex<Vec<TcpStream>>>,
                 tx: Option<Sender<i32>>, block: bool) -> std::io::Result<()> {
    for stream in listener.incoming() {
        accept_client(stream?, &writers, &tx, block);
    }
    unreachable!();
}

fn multiplex_stdin(listener: TcpListener, writers: Arc<Mutex<Vec<TcpStream>>>,
                   block: bool) {
    let writers_cloned = writers.clone();

    thread::spawn(move || listen_client(listener, writers_cloned, None, block));

    while transfer_data(std::io::stdin(), &writers) {}
}

fn multiplex_command(listener: TcpListener, writers: Arc<Mutex<Vec<TcpStream>>>,
                     block: bool, mut command: Command) {
    let (tx, rx) = mpsc::channel();

    let writers_cloned = writers.clone();

    thread::spawn(move || listen_client(listener, writers_cloned, Some(tx), block));

    loop {
        rx.recv().unwrap();

        let mut child = command.spawn().expect("Failed to spawn");

        eprintln!("command {:?} spawned", command);

        let stdout = child.stdout.take().expect("Unable to get output");

        if !transfer_data(stdout, &writers) {
            return;
        }

        child.wait().expect("unable to wait the process");

        eprintln!("command {:?} end", command);
    }
}

pub fn run(addr: &str, block: bool, in_parallel: bool, cmd: Option<Vec<&str>>) {
    let listener = TcpListener::bind(addr).expect("unable to bind");

    let writers = Arc::new(Mutex::new(vec![]));

    if !in_parallel {
        rayon::ThreadPoolBuilder::new().num_threads(1).build_global().unwrap();
    }

    match cmd {
        None => { multiplex_stdin(listener, writers, block); },
        Some(cmd) => {
            let mut command = Command::new(cmd[0]);

            command.args(&cmd[1..]).stdout(Stdio::piped());

            multiplex_command(listener, writers, block, command);
        },
    }
}
