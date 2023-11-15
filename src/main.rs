use socket::connect;

mod print;
mod socket;
mod tools;

#[tokio::main]
async fn main() {
    let caller = tools::Functions::new("./".into()).unwrap();
    connect(caller).await.unwrap()
}
