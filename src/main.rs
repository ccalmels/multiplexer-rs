use std::io;
use std::thread;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream};

fn transfer_data(writers: Arc<Mutex<Vec<TcpStream>>>) {
    let mut buffer = [0; 4096];

    loop {
        let n = io::stdin().read(&mut buffer).unwrap();

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

fn main() {
    let writers = Arc::new(Mutex::new(vec![]));

    let listener = TcpListener::bind("127.0.0.1:1234").expect("unable to bind");

    {
        let writers = writers.clone();

        thread::spawn(move || transfer_data(writers));
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new client {:?}", stream);

                let mut ws = writers.lock().unwrap();
                ws.push(stream);
            }
            Err(e) => {
                println!("connexion failed: {}", e);
            }
        }
    }
}
