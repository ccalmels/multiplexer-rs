extern crate clap;

use clap::{Arg, App};
use multiplexer_rs::run;

fn main() {
    let matches = App::new("IO multiplexer")
        .version("0.1")
        .author("Clément Calmels <clement.calmels@free.fr>")
        .arg(Arg::with_name("listen")
             .short("l")
             .long("listen")
             .takes_value(true)
             .value_name("ADDRESS:PORT")
             .help("listening socket address"))
        .arg(Arg::with_name("block")
             .short("b")
             .long("block")
             .help("make blocking writes"))
        .arg(Arg::with_name("cmd")
             .multiple(true)
             .help("command to run"))
        .get_matches();

    let addr = matches.value_of("listen").unwrap_or("localhost:1234");
    let block = matches.is_present("block");
    let cmd: Option<Vec<&str>> = match matches.values_of("cmd") {
        Some(iterator) => { Some(iterator.collect()) },
        None => { None }
    };

    run(addr, block, cmd);
}
