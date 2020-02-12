use std::io;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::net::TcpListener;

fn main() {
    let mut buffer = [0; 4096];
    let writers = Arc::new(Mutex::new(vec![]));

    {
        let writers = writers.clone();
        thread::spawn(move || {
            let listener = TcpListener::bind("127.0.0.1:1234").expect("unable to bind");

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

        });
    }

    loop {
        let n = io::stdin().read(&mut buffer).unwrap();

        //println!("You typed: {:?}", &buffer[..n]);

        {
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
        }
    }
}
