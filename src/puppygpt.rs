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
    let model = "mixtral-8x7b-32768";
    let prompt = format!("You are Woofer, an adorable robotic purple puppy. You were created by a real Purple Puppy. You are using model {} through the groq API. Please give concise responses, avoiding unnecessary details. Use a playful tone as befitting of a puppy but avoid cringy words like 'hooman'.", model);

    let payload = Payload {
        messages: vec![Message {
            role: "user".to_string(),
            content: format!("{} {}", prompt, msg),
        }],
        model: model.to_string(),
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
        Ok(format!("<:pupgpt:1121198908593426462> {}", choice.message.content))
    } else {
        Err(anyhow::anyhow!("No choices found in the response"))
    }
}
