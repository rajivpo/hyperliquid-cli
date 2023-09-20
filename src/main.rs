mod commands;
mod utils;

use clap::Parser;
use commands::{Command, dispatch};

#[tokio::main]
async fn main() {
    let command = Command::parse();
    dispatch(command).await;
}