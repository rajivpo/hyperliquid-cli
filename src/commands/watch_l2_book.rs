use std::io::stdout;
use clap::Parser;
use chrono::Local;
use colored::*;
use crossterm::{execute, cursor::MoveTo, terminal::*};
use hyperliquid_rust_sdk::{BaseUrl, BookLevel, InfoClient, Subscription, Message};
use tokio::sync::mpsc::unbounded_channel;
use crate::utils::format_decimal;

#[derive(Parser, Debug)]
pub struct WatchBookArgs {
    /// Coin to get l2book for
    #[arg(short, long)]
    pub coin: String,

    /// Levels of l2book to show
    #[arg(short, long, default_value_t = 10)]
    pub levels: u8,

    /// Show extra data about book
    #[clap(short, long, required = false, default_value = "false")]
    pub show_extra_data: bool
}

pub async fn execute(args: WatchBookArgs) {
    env_logger::init();

    let coin = args.coin;
    let levels = args.levels / 2;

    let mut info_client: InfoClient = InfoClient::new(None, Some(BaseUrl::Mainnet)).await.unwrap();
    
    let mut market_sz_decimals: usize = 0;
    let meta_response = info_client.meta().await.unwrap();
    for market in meta_response.universe {
        if market.name == coin {
            market_sz_decimals = market.sz_decimals as usize;
            break;
        }
    }

    let (sender, mut receiver) = unbounded_channel();
    info_client
        .subscribe(
            Subscription::L2Book {
                coin: coin,
            },
            sender,
        )
        .await
        .unwrap();

    while let Some(Message::L2Book(l2_book)) = receiver.recv().await {
        let bids = &l2_book.data.levels[0];
        let asks = &l2_book.data.levels[1];
    
        if let (Some(highest_bid), Some(lowest_ask)) = (bids.first(), asks.first()) {
            let highest_bid_px: f64 = format_decimal(&highest_bid.px, market_sz_decimals, Color::White, true).parse().unwrap();
            let lowest_ask_px: f64 = format_decimal(&lowest_ask.px, market_sz_decimals, Color::White, true).parse().unwrap();
            let mid_px = (lowest_ask_px + highest_bid_px) / 2.0;
            let formatted_mid_px = format_decimal(&mid_px.to_string(), market_sz_decimals, Color::White, true);

            let spread_px = lowest_ask_px - highest_bid_px;
            let spread_bps = (spread_px / mid_px) * 10000.0;

            let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");

            let mut bid_notional: f64 = 0.0;
            let mut ask_notional: f64 = 0.0;

            for bid in bids.iter() {
                let bid_px: f64 = format_decimal(&bid.px, market_sz_decimals, Color::White, true).parse().unwrap();
                if (bid_px >= mid_px * 0.98) && (bid_px <= mid_px * 1.02) {
                    let bid_sz: f64 = format_decimal(&bid.sz, market_sz_decimals, Color::White, false).parse().unwrap();
                    bid_notional += bid_sz * bid_px;
                }
            }

            for ask in asks.iter() {
                let ask_px: f64 = format_decimal(&ask.px, market_sz_decimals, Color::White, true).parse().unwrap();
                if (ask_px >= mid_px * 0.98) && (ask_px <= mid_px * 1.02) {
                    let ask_sz: f64 = format_decimal(&ask.sz, market_sz_decimals, Color::White, false).parse().unwrap();
                    ask_notional += ask_sz * ask_px;
                }
            }

            let _total_notional_in_range = bid_notional + ask_notional;
            let buy_slippage_10k = calculate_slippage(&bids, &asks, 10_000.0);
            let buy_slippage_100k = calculate_slippage(&bids, &asks, 100_000.0);
            let sell_slippage_10k = calculate_slippage(&bids, &asks, -10_000.0);
            let sell_slippage_100k = calculate_slippage(&bids, &asks, -100_000.0);

            execute!(stdout(), MoveTo(0, 0), Clear(ClearType::All)).unwrap();
            println!("{} Spread: {} px, Spread: {} bps", 
                now, 
                format_decimal(&spread_px.to_string(), market_sz_decimals, Color::White, true), 
                format_decimal(&spread_bps.to_string(), market_sz_decimals, Color::White, true),
            );
            if args.show_extra_data {
                println!("Buy slippage - $10k {} bps, $100k {} bps",
                    format_decimal(&buy_slippage_10k.to_string(), market_sz_decimals, Color::White, true),
                    format_decimal(&buy_slippage_100k.to_string(), market_sz_decimals, Color::White, true),
                );
                println!("Sell slippage - $10k {} bps, $100k {} bps",
                    format_decimal(&sell_slippage_10k.to_string(), market_sz_decimals, Color::White, true),
                    format_decimal(&sell_slippage_100k.to_string(), market_sz_decimals, Color::White, true),
                );
            }
            println!();
    
            for ask in asks.iter().take(levels as usize).rev() {
                let ask_sz = format_decimal(&ask.sz, market_sz_decimals, Color::Red, false);
                let ask_px = format_decimal(&ask.px, market_sz_decimals, Color::White, true);
                println!("{:<10} {:<10} {:<10}", "", ask_px, ask_sz);
            }
    
            println!("{:<10} {:<10} {:<10}", "", formatted_mid_px, "");
    
            for bid in bids.iter().take(levels as usize) {
                let bid_sz = format_decimal(&bid.sz, market_sz_decimals, Color::Green, false);
                let bid_px = format_decimal(&bid.px, market_sz_decimals, Color::White, true);
                println!("{:<10} {:<10} {:<10}", bid_sz, bid_px, "");
            }
        }
    }
}

fn calculate_slippage(
    bids: &Vec<BookLevel>, 
    asks: &Vec<BookLevel>, 
    amount: f64
) -> f64 {
    let is_buy = amount > 0.0;
    let mut remaining = amount.abs();
    let mut cost = 0.0;
    let mut total_quantity = 0.0;

    // Use the appropriate order book side
    let book_side = if is_buy { asks } else { bids };

    // Iterate over the order book
    for level in book_side {
        // Parse the price and size from the level
        let px: f64 = level.px.parse().unwrap_or(0.0);
        let sz: f64 = level.sz.parse().unwrap_or(0.0);

        // Determine how much we can trade at this level
        let available = sz.min(remaining / px);
        cost += available * px;
        remaining -= available * px;
        total_quantity += available;

        // If we've traded the entire amount, stop
        if remaining <= 0.0 {
            break;
        }
    }

    // If there was not enough liquidity to trade the entire amount, return 0.0
    if remaining > 0.0 {
        return 0.0;
    }

    // Calculate the average price
    let avg_price = cost / total_quantity.abs();

    // Calculate the mid-price at the time the decision to trade was made
    let mid_price = (bids[0].px.parse::<f64>().unwrap_or(0.0) + asks[0].px.parse::<f64>().unwrap_or(0.0)) / 2.0;

    // Calculate the slippage
    let slippage = (avg_price - mid_price).abs() / avg_price;

    slippage * 10_000.0
}