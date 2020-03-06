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

fn accept_client(listener: TcpListener, writers: Arc<Mutex<Vec<TcpStream>>>,
                 tx: Option<Sender<i32>>, block: bool) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new client {:?}", stream);

                stream.set_nonblocking(!block)
                    .expect("set_nonblocking fails");

                let mut ws = writers.lock().unwrap();
                ws.push(stream);

                if ws.len() == 1 {
                    if let Some(sender) = &tx {
                        sender.send(0).unwrap();
                    }
                }
            }
            Err(e) => {
                eprintln!("connexion fails: {}", e);
            }
        }
    }
}

fn multiplex_stdin(listener: TcpListener, writers: Arc<Mutex<Vec<TcpStream>>>,
                   block: bool)
{
    let writers_cloned = writers.clone();

    thread::spawn(move || accept_client(listener, writers_cloned, None, block));

    while transfer_data(&mut std::io::stdin(), &writers) {}
}

fn multiplex_command(listener: TcpListener, writers: Arc<Mutex<Vec<TcpStream>>>,
                     block: bool, mut command: Command)
{
    let (tx, rx) = mpsc::channel();

    let writers_cloned = writers.clone();

    thread::spawn(move || accept_client(listener, writers_cloned, Some(tx), block));

    loop {
        rx.recv().unwrap();

        let child = command.spawn().expect("Failed to spawn");

        let mut stdout = child.stdout.expect("Unable to get output");

        if !transfer_data(&mut stdout, &writers) {
            return;
        }
    }
}

pub fn run(addr: &str, block: bool, cmd: Vec<&str>)
{
    let listener = TcpListener::bind(&addr).expect("unable to bind");

    let writers = Arc::new(Mutex::new(vec![]));

    if cmd.len() == 0 {
        return multiplex_stdin(listener, writers, block);
    } else {
        let mut command = Command::new(&cmd[0]);

        command.args(&cmd[1..]).stdout(Stdio::piped());

        return multiplex_command(listener, writers, block, command);
    }
}
