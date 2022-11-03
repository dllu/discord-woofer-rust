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
}

#[derive(Deserialize, Debug)]
struct Meta {
    currency: String,
    regularMarketPrice: f64, // seems to come pre-rounded
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

    let currency = iso::find(&stonk_result.meta.currency)
        .expect("currency code missing from response metadata");

    let out = format!(
        "{}: {}{}",
        ticker, currency.symbol, stonk_result.meta.regularMarketPrice
    );
    Ok(out)
}
