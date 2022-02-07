use bson::doc;
use mongodb::sync::Client;
use std::collections::HashSet;
use std::fs;

pub use mongodb::options::FindOneAndUpdateOptions;

use serenity::{
    async_trait,
    model::{
        channel::{Attachment, Message},
        gateway::Ready,
        guild::{Member, PartialMember},
        interactions::*,
    },
    prelude::*,
};

struct Handler;

mod structs;
use crate::structs::defs;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref IMAGE_TYPES: HashSet<&'static str> = {
        let mut m = HashSet::new();
        m.insert("png");
        m.insert("jpeg");
        m.insert("gif");
        m.insert("svg");
        m
    };
    static ref CONFIG: defs::Config = toml::from_str(
        &fs::read_to_string("oxide.toml").expect("Something went wrong reading the file")
    )
    .expect("Error parsing toml file");
    static ref COLLECTIONS: structs::defs::Collections = connect_database();
}

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

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.channel_id == CONFIG.server.request_channel_id {
            handle_request(ctx, msg).await;
        } else if msg.channel_id == CONFIG.server.command_channel_id {
            validate_commands(msg);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::MessageComponent(component_interaction) => {
                handle_component_interaction(ctx, component_interaction).await;
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

fn connect_database() -> structs::defs::Collections {
    let client = Client::with_uri_str(&CONFIG.database.url).expect("Error connecting to database");
    let db = client.database(&CONFIG.database.name);

    let usrbg_collection = db.collection::<structs::defs::Usrbg>(&CONFIG.database.usrbg_collection);
    let blacklist_collection =
        db.collection::<structs::defs::Blacklist>(&CONFIG.database.blacklist_collection);

    let collections = structs::defs::Collections {
        usrbg: usrbg_collection,
        blacklist: blacklist_collection,
    };
    collections
}

async fn handle_request(ctx: Context, msg: Message) {
    for attachment in &msg.attachments {
        let valid_image = IMAGE_TYPES.contains(&attachment.content_type.as_ref().unwrap()[6..]);

        if valid_image {
            create_request(&ctx, &msg, attachment).await;
        }
    }
}

fn validate_commands(msg: Message) {
    let mut command_arguments = msg.content.split_whitespace();
    let command = command_arguments.next();
    match command {
        None => {
            println!("No Command Found");
        }
        Some(command) => {
            let uid = command_arguments.next();
            match uid {
                None => {
                    if command == "~remove" {
                        COLLECTIONS
                            .usrbg
                            .delete_one(doc! { "uid": msg.author.id.to_string() }, None)
                            .expect("Error removing entry");
                    }
                }
                Some(uid) => {
                    parse_commands(&msg, command, uid);
                }
            }
        }
    }
}

async fn handle_component_interaction(
    ctx: Context,
    mut component_interaction: message_component::MessageComponentInteraction,
) {
    let has_auth = component_interaction.member.as_ref().unwrap().check_auth();

    let embed = &component_interaction.message.embeds[0];

    let image_url = embed
        .thumbnail
        .clone()
        .expect("Error parsing image url")
        .url;
    let uid = embed.description.clone().expect("Error parsing user id");

    match component_interaction.data.custom_id.as_str() {
        "Approve" => {
            if has_auth {
                let client = reqwest::Client::new();

                let res = client
                    .post("https://api.imgur.com/3/image")
                    .header("Authorization", &CONFIG.api.imgur_id)
                    .body(image_url.clone())
                    .send()
                    .await;

                let json: defs::ImgurResponse =
                    serde_json::from_str(&res.unwrap().text().await.unwrap())
                        .expect("Error parsing json");

                let entry = defs::Usrbg {
                    uid: uid.to_string(),
                    img: json.data.id,
                };

                let options = FindOneAndUpdateOptions::builder()
                    .upsert(Some(true))
                    .build();
                COLLECTIONS
                    .usrbg
                    .find_one_and_update(
                        doc! { "uid": &uid },
                        doc! { "$set": bson::to_bson(&entry).unwrap() },
                        Some(options),
                    )
                    .expect("Error inserting entry");

                component_interaction
                    .message
                    .edit(ctx, |m| {
                        m.components(|c| c);
                        m.embed(|e| {
                            e.title("Request Approved");
                            e.description(uid);
                            e.thumbnail(image_url.clone());
                            e
                        });
                        m
                    })
                    .await
                    .expect("Error editing message");
            } else {
                send_no_auth(&ctx, &component_interaction).await;
            }
        }
        "Deny" => {
            if has_auth {
                reply_deny(&ctx, &mut component_interaction, uid, image_url).await;
            } else {
                send_no_auth(&ctx, &component_interaction).await;
            }
        }
        "Cancel" => {
            if component_interaction.user.id.0 == uid.trim().parse::<u64>().unwrap() {
                reply_deny(&ctx, &mut component_interaction, uid, image_url).await;
            }
        }
        &_ => {}
    }
}

async fn create_request(ctx: &Context, msg: &Message, attachment: &Attachment) {
    CONFIG
        .server
        .log_channel_id
        .send_message(&ctx.http, |m| {
            m.components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.style(message_component::ButtonStyle::Success);
                        b.custom_id("Approve");
                        b.label("Approve");

                        b
                    });
                    r.create_button(|b| {
                        b.style(message_component::ButtonStyle::Danger);
                        b.custom_id("Deny");
                        b.label("Deny");

                        b
                    });
                    r.create_button(|b| {
                        b.style(message_component::ButtonStyle::Secondary);
                        b.custom_id("Cancel");
                        b.label("Cancel");

                        b
                    });

                    r
                })
            });

            m.embed(|e| {
                e.title("Request Pending");
                e.description(msg.author.id.0);
                e.thumbnail(&attachment.url);

                e
            });

            m
        })
        .await
        .unwrap();
}

