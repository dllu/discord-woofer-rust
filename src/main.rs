#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate reqwest;
extern crate serenity;
extern crate url;
#[macro_use]
extern crate serde;

use regex::Regex;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::{collections::HashMap, env, sync::Arc};

mod puppyweather;
mod puppywhy;
use shakmaty::Position;

struct Handler {
    openweather_token: String,
    google_maps_token: String,
}

struct ChessGame;
impl TypeMapKey for ChessGame {
    type Value = Arc<RwLock<HashMap<String, Box<shakmaty::Chess>>>>;
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.is_own(&ctx.cache).await {
            return;
        }
        lazy_static! {
            static ref WOOF_RE: Regex = Regex::new(
                r"^((oua+f+\s*)+|(w(a|o|0|u)+r*f\s*)+|(aw+(o|0)+\s*)+|(b(a|o)+rk\s*)+)+!*$"
            )
            .unwrap();
            static ref WEATHER_RE: Regex = Regex::new(r"^puppy weather\s\w+").unwrap();
            static ref CHESS_RE: Regex = Regex::new(r"^puppy chess\s\w*").unwrap();
        }
        let ref content = &msg.content;
        let lower = content.to_lowercase();
        if WOOF_RE.is_match(&lower) {
            if let Err(why) = msg.reply(&ctx.http, content).await {
                println!("Error sending message: {:?}", why);
            }
        } else if lower == "puppy why" {
            if let Err(why) = msg.reply(&ctx.http, puppywhy::why()).await {
                println!("Error sending message: {:?}", why);
            }
        } else if lower == "puppy how" {
            if let Err(why) = msg
                .reply(&ctx.http, "https://github.com/dllu/discord-woofer-rust")
                .await
            {
                println!("Error sending message: {:?}", why);
            }
        } else if WEATHER_RE.is_match(&lower) {
            let address = &lower[14..];
            // TODO: error handlin
            let location = puppyweather::geocode(address.to_string(), &self.google_maps_token)
                .await
                .unwrap();
            let weather = puppyweather::weather(&location, &self.openweather_token)
                .await
                .unwrap();
            let response = puppyweather::weather_string(address.to_string(), &location, weather);
            if let Err(why) = msg.reply(&ctx.http, response).await {
                println!("Error sending message: {:?}", why);
            }
        } else if CHESS_RE.is_match(&lower) {
            let san_str = &content[12..];
            let san_res = san_str.parse();
            let san: shakmaty::san::San;
            match san_res {
                Ok(s) => {
                    san = s;
                }
                Err(_) => {
                    if let Err(why) = msg.reply(&ctx.http, "Illegal move!!!!!!").await {
                        println!("Error sending message: {:?}", why);
                    }
                    return;
                }
            }
            let game_lock = {
                let data_read = ctx.data.read().await;
                data_read
                    .get::<ChessGame>()
                    .expect("Expected ChessGame")
                    .clone()
            };
            {
                let mut map = game_lock.write().await;
                let entry = map
                    .entry(msg.author.id.to_string())
                    .or_insert(Box::new(shakmaty::Chess::default()));
                let pos = &**entry;
                let mov = san.to_move(pos);
                match mov {
                    Ok(m) => {
                        let pos2 = pos.clone().play(&m);
                        match pos2 {
                            Ok(p) => {
                                let fen = shakmaty::fen::epd(&p);
                                **entry = p;
                                if let Some(f) = fen.split(" ").next() {
                                    if let Err(why) = msg
                                        .reply(
                                            &ctx.http,
                                            format!("https://chess.dllu.net/{}.png", f),
                                        )
                                        .await
                                    {
                                        println!("Error sending message: {:?}", why);
                                    }
                                }
                            }
                            Err(_) => {
                                if let Err(why) = msg.reply(&ctx.http, "Illegal move!!!!!!").await {
                                    println!("Error sending message: {:?}", why);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        if let Err(why) = msg.reply(&ctx.http, "Illegal move!!!!!!").await {
                            println!("Error sending message: {:?}", why);
                        }
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

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let handler = Handler {
        openweather_token,
        google_maps_token,
    };

    let mut client = Client::builder(&discord_token)
        .event_handler(handler)
        .await
        .expect("Err creating client");
    {
        let mut data = client.data.write().await;
        data.insert::<ChessGame>(Arc::new(RwLock::new(HashMap::default())));
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
