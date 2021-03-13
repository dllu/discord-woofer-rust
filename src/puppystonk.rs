extern crate reqwest;
extern crate url;

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
    timestamp: Vec<i64>,
    indicators: Indicators,
}

#[derive(Deserialize, Debug)]
struct Indicators {
    quote: Vec<Quote>,
}

#[derive(Deserialize, Debug)]
struct Quote {
    open: Vec<Option<f64>>,
    close: Vec<Option<f64>>,
    low: Vec<Option<f64>>,
    high: Vec<Option<f64>>,
}

pub async fn stonk(ticker: &str) -> anyhow::Result<String> {
    let stonk_url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}",
        ticker
    );
    let stonk_response: Stonk = reqwest::get(&stonk_url).await?.json().await?;
    let out = format!(
        "{}: ${:.2}",
        ticker,
        stonk_response.chart.result[0].indicators.quote[0]
            .close
            .last()
            .expect("must have stonk")
            .expect("wtf")
    );
    Ok(out)
}
