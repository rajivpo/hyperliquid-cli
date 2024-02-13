use std::io::stdout;
use clap::Parser;
// use chrono::Local;
// use colored::*;
// use crossterm::{execute, cursor::MoveTo, terminal::*};
use hyperliquid_rust_sdk::{BaseUrl, InfoClient, Subscription, Message};
use tokio::sync::mpsc::unbounded_channel;

#[derive(Parser, Debug)]
pub struct WatchTradesArgs {
    /// Coin to watch trades for
    #[arg(short, long)]
    pub coin: String,
}

pub async fn execute(args: WatchTradesArgs) {
    env_logger::init();

    let coin = args.coin;

    let mut info_client: InfoClient = InfoClient::new(None, Some(BaseUrl::Mainnet)).await.unwrap();
    
    let mut market_sz_decimals: usize = 0;
    // let meta_response = info_client.meta().await.unwrap();
    // for market in meta_response.universe {
    //     if market.name == coin {
    //         market_sz_decimals = market.sz_decimals as usize;
    //         break;
    //     }
    // }

    let (sender, mut receiver) = unbounded_channel();
    info_client
        .subscribe(
            Subscription::Trades {
                coin: coin,
            },
            sender,
        )
        .await
        .unwrap();

    while let Some(Message::Trades(trade)) = receiver.recv().await {
        println!("Received trade data: {trade:?}");
    }
}