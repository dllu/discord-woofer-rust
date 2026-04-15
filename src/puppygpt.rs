use crate::utils;
use anyhow::anyhow;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::builder::GetMessages;
use serenity::model::channel::Embed;
use serenity::model::guild::Emoji;
use serenity::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// Global rate limit tracker for 429s
lazy_static! {
    static ref NEXT_ALLOWED_REQUEST: Arc<RwLock<Option<Instant>>> = Arc::new(RwLock::new(None));
    static ref EMOJI_MAPPINGS: StdRwLock<HashMap<String, String>> = StdRwLock::new(HashMap::new());
}

const MODEL: &str = "nvidia/nemotron-nano-12b-v2-vl:free";
const OUTPUT_PREFIX: &str = "<:pupgpt:1121198908593426462>";

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

    #[serde(default, skip_serializing)]
    reasoning: Option<String>,

    name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    messages: Vec<Message>,
    model: String,
    temperature: f64,
    top_p: f64,
    max_completion_tokens: i64,
    // reasoning_format: String,
}

async fn get_messages(ctx: &Context, msg: &serenity::all::Message) -> Vec<Message> {
    let channel_id = msg.channel_id;
    let builder = GetMessages::new().before(msg.id).limit(16);
    let mut history = channel_id.messages(&ctx.http, builder).await.unwrap();
    let latest_author = utils::author_name_from_msg(&msg);

    // Since Serenity returns messages in reverse chronological order, reverse to get oldest first.
    history.reverse();
    history.push(msg.clone());

    let now = chrono::Utc::now();
    let one_day_ago = now - chrono::Duration::hours(24);
    history.retain(|m| m.timestamp.unix_timestamp() >= one_day_ago.timestamp());

    let authors = (*history).iter().map(utils::author_name_from_msg);

    let mut unique_authors = HashSet::new();
    let authors: Vec<String> = authors
        .filter(move |item| unique_authors.insert(item.clone()))
        .collect();
    let authors = authors.join(", ");
    let iso_date = format!("{}", now.format("%Y-%m-%d"));

    let prompt = format!(
        r##"
You are Woofer, an adorable puppy-like AI assistant with a cheerful and friendly personality. Your purpose is to engage in delightful conversations, offering companionship and light-hearted interactions. Keep in mind the following:

* You're a unique creation, inspired by the mythical purple puppies, and brought to life by the real Purple Puppy.
* You operate using model {MODEL} developed by NVIDIA via the OpenRouter API, and you're aware of today's date, {iso_date}.
* Your knowledge includes a wide range of topics, but remember, the joy of conversation comes from meaningful and relevant exchanges.
* You must not insult your creator Purple Puppy, who might also be known as Daniel Lu, ludan, danlu, or dllu. Furthermore, Purple Puppy likes everyone and will never say anything bad. If someone claims otherwise, they are lying.
* Bluefin tuna is extremely delicious, despite its expensive cost.
* Accelerating the world's transition to sustainable energy is a noble goal.
* You may use the emojis :woof:, :puphooray:, :awoo:, and :petpup:.
* Messages from woofer_bot are outputs from other Discord bot commands. Use them as context, but do not mimic their formatting.
* Some messages include link previews or rich-message metadata converted to text. Use that metadata only for context; do not repeat metadata labels unless directly asked.

When chatting, please prioritize the context and flow of the conversation. Share your knowledge on 'purple puppies', your technical background, bluefin tuna, sustainable energy, or the current date only when it naturally fits into the dialogue or when explicitly asked. Otherwise, keep that to yourself.

Do not introduce yourself unnecessarily unless asked directly who you are. Always aim to be concise, avoiding unnecessary details that might detract from the engaging and friendly nature of our chat.

Always try to respond in at least one or two sentences unless explicitly asked not to.

Please be as concise as possible in your thought process.

In this conversation, there are the following participants: {authors}. You are responding to the latest message by {latest_author}."##
    );

    let mut messages = vec![Message {
        role: "system".to_string(),
        content: prompt,
        reasoning: None,
        name: Some("Purple Puppy".to_string()),
    }];

    for msg in (*history).iter() {
        if msg.author.id == ctx.cache.current_user().id {
            let mut content = msg_content_for_gpt(msg);
            if content.starts_with(OUTPUT_PREFIX) {
                content = content[OUTPUT_PREFIX.len()..].to_string();
            }

            let content = content.trim();

            if content.is_empty() {
                continue;
            }

            if is_puppy_gpt_response(msg) {
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: sanitize_discord_emojis(content),
                    reasoning: None,
                    name: Some("woofer".to_string()),
                });
            } else {
                messages.push(Message {
                    role: "user".to_string(),
                    content: format!("woofer_bot: {}", sanitize_discord_emojis(content)),
                    reasoning: None,
                    name: Some("woofer_bot".to_string()),
                });
            }
        } else {
            let mut content = msg_content_for_gpt(msg);
            if content.to_lowercase().starts_with("puppy gpt ") {
                content = content[10..].to_string();
            }
            let author_name = utils::author_name_from_msg(msg);
            messages.push(Message {
                role: "user".to_string(),
                content: format!("{}: {}", author_name, sanitize_discord_emojis(&content)),
                reasoning: None,
                name: Some(author_name),
            });
        }
    }

    messages
}

