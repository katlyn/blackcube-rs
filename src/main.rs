mod auth;
mod database;
mod handlers;
mod imgur;
mod responses;
mod structs;

use anyhow::Context as AnyhowContext;
use database::connect_database;
use handlers::{
    commands::handle_commands, components::handle_component_interaction,
    requests::handle_user_request,
};
use responses::edit_request;
use structs::{Collections, Config, PendingRequestMidStore, PendingRequestUidStore};

use std::{collections::HashMap, fs};

use reqwest::Client;
use serenity::{
    all::{MessageId, UserId},
    async_trait,
    client::{Context, EventHandler},
    model::{
        application::Interaction,
        channel::Message,
        prelude::Ready,
        prelude::{ChannelId, GuildId},
    },
    prelude::GatewayIntents,
};

use crate::structs::HttpClient;
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let data = ctx.data.read().await;
        let config = data
            .get::<Config>()
            .expect("Could not get config from data");

        if msg.channel_id == config.server.request_channel_id {
            drop(data);
            tokio::spawn(handle_user_request(ctx, msg));
        } else if msg.channel_id == config.server.command_channel_id {
            drop(data);
            tokio::spawn(handle_commands(ctx, msg));
        }
    }

    async fn message_delete(
        &self,
        ctx: Context,
        _channel_id: ChannelId,
        deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        tokio::spawn(async move {
            let mut data = ctx.data.write().await;

            let pending_request_mid_store = data
                .get_mut::<PendingRequestMidStore>()
                .context("Could not get pending request store")
                .expect("Could not get pending request mid store");

            let message_id = pending_request_mid_store.remove(&deleted_message_id);

            let config = data
                .get::<Config>()
                .expect("Could not get config from data");

            match message_id {
                Some(message_id) => {
                    let existing_request = config
                        .server
                        .log_channel_id
                        .message(&ctx.http, message_id)
                        .await;

                    match existing_request {
                        Ok(mut existing_request) => {
                            edit_request(
                                &ctx,
                                &mut existing_request,
                                "Request Cancelled",
                                None,
                                None,
                                false,
                            )
                            .await
                            .context("Could not edit request message"); // log here
                        }
                        Err(_) => {}
                    }
                }
                None => {}
            }
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Component(mut component_interaction) => {
                tokio::spawn(async move {
                    let result =
                        handle_component_interaction(ctx.clone(), component_interaction.clone())
                            .await;
                    if result.is_err() {
                        edit_request(
                            &ctx,
                            &mut component_interaction.message,
                            "Failed",
                            None,
                            None,
                            false,
                        )
                        .await; // log here
                    }
                });
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
    let config: Config = toml::from_str(
        &fs::read_to_string("/etc/oxide/oxide.toml")
            .expect("Could not read configuration file, make sure the config is located at /etc/oxide/oxide.toml")
    ).expect("could not read config");

    let collections: Collections =
        connect_database(&config).expect("Could not connect to database");

    let http_client: Client = Client::new();

    let pending_request_uid_store: HashMap<UserId, MessageId> = HashMap::new();
    let pending_request_mid_store: HashMap<MessageId, MessageId> = HashMap::new();

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let mut client = serenity::Client::builder(&config.bot.discord_token, intents)
        .application_id(config.bot.application_id.into())
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    let mut data = client.data.write().await;
    data.insert::<Config>(config);
    data.insert::<Collections>(collections);
    data.insert::<HttpClient>(HttpClient {
        client: http_client,
    });
    data.insert::<PendingRequestUidStore>(pending_request_uid_store);
    data.insert::<PendingRequestMidStore>(pending_request_mid_store);

    drop(data);

    client.start().await.expect("Error starting client");
}
