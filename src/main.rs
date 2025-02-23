mod auth;
mod database;
mod handlers;
mod imgur;
mod responses;
mod s3bucket;
mod structs;

use anyhow::Context as AnyhowContext;
use database::connect_database;
use handlers::{
    commands::handle_commands, components::handle_component_interaction,
    requests::handle_user_request,
};
use responses::edit_request;
use s3bucket::connect_bucket;
use structs::{Collections, Config, PendingRequestMidStore, PendingRequestUidStore, S3Bucket};

use std::{collections::HashMap, fs};

use reqwest::Client;
use serenity::{
    all::{MessageId, UserId},
    async_trait,
    client::{Context, EventHandler},
    model::{
        application::Interaction,
        channel::Message,
        prelude::{ChannelId, GuildId, Ready},
    },
    prelude::GatewayIntents,
};

use crate::{responses::send_ephemeral_interaction_followup_reply, structs::HttpClient};

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
                            let result = edit_request(
                                &ctx,
                                &mut existing_request,
                                "Request Cancelled",
                                None,
                                None,
                                false,
                            )
                            .await
                            .context("Could not edit request message");
                            if result.is_err() {
                                println!("{:?}", result);
                            }
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
                        println!("{:?}", result);

                        let embed = component_interaction.message.embeds.first();

                        match embed {
                            Some(embed) => {
                                let embed = embed.clone();

                                let thumbnail;

                                match &embed.thumbnail {
                                    Some(embed_thumbnail) => {
                                        thumbnail = Some(embed_thumbnail.url.as_str());
                                    }
                                    None => {
                                        thumbnail = None;
                                    }
                                }

                                let url;

                                match &embed.url {
                                    Some(embed_url) => {
                                        url = Some(embed_url.as_str());
                                    }
                                    None => {
                                        url = None;
                                    }
                                }

                                let result = edit_request(
                                    &ctx,
                                    &mut component_interaction.message,
                                    "Request Pending",
                                    thumbnail,
                                    url,
                                    true,
                                )
                                .await;
                                if result.is_err() {
                                    println!("{:?}", result);
                                }
                            }
                            None => {}
                        }

                        let result = send_ephemeral_interaction_followup_reply(
                            &ctx,
                            component_interaction,
                            "Failed to accept request",
                        )
                        .await;
                        match result {
                            Ok(()) => {}
                            Err(err) => {
                                println!("{}", err);
                            }
                        }
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
    let config_file_location;

    match std::env::consts::OS {
        "linux" => {
            config_file_location = "/etc/blackcube-rs/blackcube-rs.toml";
        }
        "windows" => {
            config_file_location = "C:\\ProgramData\\blackcube-rs\\blackcube-rs.toml";
        }
        _ => {
            unreachable!();
        }
    }

    let config: Config = toml::from_str(
        &fs::read_to_string(config_file_location)
            .expect("Could not read configuration file, make sure the config is located at /etc/blackcube-rs/blackcube-rs.toml or C:\\ProgramData\\blackcube-rs\\blackcube-rs.toml")
    ).expect("could not read config");

    let bucket = connect_bucket(&config)
        .await
        .expect("Could not initialize storage bucket connection");

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
    data.insert::<S3Bucket>(bucket);
    data.insert::<Collections>(collections);
    data.insert::<HttpClient>(HttpClient {
        client: http_client,
    });
    data.insert::<PendingRequestUidStore>(pending_request_uid_store);
    data.insert::<PendingRequestMidStore>(pending_request_mid_store);

    drop(data);

    client.start().await.expect("Error starting client");
}
