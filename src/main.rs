use clap::Parser;
use multiplexer_rs::run;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// listening socket address
    #[arg(
        short,
        long,
        default_value = "localhost:1234",
        id = "ADDRESS:PORT",
    )]
    listen: String,

    /// make blocking writes
    #[arg(short, long)]
    block: bool,
    /// make parallel writing to client
    #[arg(short, long)]
    parallel: bool,
    /// command to execute
    #[arg(trailing_var_arg = true)]
    cmd: Vec<String>,
}

fn main() {
    let args = Args::parse();

    run(&args.listen, args.block, args.parallel, args.cmd);
}
