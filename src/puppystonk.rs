use anyhow::anyhow;
use rusty_money::iso;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Stonk {
    chart: Chart,
}

#[derive(Deserialize, Debug)]
struct Chart {
    result: Vec<Result>,
}

#[derive(Deserialize, Debug)]
struct Result {
    meta: Meta,
    timestamp: Vec<Option<i64>>,
    indicators: Indicators,
}

#[derive(Deserialize, Debug)]
struct Meta {
    currency: String,

    #[serde(rename = "regularMarketPrice")]
    regular_market_price: f64, // seems to come pre-rounded
    //
    #[serde(rename = "previousClose")]
    previous_close: f64,
}

#[derive(Deserialize, Debug)]
struct Indicators {
    quote: Vec<Quote>,
}

#[derive(Deserialize, Debug)]
struct Quote {
    close: Vec<Option<f64>>,
}

fn find_min_max_i64(numbers: &[Option<i64>]) -> (Option<i64>, Option<i64>) {
    let filtered_numbers: Vec<i64> = numbers.into_iter().filter_map(|&x| x).collect();

    let max_number = filtered_numbers.iter().cloned().max();
    let min_number = filtered_numbers.iter().cloned().min();

    (min_number, max_number)
}

fn find_min_max_f64(numbers: &[Option<f64>]) -> (Option<f64>, Option<f64>) {
    let mut min_number: Option<f64> = None;
    let mut max_number: Option<f64> = None;

    for number in numbers.iter().filter_map(|&x| x) {
        match min_number {
            None => min_number = Some(number),
            Some(min) => {
                if number.is_finite() && number < min {
                    min_number = Some(number)
                }
            }
        }
        match max_number {
            None => max_number = Some(number),
            Some(max) => {
                if number.is_finite() && number > max {
                    max_number = Some(number)
                }
            }
        }
    }

    (min_number, max_number)
}

fn plot_svg(result: &Result) -> anyhow::Result<String> {
    let quote = &result.indicators.quote[0].close;

    let (min_ts, max_ts) = find_min_max_i64(&result.timestamp);
    let min_ts: i64 = min_ts.ok_or_else(|| anyhow!("no min ts found"))?;
    let max_ts: i64 = max_ts.ok_or_else(|| anyhow!("no max ts found"))?;

    let (min_quote, max_quote) = find_min_max_f64(quote);
    let min_quote: f64 = min_quote.ok_or_else(|| anyhow!("no min quote found"))?;
    let max_quote: f64 = max_quote.ok_or_else(|| anyhow!("no max quote found"))?;

    const WIDTH: i64 = 512;
    const HEIGHT: i64 = 256;
    let mut svg_out: String =
        format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" fill="#000">"##).to_string();
    svg_out.push_str(r##"<polyline fill="none" stroke="#f00" points=""##);

    for pair in result.timestamp.iter().zip(quote.iter()) {
        if let (Some(timestamp), Some(close)) = pair {
            let x = (timestamp - min_ts) * WIDTH / (max_ts - min_ts);
            let y = (close - min_quote) * (HEIGHT as f64) / (max_quote - min_quote);
            svg_out.push_str(&format!("{},{} ", x, y).to_string());
            println!("{} {}", timestamp, close);
        }
    }
    svg_out.push_str(r##""/></svg>"##);
    Ok(svg_out)
}

pub async fn stonk(ticker: &str) -> anyhow::Result<String> {
    // TODO use a source that has not been officially discontinued
    let stonk_url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}",
        ticker
    );

    let stonk_result: &Result = &reqwest::get(&stonk_url)
        .await?
        .json::<Stonk>()
        .await?
        .chart
        .result[0];

    plot_svg(&stonk_result);
    let currency = iso::find(&stonk_result.meta.currency)
        .expect("currency code missing from response metadata");

    let emoji = if stonk_result.meta.regular_market_price > stonk_result.meta.previous_close {
        "<:puprocket:1213637827619848284>"
    } else {
        "<:ruprocket:1213637826374144000>"
    };

    let out = format!(
        "{}: {}{} {}",
        ticker, currency.symbol, stonk_result.meta.regular_market_price, emoji
    );
    Ok(out)
}
