mod database;
mod handlers;
mod imgur;
mod responses;
mod structs;

#[macro_use]
extern crate lazy_static;

use reqwest::Client;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        channel::Message,
        guild::{Member, PartialMember},
        interactions::Interaction,
        prelude::Ready,
    },
};
use std::fs;

use database::connect_database;
use handlers::{handle_commands, handle_component_interaction, handle_request};
use structs::{Blacklist, Collections, Config, Usrbg};

struct Handler;

trait HasAuth {
    fn check_auth(&self) -> bool;
}

impl HasAuth for PartialMember {
    fn check_auth(&self) -> bool {
        self.roles.contains(&CONFIG.server.auth_role_id)
    }
}
impl HasAuth for Member {
    fn check_auth(&self) -> bool {
        self.roles.contains(&CONFIG.server.auth_role_id)
    }
}

pub trait TypeInfo {
    fn type_of(&self) -> &'static str;
}

impl TypeInfo for Usrbg {
    fn type_of(&self) -> &'static str {
        "Usrbg"
    }
}

impl TypeInfo for Blacklist {
    fn type_of(&self) -> &'static str {
        "Blacklist"
    }
}

lazy_static! {
    static ref CONFIG: Config = toml::from_str(
        &fs::read_to_string("oxide.toml").expect("Something went wrong reading the file")
    )
    .expect("Error parsing toml file");
    static ref COLLECTIONS: Collections = connect_database(&*CONFIG);
    static ref HTTP_CLIENT: Client = Client::new();
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.channel_id == CONFIG.server.request_channel_id {
            handle_request(ctx, msg).await;
        } else if msg.channel_id == CONFIG.server.command_channel_id {
            handle_commands(msg, ctx);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::MessageComponent(component_interaction) => {
                handle_component_interaction(ctx, component_interaction);
            }
            _ => {}
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let mut client = serenity::Client::builder(&CONFIG.bot.discord_token)
        .application_id(CONFIG.bot.application_id)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    client.start().await.expect("Error starting client");
}
