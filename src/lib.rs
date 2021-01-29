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
    is_blocking: bool,
}

impl Clients {
    pub fn send_to_one(mut stream: &std::net::TcpStream, buffer: &[u8]) -> bool {
        match stream.write(buffer) {
            Ok(_) => { true }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                eprintln!("{:?} would block", stream);
                true
            }
            Err(e) => {
                eprintln!("{:?} unable to send data: {}", stream, e);
                false
            }
        }
    }

    pub fn send_to_all(&self, buffer: &[u8]) -> bool {
        let send_closure = |w| Clients::send_to_one(w, buffer);
        let mut ws = self.writers.lock().unwrap();
        let status: Vec<bool> = ws.par_iter().map(send_closure).collect();
        let mut i = 0;

        ws.retain(|_| (status[i], i += 1).0);

        !ws.is_empty()
    }

    pub fn add_client(&self, stream: std::net::TcpStream) -> bool {
        stream.set_nonblocking(!self.is_blocking).expect("set_nonblocking fails");

        let mut ws = self.writers.lock().unwrap();

        ws.push(stream);
        ws.len() == 1
    }
}

fn transfer_data(mut input: impl Read, clients: &Clients) -> bool {
    let mut buffer = [0; 4096];

    loop {
        match input.read(&mut buffer) {
            Ok(0) => {
                return false;
            }
            Ok(n) => {
                if !clients.send_to_all(&buffer[..n]) {
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

fn accept_client(stream: TcpStream, clients: &Clients,
                 tx: &Option<Sender<i32>>) {
    eprintln!("new client {:?}", stream);

    if clients.add_client(stream) {
        if let Some(sender) = &tx {
            sender.send(0).unwrap();
        }
    }
}

fn listen_client(listener: TcpListener, clients: Clients,
                 tx: Option<Sender<i32>>) -> std::io::Result<()> {
    for stream in listener.incoming() {
        accept_client(stream?, &clients, &tx);
    }
    unreachable!();
}

fn multiplex_stdin(listener: TcpListener, clients: Clients) {
    let clients_cloned = clients.clone();

    thread::spawn(move || listen_client(listener, clients_cloned, None));

    while transfer_data(std::io::stdin(), &clients) {}
}

fn multiplex_command(listener: TcpListener, clients: Clients,
                     mut command: Command) {
    let (tx, rx) = mpsc::channel();

    let clients_cloned = clients.clone();

    thread::spawn(move || listen_client(listener, clients_cloned, Some(tx)));

    loop {
        rx.recv().unwrap();

        let mut child = command.spawn().expect("Failed to spawn");

        eprintln!("command {:?} spawned", command);

        let stdout = child.stdout.take().expect("Unable to get output");

        if !transfer_data(stdout, &clients) {
            return;
        }

        child.wait().expect("unable to wait the process");

        eprintln!("command {:?} end", command);
    }
}

pub fn run(addr: &str, block: bool, cmd: Option<Vec<&str>>) {
    let listener = TcpListener::bind(addr).expect("unable to bind");

    let clients = Clients {
        writers: Arc::new(Mutex::new(vec![])),
        is_blocking: block,
    };

    match cmd {
        None => { multiplex_stdin(listener, clients); },
        Some(cmd) => {
            let mut command = Command::new(cmd[0]);

            command.args(&cmd[1..]).stdout(Stdio::piped());

            multiplex_command(listener, clients, command);
        },
    }
}
