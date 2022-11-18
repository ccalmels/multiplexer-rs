use clap::Parser;
use multiplexer_rs::run;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "localhost:1234",
        id = "ADDRESS:PORT",
        help = "listening socket addresse"
    )]
    listen: String,

    #[arg(short, long, help = "make blocking writes")]
    block: bool,
    #[arg(short, long, help = "make parallel writing to client")]
    parallel: bool,
    #[arg(trailing_var_arg = true)]
    cmd: Vec<String>,
}

fn main() {
    let args = Args::parse();

    run(&args.listen, args.block, args.parallel, args.cmd);
}