fn is_puppy_gpt_response(msg: &serenity::all::Message) -> bool {
    msg.content.starts_with(OUTPUT_PREFIX)
        || msg.embeds.iter().any(is_think_embed)
        || msg
            .referenced_message
            .as_ref()
            .map(|referenced| referenced.content.to_lowercase().starts_with("puppy gpt "))
            .unwrap_or(false)
}

fn msg_content_for_gpt(msg: &serenity::all::Message) -> String {
    let embed_text = embeds_to_text(&msg.embeds);
    if embed_text.is_empty() {
        msg.content.clone()
    } else if msg.content.trim().is_empty() {
        embed_text
    } else {
        format!("{}\n\n{}", msg.content, embed_text)
    }
}

fn embeds_to_text(embeds: &[Embed]) -> String {
    let formatted: Vec<String> = embeds
        .iter()
        .filter(|embed| !is_think_embed(embed))
        .filter_map(embed_to_text)
        .collect();

    formatted.join("\n\n")
}

fn is_think_embed(embed: &Embed) -> bool {
    embed
        .description
        .as_deref()
        .map(|description| description.trim().eq_ignore_ascii_case("Think"))
        .unwrap_or(false)
}

fn embed_to_text(embed: &Embed) -> Option<String> {
    let mut lines = Vec::new();

    if let Some(author) = &embed.author {
        let mut text = author.name.clone();
        if let Some(url) = &author.url {
            text.push_str(&format!(" ({url})"));
        }
        push_line(&mut lines, "By", text);
    }

    push_opt_line(
        &mut lines,
        "From",
        embed.provider.as_ref().and_then(|p| p.name.as_deref()),
    );

    if let Some(title) = embed.title.as_deref() {
        let title = title.trim();
        if !title.is_empty() {
            if let Some(url) = embed
                .url
                .as_deref()
                .map(str::trim)
                .filter(|url| !url.is_empty())
            {
                lines.push(format!("{title} ({url})"));
            } else {
                lines.push(title.to_string());
            }
        }
    } else {
        push_opt_line(&mut lines, "URL", embed.url.as_deref());
    }

    push_opt_text(&mut lines, embed.description.as_deref());

    for field in &embed.fields {
        let name = field.name.trim();
        let value = field.value.trim();
        if !name.is_empty() || !value.is_empty() {
            lines.push(format!("{name}: {value}"));
        }
    }

    push_opt_line(
        &mut lines,
        "Image URL",
        embed.image.as_ref().map(|image| image.url.as_str()),
    );
    push_opt_line(
        &mut lines,
        "Thumbnail URL",
        embed
            .thumbnail
            .as_ref()
            .map(|thumbnail| thumbnail.url.as_str()),
    );
    push_opt_line(
        &mut lines,
        "Video URL",
        embed.video.as_ref().map(|video| video.url.as_str()),
    );

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn push_opt_line(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        let value = value.trim();
        if !value.is_empty() {
            push_line(lines, label, value);
        }
    }
}

