mod utils;

mod errors;
mod tests;

use crate::utils::Server;
use common::CONFIG;
use std::env;

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", CONFIG.get_log_level());
    env_logger::init();

    let mut server = Server::new(CONFIG.get_server_ip(), CONFIG.get_server_port());
    server.listen().await;
}


