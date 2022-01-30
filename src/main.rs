use bson::doc;
use mongodb::sync::Client;
use std::collections::HashSet;
use std::fs;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, id::RoleId, interactions::*},
    prelude::*,
};

struct Handler;

mod structs;
use crate::structs::defs;

#[macro_use]
extern crate lazy_static;

// Define static hashset for image types
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

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.channel_id == CONFIG.server.request_channel_id {
            // Check for attachments
            if msg.attachments.len() > 0 {
                create_request(ctx, msg).await;
            }
        } else if msg.channel_id == CONFIG.server.command_channel_id {
            let mut iter = msg.content.split_whitespace();
            match iter.next() {
                None => {
                    println!("Invalid Command");
                }
                Some(command) => {
                    let uid = iter.next().expect("Error initiating ban");
                    if uid.trim().parse::<u64>().is_ok() {
                        if check_auth(
                            &msg.member.expect("Error").roles,
                            CONFIG.server.auth_role_id,
                        ) {
                            match command {
                                "~rm" => {
                                    COLLECTIONS
                                        .usrbg
                                        .delete_one(doc! { "uid": uid }, None)
                                        .expect("Error removing entry");
                                }
                                "~ban" => {
                                    let entry = defs::Blacklist {
                                        uid: uid.to_owned(),
                                    };
                                    COLLECTIONS
                                        .blacklist
                                        .insert_one(entry, None)
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
                    } else if command == "~rm" {
                        COLLECTIONS
                        .usrbg
                        .delete_one(doc! { "uid": msg.author.id.to_string() }, None)
                        .expect("Error removing entry");
                    }
                }
            }
        }
    }

    // Watch for button interactions
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // Unwrap interaction into message component
        let mut interaction = interaction
            .message_component()
            .expect("Error unwrapping interaction");

        // Check authentication role
        if check_auth(
            &interaction.member.as_ref().expect("Error").roles,
            CONFIG.server.auth_role_id,
        ) {
            // Clone the embed to avoid shared reference issues while reading values from before the editing
            let embed = &interaction.message.embeds[0];

            // Extract the banner image url from the embed
            let image_url = embed
                .thumbnail
                .clone()
                .expect("Error parsing image url")
                .url;
            let uid = embed.description.clone().expect("Error parsing user id");

            // Choose action based on button type
            match interaction.data.custom_id.as_str() {
                "Approve" => {
                    // interaction.defer(&ctx.http);

                    let client = reqwest::Client::new();

                    // Upload image to imgur
                    let res = client
                        .post("https://api.imgur.com/3/image")
                        .header("Authorization", &CONFIG.api.imgur_id)
                        // Clone to avoid shared reference issues
                        .body(image_url.clone())
                        .send()
                        .await;

                    // Serialize json from string to retrieve image id
                    let json: defs::ImgurResponse =
                        serde_json::from_str(&res.unwrap().text().await.unwrap())
                            .expect("Error parsing json");

                    let entry = defs::Usrbg {
                        uid: uid.to_string(),
                        img: json.data.id,
                    };

                    COLLECTIONS
                        .usrbg
                        .insert_one(entry, None)
                        .expect("Error inserting entry");

                    // Update message with approval
                    interaction
                        .message
                        .edit(&ctx, |m| {
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
                }
                "Deny" => {
                    // Update message with denial, nothing else needs to be done here
                    interaction
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
                // Null case -- Need error handling?
                &_ => {}
            }
        } else {
            // Reply ephemeraly stating a lack of authentication role
            interaction
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
    let mut client = serenity::Client::builder(&CONFIG.bot.discord_token)
        .application_id(CONFIG.bot.application_id)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(err) = client.start().await {
        println!("Client error: {:?}", err);
    }
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

async fn create_request(ctx: Context, msg: Message) {
    for attachment in msg.attachments {
        // Attempt to get content type from Some(T)
        match attachment.content_type {
            // Found valid attachment type
            Some(content) => {
                // Match for valid image type, removing "image/" prefix
                if IMAGE_TYPES.contains(&content[6..]) {
                    // Send log message in specified channel, attach embed and button components within the action row
                    if let Err(err) = CONFIG
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

                                    r
                                })
                            });

                            m.embed(|e| {
                                e.title("Request Pending");
                                e.description(msg.author.id.0);
                                e.thumbnail(attachment.url);

                                e
                            });

                            m
                        })
                        .await
                    {
                        println!("Error sending message: {:?}", err);
                    }
                }
            }
            // No valid attachment type
            None => {
                println!("No Attachment");
            }
        }
    }
}

fn check_auth(roles: &Vec<RoleId>, auth_id: RoleId) -> bool {
    roles.contains(&auth_id)
}
