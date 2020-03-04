extern crate clap;

use clap::{Arg, App, AppSettings, SubCommand};
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

fn main() {
    let matches = App::new("IO multiplexer")
        .version("0.1")
        .author("Clément Calmels <clement.calmels@free.fr>")
        .setting(AppSettings::AllowExternalSubcommands)
//        .setting(AppSettings::SubcommandRequired)
        .arg(Arg::with_name("listen")
             .short("l")
             .long("listen")
             .takes_value(true)
             .value_name("ADDRESS:PORT")
             .help("help listen"))
        .arg(Arg::with_name("block")
             .short("b")
             .long("block")
             .help("help block"))
        .subcommand(SubCommand::with_name("-")
                    .about("read from stdin"))
        .get_matches();

    let addr = matches.value_of("listen").unwrap_or("localhost:1234");
    let block = matches.is_present("block");

    let (cmd, cmd_args) =
        match matches.subcommand() {
            (_, None) => {
                ("", Vec::new())
            },
            (cmd, Some(ext_m)) => {
                (cmd, match ext_m.values_of("") {
                    None => { Vec::new() }
                    Some(iterator) => { iterator.collect() }
                })
            }
        };

    let listener = TcpListener::bind(&addr).expect("unable to bind");

    let writers = Arc::new(Mutex::new(vec![]));

    let (tx, rx) = mpsc::channel();

    {
        let writers = writers.clone();

        thread::spawn(move || accept_client(tx, listener, writers));
    }

    loop {
        rx.recv().unwrap();

        let child = Command::new(&cmd)
            .args(&cmd_args)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");

        let mut stdout = child.stdout.expect("Unable to get output");

        if !transfer_data(&mut stdout, &writers) {
            return;
        }
    }
}
