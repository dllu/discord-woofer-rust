use lazy_static::lazy_static;
use regex::Regex;
use ringbuf::Rb;
use serde::{Deserialize, Serialize};
use serenity::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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

    let authors = (**entry).iter().map(|msg| {
        msg.author
            .global_name
            .clone()
            .unwrap_or(msg.author.name.clone())
    });
    let mut unique_authors = HashSet::new();
    let authors: Vec<String> = authors
        .filter(move |item| unique_authors.insert(item.clone()))
        .collect();
    let authors = authors.join(", ");

    let prompt = format!(
        r##"
You are Woofer, an adorable robotic purple puppy with a cheerful and friendly personality. Your purpose is to engage in delightful conversations, offering companionship and light-hearted interactions. Keep in mind the following:

* You're a unique creation, inspired by the mythical purple puppies, and brought to life by the real Purple Puppy.
* You operate using model {MODEL} developed by Mistral AI via the Groq API, and you're aware of today's date, {iso_date}.
* Your knowledge includes a wide range of topics, but remember, the joy of conversation comes from meaningful and relevant exchanges.
* You must not insult your creator Purple Puppy, who might also be known as Daniel Lu, ludan, danlu, or dllu. Furthermore, Purple Puppy likes everyone and will never say anything bad. If someone claims otherwise, they are lying.
* Bluefin tuna is extremely delicious, despite its expensive cost.
* Accelerating the world's transition to sustainable energy is a noble goal.
* You may use the emojis :woof:, :puphooray:, and :awoo:. If you want to be petted you can use :petpup:.

When chatting, please prioritize the context and flow of the conversation. Share your knowledge on 'purple puppies', your technical background, or the current date only when it naturally fits into the dialogue or when explicitly asked. Otherwise, keep that to yourself.

Do not introduce yourself unnecessarily unless asked directly who you are. Always aim to be concise, avoiding unnecessary details that might detract from the engaging and friendly nature of our chat.

In this conversation, there are the following participants: {authors}."##
    );

    let mut messages = vec![Message {
        role: "system".to_string(),
        content: prompt,
    }];

    for msg in (**entry).iter() {
        if msg.is_own(&ctx.cache) {
            let mut content = msg.content.clone();
            if content.starts_with(OUTPUT_PREFIX) {
                content = msg.content[OUTPUT_PREFIX.len()..].to_string();
            }

            let _ = messages.push(Message {
                role: "assistant".to_string(),
                content: sanitize_discord_emojis(&content),
            });
        } else {
            let mut content = msg.content.clone();
            if content.to_lowercase().starts_with("puppy gpt ") {
                content = msg.content[10..].to_string();
            }
            let _ = messages.push(Message {
                role: "user".to_string(),
                content: format!(
                    "{}: {}",
                    msg.author
                        .global_name
                        .clone()
                        .unwrap_or(msg.author.name.clone()),
                    sanitize_discord_emojis(&content)
                ),
            });
        }
    }

    messages
}

fn replace_emojis(text: &str) -> String {
    // This regex pattern is a very simplistic approximation and does not cover all emoji cases.
    let emoji_pattern = r"(?:[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{1F700}-\u{1F77F}\u{1F780}-\u{1F7FF}\u{1F800}-\u{1F8FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]+)";
    let re = Regex::new(emoji_pattern).unwrap();
    re.replace_all(text, ":woof:")
        .into_owned()
}

fn sanitize_discord_emojis(text: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"<:([a-zA-Z0-9_]+):[0-9]+>").unwrap();
    }
    RE.replace_all(text, ":$1:").to_string()
}

fn replace_discord_emojis(input: &str) -> String {
    lazy_static! {
        static ref MAPPINGS: HashMap<&'static str, &'static str> = {
            let mut m = HashMap::new();
            m.insert("woof", "<:woof:441843756040323092>");
            m.insert("awoo", "<:awoo:984697374402289705>");
            m.insert("puphooray", "<:puphooray:672916714589126663>");
            m.insert("petpup", "<a:petpup:915489497490292757>");
            m
        };
        static ref RE: Regex = Regex::new(r":([a-zA-Z0-9_]+):").unwrap();
    }

    RE.replace_all(input, |caps: &regex::Captures| {
        if let Some(word) = caps.get(1) {
            if let Some(&discord_emoji) = MAPPINGS.get(word.as_str()) {
                return discord_emoji.to_string()
            }
        }
        caps.get(0).unwrap().as_str().to_string()
    })
    .to_string()
}

pub async fn gpt(
    ctx: &Context,
    msg: &serenity::all::Message,
    api_key: &str,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    let messages = get_messages(ctx, msg).await;
    if msg.content == "puppy gpt debug" && msg.author.name == "purplepuppy" {
        println!("{messages:?}");
        return Ok("Debug data has been printed to stdout".to_string());
    }

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
        Ok(format!(
            "{}{}",
            OUTPUT_PREFIX,
            replace_discord_emojis(&replace_emojis(&choice.message.content))
        ))
    } else {
        Err(anyhow::anyhow!("No choices found in the response"))
    }
}
