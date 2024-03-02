use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Choice {
    message: Message,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    messages: Vec<Message>,
    model: String,
}

pub async fn gpt(msg: &str, api_key: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    let payload = Payload {
        messages: vec![Message {
            role: "user".to_string(),
            content: msg.to_string(),
        }],
        model: "mixtral-8x7b-32768".to_string(),
    };

    let response: ChatCompletionResponse = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;

    if let Some(choice) = response.choices.get(0) {
        Ok(choice.message.content.clone())
    } else {
        Err(anyhow::anyhow!("No choices found in the response"))
    }
}
