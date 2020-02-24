use std::env;
use std::thread;
use std::io::{Read, Write, ErrorKind};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

fn transfer_data(input: &mut impl Read,
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
                            eprintln!("would block");
                            true
                        }
                        Err(e) => {
                            eprintln!("unable to send data: {}", e);
                            false
                        }
                    }
                });

                if ws.len() == 0 {
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

fn accept_client(tx: Sender<i32>, listener: TcpListener,
                 writers: Arc<Mutex<Vec<TcpStream>>>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new client {:?}", stream);

                stream.set_nonblocking(true)
                    .expect("set_nonblocking fails");

                let mut ws = writers.lock().unwrap();
                ws.push(stream);

                if ws.len() == 1 {
                    tx.send(0).unwrap();
                }
            }
            Err(e) => {
                eprintln!("connexion fails: {}", e);
            }
        }
    }
}

fn usage(basename: &str) {
    eprintln!("usage: {} <listen_address> <command> [args...]", basename);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        return usage(&args[0]);
    }

    let addr = &args[1];
    let cmd = &args[2];
    let cmd_args = if args.len() > 3 {
        &args[3..]
    } else {
        &[]
    };

    let listener = TcpListener::bind(addr).expect("unable to bind");

    let writers = Arc::new(Mutex::new(vec![]));

    let (tx, rx) = mpsc::channel();

    {
        let writers = writers.clone();

        thread::spawn(move || accept_client(tx, listener, writers));
    }

    loop {
        rx.recv().unwrap();

        let child = Command::new(cmd)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");

        let mut stdout = child.stdout.expect("Unable to get output");

        if !transfer_data(&mut stdout, &writers) {
            return;
        }
    }
}
