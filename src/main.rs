#[macro_use] extern crate lazy_static;
extern crate serenity;
extern crate regex;

use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use regex::Regex;
use std::env;

mod puppywhy;

struct Handler;

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
            static ref WOOFRE: Regex =
                Regex::new(r"pupper pupper ((oua+f+)|(w(a|o|0|u)+r*f)|(aw+(o|0)+)|(b(a|o)+rk) ?)+!*").unwrap();
        }
        let ref content = &msg.content;
        let lower = content.to_lowercase();
        if WOOFRE.is_match(&lower) {
            if let Err(why) = msg.channel_id.say(content) {
                println!("Error sending message: {:?}", why);
            }
        } else if lower == "pupper pupper puppy why" {
            if let Err(why) = msg.channel_id.say(puppywhy::why()) {
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
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::new(&token, Handler).expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
