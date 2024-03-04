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

fn plot_svg(result: &Result) -> anyhow::Result<(String, i64)> {
    let quote = &result.indicators.quote[0].close;

    let (min_ts, max_ts) = find_min_max_i64(&result.timestamp);
    let min_ts: i64 = min_ts.ok_or_else(|| anyhow!("no min ts found"))?;
    let max_ts: i64 = max_ts.ok_or_else(|| anyhow!("no max ts found"))?;

    let (min_quote, max_quote) = find_min_max_f64(quote);
    let mut min_quote: f64 = min_quote.ok_or_else(|| anyhow!("no min quote found"))?;
    let mut max_quote: f64 = max_quote.ok_or_else(|| anyhow!("no max quote found"))?;
    let previous_close = result.meta.previous_close;
    if previous_close < min_quote {
        min_quote = previous_close;
    }
    if previous_close > max_quote {
        max_quote = previous_close;
    }

    const WIDTH: i64 = 2048;
    const HEIGHT: i64 = 768;
    const FONTSIZE: i64 = 64;
    let color = if result.meta.regular_market_price > previous_close {
        "#3c1"
    } else {
        "#e21"
    };
    let grey = "#888";

    let close_y = |close: f64| -> f64 {
        (HEIGHT as f64) * (0.90 - 0.8 * (close - min_quote) / (max_quote - min_quote))
    };

    let mut svg_out: String = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}">
                    <style>.text {{ font: {FONTSIZE}px monospace; fill: {grey}; }}</style>"##
    );

    let last_close_y = close_y(previous_close);
    svg_out.push_str(
        &format!(r##"<line x1="0" y1="{last_close_y}" x2="{WIDTH}" y2="{last_close_y}"
                 stroke="{grey}" stroke-dasharray="16" stroke-width="4" />"##).to_string(),
    );

    svg_out.push_str(&format!(r##"<polyline fill="none" stroke="{color}" stroke-width="4" points=""##).to_string());

    for pair in result.timestamp.iter().zip(quote.iter()) {
        if let (Some(timestamp), Some(close)) = pair {
            let x = (timestamp - min_ts) * WIDTH / (max_ts - min_ts);
            let y = close_y(*close);
            svg_out.push_str(&format!("{x},{y} ").to_string());
        }
    }
    svg_out.push_str(r##""/>"##);

    let top = 10 + FONTSIZE;
    svg_out.push_str(
        &format!(r##"<text x="10" y="{top}" class="text">{max_quote:.2}</text>"##).to_string(),
    );

    let bottom = HEIGHT - 10;
    svg_out.push_str(
        &format!(r##"<text x="10" y="{bottom}" class="text">{min_quote:.2}</text>"##).to_string(),
    );
    svg_out.push_str(r##"</svg>"##);

    Ok((svg_out, max_ts))
}

fn save_png(svg: &str) -> anyhow::Result<String> {
    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    fontdb.set_monospace_family("DejaVu Sans Mono");
    let opt = resvg::usvg::Options::default();
    let rtree = resvg::usvg::Tree::from_str(svg, &opt, &fontdb)?;

    let pixmap_size = rtree.size();
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(pixmap_size.width() as u32, pixmap_size.height() as u32)
            .ok_or_else(|| anyhow!("couldn't allocate pixmap"))?;

    resvg::render(
        &rtree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let filename = format!("{}.png", uuid::Uuid::new_v4()).to_string();
    pixmap.save_png(&filename)?;
    Ok(filename)
}

pub async fn stonk(ticker: &str) -> anyhow::Result<(String, String, i64)> {
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

    let (svg, latest_ts) = plot_svg(&stonk_result)?;
    let filename = save_png(&svg)?;
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
    Ok((out, filename, latest_ts))
}
