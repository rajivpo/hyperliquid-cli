use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{collections::VecDeque, error::Error, io,  time::Duration};
use ratatui::{
    backend::CrosstermBackend,
    Frame,
    widgets::{Block, Borders, Chart, Dataset, Axis, Paragraph, GraphType},
    prelude::{Backend, Style, Span, Color, Marker, Rect, Line},
    terminal::Terminal,
};
use hyperliquid_rust_sdk::{BaseUrl, InfoClient, Subscription, Message};
use tokio::{sync::mpsc::unbounded_channel, select};
use chrono::{DateTime, Utc};


#[derive(Parser, Debug)]
pub struct GraphBookArgs {
    /// Coin to get l2book for
    #[arg(short, long)]
    pub coin: String,

    /// Levels of l2book to show (only the top level will be graphed)
    #[arg(short, long, default_value_t = 10)]
    pub levels: u8,
}

struct PricePoint {
    time: DateTime<Utc>,
    bid_price: f64,
    ask_price: f64,
    slippage_10k: f64,
    slippage_100k: f64,
}

async fn run_app(args: GraphBookArgs) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = unbounded_channel();

    let mut info_client: InfoClient = InfoClient::new(None, Some(BaseUrl::Mainnet)).await.unwrap();
    info_client.subscribe(Subscription::L2Book { coin: args.coin.clone() }, tx).await.unwrap();

    let mut price_history: VecDeque<(DateTime<Utc>, f64, f64)> = VecDeque::new();
    let history_capacity = 100;

    loop {
        select! {
            Some(message) = rx.recv() => {
                match message {
                    Message::L2Book(l2_book) => {
                        if let (Some(highest_bid), Some(lowest_ask)) = (l2_book.data.levels[0].first(), l2_book.data.levels[1].first()) {
                            let bid_px: f64 = highest_bid.px.parse()?;
                            let ask_px: f64 = lowest_ask.px.parse()?;

                            if price_history.len() == history_capacity {
                                price_history.pop_front();
                            }
                            let now = Utc::now();
                            price_history.push_back((now, bid_px, ask_px));

                            // Ensure terminal drawing does not await or block
                            terminal.draw(|f| draw_ui(f, &price_history))?;

                            // Check for keyboard input to exit
                            if event::poll(Duration::ZERO)? {
                                if let CEvent::Key(key) = event::read()? {
                                    if key.code == KeyCode::Char('q') {
                                        break;
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            },
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw_ui<B: Backend>(f: &mut Frame<B>, price_history: &VecDeque<(DateTime<Utc>, f64, f64)>) {
    let size = f.size();

    // Calculate the height for the info block
    let info_block_height = 3;
    // Adjust the chart area to make room for the info block
    let chart_area = Rect::new(0, info_block_height, size.width, size.height - info_block_height);

    if let Some((time, bid, ask)) = price_history.back() {
        let spread = ask - bid;

        // Create a text to display the mid price and spread
        let text = vec![
            Line::from(vec![
                Span::raw("Bid Price: "),
                Span::styled(format!("{:.2}", bid), Style::default().fg(Color::White)),
                Span::raw(" | "),
                Span::raw("Ask Price: "),
                Span::styled(format!("{:.2}", ask), Style::default().fg(Color::White)),
                Span::raw(" | "),
                Span::raw("Spread: "),
                Span::styled(format!("{:.2}", spread), Style::default().fg(Color::White)),
                Span::raw(" | "),
                Span::raw("Time: "),
                Span::styled(time.format("%H:%M:%S.%3f").to_string(), Style::default().fg(Color::White)),
            ]),
        ];

        // Create a Paragraph widget and render it at the top
        let info_block = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Info"));
        // Render the info block at the very top
        f.render_widget(info_block, Rect::new(0, 0, size.width, info_block_height));
    }

    // Assuming min_price and max_price have been calculated earlier
    let min_price = price_history.iter().map(|(_, bid, ask)| bid.min(*ask)).fold(f64::INFINITY, f64::min);
    let max_price = price_history.iter().map(|(_, bid, ask)| bid.max(*ask)).fold(f64::NEG_INFINITY, f64::max);

    let y_ticks = generate_ticks(min_price, max_price);
    // println!("{:?}", y_ticks);

    // Assuming first and last timestamps have been determined from price_history
    if let (Some(first), Some(last)) = (price_history.front(), price_history.back()) {
        // Convert y_ticks and x_ticks into a format suitable for the charting library
        let y_labels: Vec<Span> = y_ticks.iter().map(|&(value, ref label)| {
            Span::styled(label.clone(), Style::default().fg(Color::White))
        }).collect();
    

        // Prepare datasets for bids and asks
        let bids: Vec<(f64, f64)> = price_history.iter().map(|(time, bid, _)| {
            let time_offset = time.signed_duration_since(first.0).num_seconds() as f64;
            (time_offset, *bid)
        }).collect();

        let asks: Vec<(f64, f64)> = price_history.iter().map(|(time, _, ask)| {
            let time_offset = time.signed_duration_since(first.0).num_seconds() as f64;
            (time_offset, *ask)
        }).collect();

        let datasets = vec![
            Dataset::default().name("Bids").marker(Marker::Dot).style(Style::default().fg(Color::Green)).graph_type(GraphType::Line).data(&bids),
            Dataset::default().name("Asks").marker(Marker::Dot).style(Style::default().fg(Color::Red)).graph_type(GraphType::Line).data(&asks),
        ];

        let total_duration = last.0.signed_duration_since(first.0).num_seconds() as f64;
        let x_ticks = generate_time_ticks(first.0, last.0, 30);


        let x_labels: Vec<Span> = x_ticks.iter().map(|&(seconds, ref label)| {
            Span::styled(label.clone(), Style::default().fg(Color::White))
        }).collect();

        // Render the chart with the datasets, x-axis, and y-axis configured
        let chart = Chart::new(datasets)
            .block(Block::default().title("L2 Book").borders(Borders::ALL))
            .x_axis(Axis::default().title("Time").style(Style::default().fg(Color::White)).bounds([0.0, total_duration]).labels(x_labels))
            .y_axis(Axis::default().title("Price").style(Style::default().fg(Color::White)).bounds([min_price, max_price]).labels(y_labels));

        f.render_widget(chart, chart_area);
    }
}

fn generate_ticks(min_price: f64, max_price: f64) -> Vec<(f64, String)> {
    let mut ticks = Vec::new();

    let mut current = min_price - 0.2;
    while current <= (max_price + 0.2) {
        let label = format!("{:.1}", current);
        ticks.push((current, label));
        current += 0.1;
    }

    ticks
}
fn generate_time_ticks(start: DateTime<Utc>, end: DateTime<Utc>, interval: i64) -> Vec<(f64, String)> {
    let start_seconds = start.timestamp();
    let end_seconds = end.timestamp();
    let duration = end_seconds - start_seconds;

    (0..=duration).step_by(interval as usize)
        .map(|s| {
            let current_time = start + chrono::Duration::seconds(s);
            // Use the actual minute and second for the label
            let label = current_time.format("%M:%S").to_string();
            (s as f64, label)
        })
        .collect()
}

pub async fn execute(args: GraphBookArgs) {
    match run_app(args).await {
        Ok(_) => println!("Exited successfully"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
