use clap::Parser;
use functions::Functions;
use socket::connect;

mod functions;
mod print;
mod socket;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    query: String,

    #[clap(long, default_value = "./")]
    directory: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let functions = Functions::new(args.directory.into()).unwrap();
    connect(functions, args.query).await.unwrap()
}
