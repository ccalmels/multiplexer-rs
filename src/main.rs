use std::env;
use std::thread;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

fn transfer_data(input: &mut impl Read,
                 writers: &Arc<Mutex<Vec<TcpStream>>>) {
    let mut buffer = [0; 4096];

    loop {
        let n = input.read(&mut buffer).unwrap();

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

            if ws.len() == 0 {
                return;
            }
        } else {
            return;
        }
    }
}

fn accept_client(tx: Sender<i32>, listener: TcpListener,
                 writers: Arc<Mutex<Vec<TcpStream>>>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new client {:?}", stream);

                let mut ws = writers.lock().unwrap();
                ws.push(stream);

                if ws.len() == 1 {
                    tx.send(0).unwrap();
                }
            }
            Err(e) => {
                println!("connexion fails: {}", e);
            }
        }
    }
}

fn get_input(args: &Vec<String>) -> Box<dyn Read> {
    if args.len() > 0 {
        let cmd = &args[0];
        let cmd_args = if args.len() > 1 {
            &args[1..]
        } else {
            &[]
        };

        let child = Command::new(cmd)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");

        Box::new(child.stdout.expect("Unable to get output"))
    } else {
        Box::new(std::io::stdin())
    }
}

fn main() {
    let mut args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("usage");
        return;
    }

    args.remove(0);
    let addr = args.remove(0);

    let listener = TcpListener::bind(addr).expect("unable to bind");

    let writers = Arc::new(Mutex::new(vec![]));

    let (tx, rx) = mpsc::channel();

    {
        let writers = writers.clone();

        thread::spawn(move || accept_client(tx, listener, writers));
    }

    loop {
        rx.recv().unwrap();

        transfer_data(&mut get_input(&args), &writers);
    }
}
