use std::net::SocketAddr;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "gossipers", about = "Дорогие сплетники...")]
struct Opt {
    #[structopt(long = "period", default_value = "5.")]
    period: f32,
    #[structopt(long = "port", default_value = "8080")]
    port: u16,
    #[structopt(long = "connect")]
    connect: Option<SocketAddr>,
}
fn main() {
    env_logger::init();
    let opt = Opt::from_args();
}
