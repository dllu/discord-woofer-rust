use rusty_money::{Money, iso};
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
    // timestamp: Vec<i64>,
    indicators: Indicators,
    meta: Meta,
}

#[derive(Deserialize, Debug)]
struct Meta {
    currency: str,
    regularMarketPrice: f64, // seems to come pre-rounded
}


// obviated by use of regularMarketPrice
/*
#[derive(Deserialize, Debug)]
struct Indicators {
    quote: Vec<Quote>,
}

#[derive(Deserialize, Debug)]
struct Quote {
    // open: Vec<Option<f64>>,
    close: Vec<Option<f64>>,
    // low: Vec<Option<f64>>,
    // high: Vec<Option<f64>>,
}
*/

pub async fn stonk(ticker: &str) -> anyhow::Result<String> {
    // TODO use a source that has not been officially discontinued
    let stonk_url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}",
        ticker
    );
    let stonk_result: Result = reqwest::get(&stonk_url).await?.json().await?
        .chart.result[0];

    let currency = iso::find(&stonk_result.meta.currency)
        .expect("currency code missing from response metadata");

    let price = Money::from_str(
        &format!("{}", stonk_result.meta.regularMarketPrice),
        currency).unwrap();

    // let formatted = Money::from_str()
    let out = format!(
        "{}: {}",
        ticker,
        price
    );
    Ok(out)
}
