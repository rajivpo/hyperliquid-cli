use clap::Parser;
use std::convert::TryInto;
use colored::*;
use hyperliquid_rust_sdk::{BaseUrl, InfoClient, Subscription, Message};
use tokio::sync::mpsc::unbounded_channel;
use chrono::{NaiveDateTime, TimeZone, Utc};


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

        while let Some(Message::Trades(trades_message)) = receiver.recv().await {
            // Assuming trades_message.data is the correct way to access trades
            for trade in trades_message.data { // Corrected iteration
                let seconds = (trade.time / 1000).try_into().expect("Timestamp is out of range for i64");
                let nanoseconds = ((trade.time % 1000) * 1_000_000).try_into().expect("Nanoseconds conversion error");
    
                let naive_datetime = NaiveDateTime::from_timestamp_opt(seconds, nanoseconds).unwrap();
                let utc_datetime = Utc.from_utc_datetime(&naive_datetime);
                let human_readable_time = utc_datetime.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    
                let trade_side = if trade.side == "B" { "Buy".green() } else { "Sell".red() };
                let usd_amount = trade.px.parse::<f64>().unwrap() * trade.sz.parse::<f64>().unwrap();
                let order_type =  if trade.hash == "0x0000000000000000000000000000000000000000000000000000000000000000" { "TWAP".yellow() } else { "".white() };
    
                println!(
                    "{:<20} {:<8} {:>10} {:>10} {:>15.2} USD {:<3}",
                    human_readable_time,
                    trade_side,
                    trade.px,
                    trade.sz,
                    usd_amount,
                    order_type,
                );
            }
        }
}
