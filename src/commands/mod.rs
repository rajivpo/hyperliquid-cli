mod watch_l2_book;

use clap::Parser;
use watch_l2_book::WatchBookArgs;

#[derive(Parser, Debug)]
pub enum Command {
    /// Watch L2Book for a specific coin
    WatchBook(WatchBookArgs),
    
}

pub async fn dispatch(command: Command) {
    match command {
        Command::WatchBook(args) => watch_l2_book::execute(args).await,
        // Dispatch other commands here
    }
}