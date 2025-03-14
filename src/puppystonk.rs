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
    let filtered_numbers: Vec<i64> = numbers.iter().filter_map(|&x| x).collect();

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

    use chrono::{TimeZone, Utc};
    use chrono_tz::America::New_York;
    let first_dt = Utc
        .timestamp_opt(min_ts, 0)
        .single()
        .ok_or_else(|| anyhow!("Invalid timestamp: {}", min_ts))?
        .with_timezone(&New_York);
    let naive_date = first_dt.date_naive();
    let market_open_naive = naive_date
        .and_hms_opt(9, 30, 0)
        .ok_or_else(|| anyhow!("Invalid market open time"))?;
    let market_close_naive = naive_date
        .and_hms_opt(16, 0, 0)
        .ok_or_else(|| anyhow!("Invalid market close time"))?;
    let market_open = New_York
        .from_local_datetime(&market_open_naive)
        .single()
        .ok_or_else(|| anyhow!("Ambiguous market open datetime"))?;
    let market_close = New_York
        .from_local_datetime(&market_close_naive)
        .single()
        .ok_or_else(|| anyhow!("Ambiguous market close datetime"))?;
    let market_open_utc = market_open.with_timezone(&Utc).timestamp();
    let market_close_utc = market_close.with_timezone(&Utc).timestamp();

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
        &format!(
            r##"<line x1="0" y1="{last_close_y}" x2="{WIDTH}" y2="{last_close_y}"
                 stroke="{grey}" stroke-dasharray="16" stroke-width="4" />"##
        )
        .to_string(),
    );

    if market_open_utc >= min_ts && market_open_utc <= max_ts {
        let open_x = (market_open_utc - min_ts) * WIDTH / (max_ts - min_ts);
        svg_out.push_str(
            &format!(
                r#"<line x1="{open_x}" y1="0" x2="{open_x}" y2="{HEIGHT}" stroke="{grey}" stroke-dasharray="16" stroke-width="4" />"#
            )
        );
    }
    if market_close_utc >= min_ts && market_close_utc <= max_ts {
        let close_x = (market_close_utc - min_ts) * WIDTH / (max_ts - min_ts);
        svg_out.push_str(
            &format!(
                r#"<line x1="{close_x}" y1="0" x2="{close_x}" y2="{HEIGHT}" stroke="{grey}" stroke-dasharray="16" stroke-width="4" />"#
            )
        );
    }

    svg_out.push_str(
        &format!(
            r##"<polyline fill="none" stroke="{color}" opacity="0.5" stroke-width="4" points=""##
        )
        .to_string(),
    );

    let mut svg_in_hours =
        format!(r##"<polyline fill="none" stroke="{color}" stroke-width="6" points=""##)
            .to_string();

    for pair in result.timestamp.iter().zip(quote.iter()) {
        if let (Some(timestamp), Some(close)) = pair {
            let x = (timestamp - min_ts) * WIDTH / (max_ts - min_ts);
            let y = close_y(*close);
            svg_out.push_str(&format!("{x},{y} ").to_string());
            if *timestamp >= market_open_utc && *timestamp <= market_close_utc {
                svg_in_hours.push_str(&format!("{x},{y} ").to_string());
            }
        }
    }
    svg_out.push_str(r##""/>"##);
    svg_in_hours.push_str(r##""/>"##);
    svg_out.push_str(&svg_in_hours);

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

fn stonk_to_image(stonk_result: &Result, ticker: &str) -> anyhow::Result<(String, String, i64)> {
    let (svg, latest_ts) = plot_svg(stonk_result)?;
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

pub async fn stonk(ticker: &str) -> anyhow::Result<(String, String, i64)> {
    // TODO use a source that has not been officially discontinued
    let stonk_url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?includePrePost=true",
        ticker
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/4.0 (compatible; MSIE 6.0; Windows NT 5.2; .NET CLR 1.0.3705;)") // Mimic curl user-agent
        .build()?;

    let response = client.get(&stonk_url).send().await;

    match response {
        Ok(resp) => {
            // Check if the request was successful
            let status = resp.status();
            if status.is_success() {
                let stonk_result: &Result = &resp.json::<Stonk>().await?.chart.result[0];
                stonk_to_image(stonk_result, ticker)
            } else {
                Err(anyhow!("Request failed with status: {}", resp.status()))
            }
        }
        Err(e) => Err(anyhow!("Request failed cuz: {}", e)),
    }
}
