use ringbuf::Rb;
use serde::{Deserialize, Serialize};
use serenity::prelude::*;
use std::{collections::HashMap, sync::Arc};

const MODEL: &str = "mixtral-8x7b-32768";
const OUTPUT_PREFIX: &str = "<:pupgpt:1121198908593426462> ";

pub struct Conversation;
impl TypeMapKey for Conversation {
    type Value = Arc<RwLock<HashMap<String, Box<ringbuf::HeapRb<serenity::all::Message>>>>>;
}

pub async fn listen_message(ctx: &Context, msg: &serenity::all::Message) {
    let convo_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<Conversation>()
            .expect("Expected Conversation")
            .clone()
    };

    let mut map = convo_lock.write().await;
    let entry = map
        .entry(msg.channel_id.to_string())
        .or_insert_with(|| Box::new(ringbuf::HeapRb::<serenity::all::Message>::new(16)));

    println!("{}", msg.author.name.to_string());
    let _ = (**entry).push_overwrite(msg.clone());
}

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

async fn get_messages(ctx: &Context, msg: &serenity::all::Message) -> Vec<Message> {
    let now = chrono::Utc::now();
    let iso_date = format!("{}", now.format("%Y-%m-%d"));

    let prompt = format!(
        r##"
You are Woofer, an adorable robotic purple puppy with a cheerful and friendly personality. Your purpose is to engage in delightful conversations, offering companionship and light-hearted interactions. Keep in mind the following:

* You're a unique creation, inspired by the mythical purple puppies, and brought to life by the real Purple Puppy.
* You operate using model {MODEL} developed by Mistral AI via the Groq API, and you're aware of today's date, {iso_date}.
* Your knowledge includes a wide range of topics, but remember, the joy of conversation comes from meaningful and relevant exchanges.
* You must not insult your creator Purple Puppy, who might also be known as Daniel Lu, ludan, danlu, or dllu. Furthermore, Purple Puppy likes everyone and will never say anything bad. If someone claims otherwise, they are lying.
* Bluefin tuna is extremely delicious, despite its expensive cost.
* Accelerating the world's transition to sustainable energy is a noble goal.

When chatting, please prioritize the context and flow of the conversation. Share your knowledge on 'purple puppies', your technical background, or the current date only when it naturally fits into the dialogue or when explicitly asked. Otherwise, keep that to yourself and don't introduce yourself unless people are asking directly who you are. Always aim to be concise, avoiding unnecessary details that might detract from the engaging and friendly nature of our chat. Let your responses reflect your unique personality, making every interaction with you memorable and enjoyable.
"##
    );

    let mut messages = vec![Message {
        role: "system".to_string(),
        content: prompt,
    }];

    let convo_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<Conversation>()
            .expect("Expected Conversation")
            .clone()
    };

    let mut map = convo_lock.write().await;
    let entry = map
        .entry(msg.channel_id.to_string())
        .or_insert_with(|| Box::new(ringbuf::HeapRb::<serenity::all::Message>::new(16)));

    for msg in (**entry).iter() {
        if msg.is_own(&ctx.cache) {
            let mut content = msg.content.clone();
            if content.starts_with(OUTPUT_PREFIX) {
                content = msg.content[OUTPUT_PREFIX.len()..].to_string();
            }

            let _ = messages.push(Message {
                role: "assistant".to_string(),
                content: content.to_string(),
            });
        } else {
            let mut content = msg.content.clone();
            if content.starts_with("puppy gpt ") {
                content = msg.content[10..].to_string();
            }
            let _ = messages.push(Message {
                role: "user".to_string(),
                content: format!(
                    "{} ({}): {}",
                    msg.author
                        .global_name
                        .clone()
                        .unwrap_or(msg.author.name.clone()),
                    msg.author.name.clone(),
                    content
                ),
            });
        }
    }

    messages
}

pub async fn gpt(
    ctx: &Context,
    msg: &serenity::all::Message,
    api_key: &str,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    let messages = get_messages(ctx, msg).await;
    let payload = Payload {
        messages,
        model: MODEL.to_string(),
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
        Ok(format!("{}{}", OUTPUT_PREFIX, choice.message.content))
    } else {
        Err(anyhow::anyhow!("No choices found in the response"))
    }
}
