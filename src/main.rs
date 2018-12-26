#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate reqwest;
extern crate serenity;
extern crate url;
#[macro_use]
extern crate serde_derive;

use regex::Regex;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::env;

mod puppyweather;
mod puppywhy;

struct Handler {
    darksky_token: String,
    google_maps_token: String,
}

impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    fn message(&self, _: Context, msg: Message) {
        if msg.is_own() {
            return;
        }
        lazy_static! {
            static ref WOOF_RE: Regex =
                Regex::new(r"^pupper pupper\s+((oua+f+\s*)+|(w(a|o|0|u)+r*f\s*)+|(aw+(o|0)+\s*)+|(b(a|o)+rk\s*)+)+!*$").unwrap();
            static ref WEATHER_RE: Regex =
                Regex::new(r"^pupper pupper puppy weather\s\w+").unwrap();
        }
        let ref content = &msg.content;
        let lower = content.to_lowercase();
        if WOOF_RE.is_match(&lower) {
            if let Err(why) = msg.channel_id.say(content) {
                println!("Error sending message: {:?}", why);
            }
        } else if lower == "pupper pupper puppy why" {
            if let Err(why) = msg.channel_id.say(puppywhy::why()) {
                println!("Error sending message: {:?}", why);
            }
        } else if WEATHER_RE.is_match(&lower) {
            let address = &lower[28..];
            let location =
                puppyweather::geocode(address.to_string(), &self.google_maps_token).unwrap();
            let weather = puppyweather::weather(&location, &self.darksky_token).unwrap();
            let response = puppyweather::weather_string(address.to_string(), &location, weather);
            if let Err(why) = msg.channel_id.say(response) {
                println!("Error sending message: {:?}", why);
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

fn main() {
    let discord_token =
        env::var("DISCORD_TOKEN").expect("Expected $DISCORD_TOKEN in the environment");
    let darksky_token =
        env::var("FORECAST_TOKEN").expect("Expected $FORECAST_TOKEN in the environment");
    let google_maps_token =
        env::var("GOOGLE_MAPS_TOKEN").expect("Expected $GOOGLE_MAPS_TOKEN in the environment");

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let handler = Handler {
        darksky_token: darksky_token,
        google_maps_token: google_maps_token,
    };
    let mut client = Client::new(&discord_token, handler).expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
