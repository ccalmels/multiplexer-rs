use std::env;
use std::thread;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

fn transfer_data(writers: Arc<Mutex<Vec<TcpStream>>>) {
    let mut buffer = [0; 4096];
    let child = Command::new("yes")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spwan");

    let mut stdout = child.stdout.expect("Unable to get output");

    loop {
        let n = stdout.read(&mut buffer).unwrap();

        if n > 0 {
            let mut ws = writers.lock().unwrap();

            ws.retain(|mut writer| {
                let res = writer.write(&buffer[..n]);
                match res {
                    Ok(_) => { true }
                    Err(e) => {
                        println!("unable to send data: {}", e);
                        false
                    }
                }
            });
        } else {
            return;
        }
    }
}

fn accept_client(listener: TcpListener, writers: Arc<Mutex<Vec<TcpStream>>>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new client {:?}", stream);

                let mut ws = writers.lock().unwrap();
                ws.push(stream);
            }
            Err(e) => {
                println!("connexion fails: {}", e);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut addr = "127.0.0.1:1234";

    if args.len() > 1 {
        addr = &args[1];
    }

    let listener = TcpListener::bind(addr).expect("unable to bind");

    let writers = Arc::new(Mutex::new(vec![]));
    {
        let writers = writers.clone();

        thread::spawn(move || transfer_data(writers));
    }

    accept_client(listener, writers);
}
