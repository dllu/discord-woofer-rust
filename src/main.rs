use lazy_static::lazy_static;
use regex::Regex;
use serenity::all::CreateEmbedFooter;
use serenity::builder::{CreateAttachment, CreateEmbed, CreateMessage};
use serenity::model::Timestamp;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::{collections::HashMap, env, fs::File, io::prelude::*, sync::Arc};

mod puppychess;
mod puppygpt;
mod puppystonk;
mod puppyweather;
mod puppywhy;
mod utils;
use rand::prelude::SliceRandom;

struct Handler {
    openweather_token: String,
    google_maps_token: String,
    groq_token: String,
    avwx_token: String,
    no_reasons: Vec<String>,
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.is_own(&ctx.cache) {
            return;
        }

        lazy_static! {
            static ref WOOF_RE: Regex = Regex::new(
                r"^((oua+f+\s*)+|(w(a|o|0|u|🌕)+r*f\s*)+|(aw+(o|0|🌕)+\s*)+|(b(a|o)+rk\s*)+|(汪\s*)+|(ワン\s*)+|(わん\s*)+|(гав\s*)+|(uowhf\s*)+|(arflee+bloo+\s*)+)+(!|！)*$"
            )
            .unwrap();
            static ref WEATHER_RE: Regex = Regex::new(r"^puppy weather\s\w+").unwrap();
            static ref METAR_RE: Regex = Regex::new(r"^puppy metar\s\w+").unwrap();
            static ref STONK_RE: Regex = Regex::new(r"^puppy stonk\s\w+").unwrap();
            static ref CHESS_RE: Regex = Regex::new(r"^puppy chess\s\w*").unwrap();
            static ref GPT_RE: Regex = Regex::new(r"^puppy gpt\s\w*").unwrap();
        }
        const ERROR_MSG: &str = "<a:pupgone:1061133208676204605> It didn't work!";
        let content = &msg.content;
        let lower = content.to_lowercase();
        if WOOF_RE.is_match(&lower) {
            if let Err(why) = msg.reply(&ctx.http, content).await {
                eprintln!("Error sending message: {:?}", why);
            }
        } else if lower == "puppy why" {
            if let Err(why) = msg.reply(&ctx.http, puppywhy::why()).await {
                eprintln!("Error sending message: {:?}", why);
            }
        } else if lower == "puppy how" {
            if let Err(why) = msg
                .reply(&ctx.http, "https://github.com/dllu/discord-woofer-rust")
                .await
            {
                eprintln!("Error sending message: {:?}", why);
            }
        } else if lower == "puppy no" {
            let reason = self.no_reasons.choose(&mut rand::thread_rng()).unwrap();
            if let Err(why) = msg.reply(&ctx.http, reason).await {
                eprintln!("Error sending message: {:?}", why);
            }
        } else if STONK_RE.is_match(&lower) {
            let typing = msg.channel_id.start_typing(&ctx.http);
            let ticker = &lower[12..];
            let res = puppystonk::stonk(ticker).await;
            match res {
                Ok((stonk, filename, timestamp)) => {
                    typing.stop();
                    let embed = CreateEmbed::new()
                        .title(lower)
                        .description(stonk)
                        .image(format!("attachment://{filename}"))
                        .timestamp(Timestamp::from_unix_timestamp(timestamp).unwrap());
                    let builder = CreateMessage::new().embed(embed).add_file(
                        CreateAttachment::path(format!("./{filename}"))
                            .await
                            .unwrap(),
                    );

                    if let Err(why) = msg.channel_id.send_message(&ctx.http, builder).await {
                        println!("Error sending message: {why:?}");
                    }
                    std::fs::remove_file(filename).unwrap();
                }
                Err(whyy) => {
                    eprintln!("Error with getting stonk: {:?}", whyy);
                    if let Err(why) = msg.reply(&ctx.http, format!("{ERROR_MSG} {whyy:?}")).await {
                        eprintln!("Error sending message: {:?}", why);
                    }
                }
            }
        } else if WEATHER_RE.is_match(&lower) {
            let typing = msg.channel_id.start_typing(&ctx.http);
            let mut units = String::from("kelvin");
            let mut offset = 14; // base length of "puppy weather "
            lazy_static! {
                static ref CELSIUS_RE: Regex = Regex::new(r"^celsius\s\w*").unwrap();
                static ref FAHRENHEIT_RE: Regex = Regex::new(r"^fahrenheit\s\w*").unwrap();
                static ref KELVIN_RE: Regex = Regex::new(r"^kelvin\s\w*").unwrap();
            }
            if CELSIUS_RE.is_match(&lower[offset..]) {
                offset += 8; // base length of "celsius "
                units = String::from("celsius");
            } else if FAHRENHEIT_RE.is_match(&lower[offset..]) {
                offset += 11; // base length of "fahrenheit "
                units = String::from("fahrenheit");
            } else if KELVIN_RE.is_match(&lower[offset..]) {
                offset += 7; // base length of "kelvin "
            }
            let address = &lower[offset..];
            // TODO: error handlin
            let location = puppyweather::geocode(address.to_string(), &self.google_maps_token)
                .await
                .unwrap();
            let weather = puppyweather::weather(&location, &self.openweather_token)
                .await
                .unwrap();
            let response =
                puppyweather::weather_string(address.to_string(), &location, &units, weather);
            typing.stop();
            if let Err(why) = msg.reply(&ctx.http, response).await {
                eprintln!("Error sending message: {:?}", why);
            }
        } else if METAR_RE.is_match(&lower) {
            let typing = msg.channel_id.start_typing(&ctx.http);
            let address = &lower[12..];
            // TODO: error handlin
            let weather = puppyweather::metar(address, &self.avwx_token)
                .await
                .unwrap();
            typing.stop();
            if let Err(why) = msg.reply(&ctx.http, weather).await {
                eprintln!("Error sending message: {:?}", why);
            }
        } else if CHESS_RE.is_match(&lower) {
            let mut res = puppychess::chess(&ctx, &msg).await;
            if let Err(why2) = res {
                eprintln!("Error making chess move: {:?}", why2);

                res = puppychess::chess_illegal_move(&ctx, &msg).await;
            }
            if let Err(why) = puppychess::reply(&ctx, &msg, res.unwrap()).await {
                eprintln!("Error sending message: {:?}", why);
            }
        } else if GPT_RE.is_match(&lower) {
            let typing = msg.channel_id.start_typing(&ctx.http);
            let response = puppygpt::gpt(&ctx, &msg, &self.groq_token).await;
            match response {
                Ok((think, res)) => {
                    let parts = split_string(&res);

                    typing.stop();

                    for (i, part) in parts.iter().enumerate() {
                        let mut temp_file = None;
                        let mut builder =
                            CreateMessage::new().content(part).reference_message(&msg);
                        if i == 0 {
                            if let Some(ref thonk) = think {
                                if thonk.len() < 2000 {
                                    builder = builder.embed(
                                        CreateEmbed::new()
                                            .description("Think")
                                            .footer(CreateEmbedFooter::new(thonk)),
                                    );
                                } else {
                                    let filename =
                                        format!("think_{}.txt", blake3::hash(thonk.as_bytes()));
                                    let mut file = File::create(&filename).unwrap();
                                    file.write_all(thonk.as_bytes()).unwrap();
                                    let attachment =
                                        CreateAttachment::path(filename.to_string()).await.unwrap();

                                    builder = builder.add_file(attachment);
                                    temp_file = Some(filename);
                                }
                            }
                        }

                        if let Err(why) = msg.channel_id.send_message(&ctx.http, builder).await {
                            eprintln!("Error sending reply: {why:?}");
                        }
                        if let Some(filename) = temp_file {
                            if let Err(why) = std::fs::remove_file(filename) {
                                eprintln!("Error deleting temporary file: {:?}", why);
                            }
                        }
                    }
                }
                Err(why2) => {
                    typing.stop();

                    if let Err(why) = msg.reply(&ctx.http, format!("{ERROR_MSG} {why2:?}")).await {
                        eprintln!("Error sending message: {:?}", why);
                    }
                }
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let discord_token =
        env::var("DISCORD_TOKEN").expect("Expected $DISCORD_TOKEN in the environment");
    let openweather_token =
        env::var("FORECAST_TOKEN").expect("Expected $FORECAST_TOKEN in the environment");
    let google_maps_token =
        env::var("GOOGLE_MAPS_TOKEN").expect("Expected $GOOGLE_MAPS_TOKEN in the environment");
    let groq_token = env::var("GROQ_TOKEN").expect("Expected $GROQ_TOKEN in the environment");
    let avwx_token = env::var("AVWX_TOKEN").expect("Expected $AVWX_TOKEN in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut file = File::open("assets/no_reasons.json").expect("Failed to open no_reasons.json");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read no_reasons.json");
    let no_reasons: Vec<String> = serde_json::from_str(&contents).expect("Invalid JSON format");

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let handler = Handler {
        openweather_token,
        google_maps_token,
        groq_token,
        avwx_token,
        no_reasons,
    };

    let mut client = Client::builder(&discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");
    {
        let mut data = client.data.write().await;
        data.insert::<puppychess::ChessGame>(Arc::new(RwLock::new(HashMap::default())));
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

fn split_string(input: &str) -> Vec<String> {
    const MAX_LENGTH: usize = 2000;
    let mut result = Vec::new();

    // Check if the input is already within the limit.
    if input.len() <= MAX_LENGTH {
        result.push(input.to_string());
    } else {
        let mut start_index = 0;
        while start_index < input.len() {
            // Determine the end index for the current piece, ensuring we don't go beyond the input's length.
            let mut end_index = std::cmp::min(start_index + MAX_LENGTH, input.len());
            if end_index == input.len() {
                result.push(input[start_index..].to_string());
                break;
            }
            let current_piece = &input[start_index..end_index];

            if let Some(last_newline) = current_piece.rfind("\n\n") {
                end_index = start_index + last_newline + 1;
            } else if let Some(last_newline) = current_piece.rfind('\n') {
                end_index = start_index + last_newline + 1;
            } else if let Some(last_space) = current_piece.rfind(char::is_whitespace) {
                end_index = start_index + last_space + 1;
            }

            result.push(input[start_index..end_index].to_string());

            start_index = end_index;
        }
    }

    result
}