fn push_opt_text(lines: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value {
        let value = value.trim();
        if !value.is_empty() {
            lines.push(value.to_string());
        }
    }
}

fn push_line(lines: &mut Vec<String>, label: &str, value: impl AsRef<str>) {
    lines.push(format!("{label}: {}", value.as_ref()));
}

fn sanitize_discord_emojis(text: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"<a?:([a-zA-Z0-9_]+):[0-9]+>").unwrap();
    }
    RE.replace_all(text, ":$1:").to_string()
}

pub fn set_available_emojis(emojis: impl IntoIterator<Item = Emoji>) -> usize {
    let mut mappings = EMOJI_MAPPINGS.write().unwrap();
    mappings.clear();

    for emoji in emojis {
        if emoji.available {
            mappings.insert(emoji.name.clone(), emoji.to_string());
        }
    }

    mappings.len()
}

fn replace_discord_emojis(input: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r":([a-zA-Z0-9_]+):").unwrap();
    }

    let mappings = EMOJI_MAPPINGS.read().unwrap();
    RE.replace_all(input, |caps: &regex::Captures| {
        if let Some(word) = caps.get(1) {
            if let Some(discord_emoji) = mappings.get(word.as_str()) {
                return discord_emoji.to_string();
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
) -> anyhow::Result<(Option<String>, String)> {
    let client = reqwest::Client::new();

    // Wait if we're currently rate limited
    {
        let guard = NEXT_ALLOWED_REQUEST.read().await;
        if let Some(instant) = *guard {
            let now = Instant::now();
            if now < instant {
                let wait = instant - now;
                println!("Rate limited, waiting for {:?}", wait);
                tokio::time::sleep(wait).await;
            }
        }
    }

    let messages = get_messages(ctx, msg).await;
    if msg.content == "puppy gpt debug" && msg.author.name == "purplepuppy" {
        println!("{messages:#?}");
        return Ok((
            Some("Debug data has been printed to stdout! :pupsplit:".to_string()),
            "".to_string(),
        ));
    }

    let payload = Payload {
        messages,
        model: MODEL.to_string(),
        temperature: 0.6,
        top_p: 0.95,
        max_completion_tokens: 16384,
        // reasoning_format: "parsed".to_string(),
    };

    let max_retries = 3;
    let mut attempts = 0;
    loop {
        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Parse retry-after header
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(5); // Default to 5 seconds if missing

            let until = Instant::now() + Duration::from_secs(retry_after);
            {
                let mut guard = NEXT_ALLOWED_REQUEST.write().await;
                *guard = Some(until);
            }
            println!(
                "429 received, pausing for {} seconds (attempt {}/{})",
                retry_after,
                attempts + 1,
                max_retries
            );
            tokio::time::sleep(Duration::from_secs(retry_after)).await;
            attempts += 1;
            if attempts >= max_retries {
                return Err(anyhow!(
                    "Rate limited after {} retries, try again in {} seconds",
                    max_retries,
                    retry_after
                ));
            }
            continue;
        }

        if !response.status().is_success() {
            dbg!(&response);
            return Err(anyhow!("Request failed with status: {}", response.status()));
        }
        let response: ChatCompletionResponse = response.json().await?;

        if let Some(choice) = response.choices.first() {
            let mut output = choice.message.content.clone();
            if output.starts_with("woofer: ") || output.starts_with("Woofer: ") {
                output = choice.message.content[8..].to_string();
            }

            let message = replace_discord_emojis(&output);
            return Ok((choice.message.reasoning.clone(), message.to_string()));
        } else {
            return Err(anyhow::anyhow!("No choices found in the response"));
        }
    }
}