async fn send_no_auth(
    ctx: &Context,
    component_interaction: &message_component::MessageComponentInteraction,
) {
    component_interaction
        .create_interaction_response(&ctx, |m| {
            m.interaction_response_data(|d| {
                d.content("You must wait for a moderator to approve/deny this banner");
                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                d
            });
            m
        })
        .await
        .expect("Error sending response");
}

async fn reply_deny(
    ctx: &Context,
    component_interaction: &mut message_component::MessageComponentInteraction,
    uid: String,
    image_url: String,
) {
    component_interaction
        .message
        .edit(&ctx, |m| {
            m.components(|c| c);
            m.embed(|e| {
                e.title("Request Denied");
                e.description(uid);
                e.thumbnail(image_url.clone());
                e
            });
            m
        })
        .await
        .expect("Error editing message");
}

fn parse_commands(msg: &Message, command: &str, uid: &str) {
    let valid_id = uid.trim().parse::<u64>().is_ok();
    let has_auth = msg.member.as_ref().unwrap().check_auth();

    if valid_id && has_auth {
        match command {
            "~remove" => {
                COLLECTIONS
                    .usrbg
                    .delete_one(doc! { "uid": uid }, None)
                    .expect("Error removing entry");
            }
            "~ban" => {
                let entry = defs::Blacklist {
                    uid: uid.to_owned(),
                };
                let options = FindOneAndUpdateOptions::builder()
                    .upsert(Some(true))
                    .build();
                COLLECTIONS
                    .blacklist
                    .find_one_and_update(
                        doc! { "uid": uid },
                        bson::to_document(&entry).unwrap(),
                        Some(options),
                    )
                    .expect("Error inserting entry");
            }
            "~unban" => {
                COLLECTIONS
                    .blacklist
                    .delete_one(doc! { "uid": uid }, None)
                    .expect("Error unbanning user");
            }
            &_ => {}
        }
    }
}
