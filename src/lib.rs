use std::thread;
use std::io::{Read, Write, ErrorKind};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

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

                ws.retain(|mut writer| {
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
                });

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

pub fn run(addr: &str, block: bool, cmd: Option<Vec<&str>>) {
    let listener = TcpListener::bind(addr).expect("unable to bind");

    let writers = Arc::new(Mutex::new(vec![]));

    match cmd {
        None => { multiplex_stdin(listener, writers, block); },
        Some(cmd) => {
            let mut command = Command::new(cmd[0]);

            command.args(&cmd[1..]).stdout(Stdio::piped());

            multiplex_command(listener, writers, block, command);
        },
    }
}
