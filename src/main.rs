use socket::connect;
use clap::Parser;
use functions::Functions;

mod print;
mod socket;
mod functions;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    query: String,

    #[clap(short, long, default_value = "./")]
    directory: String,
}


#[tokio::main]
async fn main() {
    let args = Args::parse();
    let functions = Functions::new(args.directory.into()).unwrap();
    connect(functions, args.query).await.unwrap()
}
