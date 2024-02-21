mod watch_l2_book;
mod graph_l2_book;
mod watch_trades;

use clap::Parser;
use graph_l2_book::GraphBookArgs;
use watch_l2_book::WatchBookArgs;
use watch_trades::WatchTradesArgs;

#[derive(Parser, Debug)]
pub enum Command {
    /// Watch L2Book for a specific coin
    WatchBook(WatchBookArgs),
    /// Watch Trades for specific coin
    WatchTrades(WatchTradesArgs),
    /// Graph L2 Book for a specific coin
    GraphBook(GraphBookArgs),
}

pub async fn dispatch(command: Command) {
    match command {
        Command::WatchBook(args) => watch_l2_book::execute(args).await,
        Command::WatchTrades(args) => watch_trades::execute(args).await,
        Command::GraphBook(args) => graph_l2_book::execute(args).await,
    }
}