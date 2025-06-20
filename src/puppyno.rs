use anyhow::anyhow;
use reqwest::{Response, Error};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct No {
  reason: String,
}


pub async fn no() -> anyhow::Result<String> {
  
  let url = "https://naas.isalman.dev/no";

  let client = reqwest::Client::builder()
      .user_agent("curl/7.68.0") // Mimic curl user-agent
      .build()?;

  let response: Result<Response, Error> = client.get(url).send().await;
  match response {
    Ok(resp) => {
        // Check if the request was successful
        let status = resp.status();
        if status.is_success() {
            let parsed_data = resp.json::<No>().await?;
            Ok(parsed_data.reason)
        } else {
            Err(anyhow!("Request failed with status: {}", resp.status()))
        }
    }
    Err(e) => Err(anyhow!("Request failed cuz: {}", e)),
  }
}