use std::collections::HashSet;
use std::env;

use mongodb::{options::ClientOptions, Client};

use serde::{Deserialize, Serialize};

// Struct def for database entries
#[derive(Debug, Serialize, Deserialize)]
struct Usrbg {
    uid: String,
    img: String,
}

use serenity::{
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
        id::{ChannelId, RoleId},
        interactions::*,
    },
    prelude::*,
};

struct Handler;

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
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let request_channel_id: u64 = env::var("REQUEST_CHANNEL_ID")
            .unwrap()
            .trim()
            .parse()
            .expect("Expected a request channel id in the environment");

        if msg.channel_id == request_channel_id {
            // Check for attachments
            if msg.attachments.len() > 0 {
                for attachment in msg.attachments {
                    // Attempt to get content type from Some(T)
                    match attachment.content_type {
                        // Found valid attachment type
                        Some(content) => {
                            // Match for valid image type, removing "image/" prefix
                            if IMAGE_TYPES.contains(&content[6..]) {

                                // Parse env var
                                let log_channel_id: u64 = env::var("LOG_CHANNEL_ID")
                                    .unwrap()
                                    .trim()
                                    .parse()
                                    .expect("Expected a log channel id in the environment");
                                
                                // Convert to channel id type for later use
                                let log_channel_id = ChannelId(log_channel_id);

                                // Send log message in specified channel, attach embed and button components within the action row
                                if let Err(why) = log_channel_id
                                    .send_message(&ctx.http, |m| {
                                        m.components(|c| {
                                            c.create_action_row(|r| {
                                                r.create_button(|b| {
                                                    b.style(
                                                        message_component::ButtonStyle::Success,
                                                    );
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
                                    println!("Error sending message: {:?}", why);
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
        }
    }

    // Watch for button interactions
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {

        // Parse env var
        let check_role = RoleId {
            0: env::var("AUTH_ROLE_ID")
                .unwrap()
                .trim()
                .parse()
                .expect("Expected an authorization role id in the environment"),
        };

        // Unwrap interaction into message component
        let interaction = interaction
            .message_component()
            .expect("Error unwrapping interaction");


        // Check authentication role
        if interaction
            .member
            .as_ref()
            .expect("Error parsing member")
            .roles
            .contains(&check_role)
        {
            // Parse env var
            let imgur_id = env::var("IMGUR_ID").expect("Expected an imgur id in the environment");

            // Get button type (Accept/Deny)
            let id = interaction.data.custom_id;

            // Create mutable message for responses
            let mut message = interaction.message;

            // Clone the embed to avoid shared reference issues while reading values from before the editing
            let embeds = message.clone().embeds;

            // Extract the banner image url from the embed
            let image_url = &embeds[0]
                .thumbnail
                .as_ref()
                .expect("Error parsing image url")
                .url;

            // Extract the user id from the embed
            let uid = &embeds[0]
                .description
                .as_ref()
                .expect("Error parsing user id");

            // Choose action based on button type
            match id.as_str() {
                "Approve" => {

                    // Parse env var
                    let mongo_url = env::var("MONGO_URL").expect("Expected a link to a mongodb database in the environment");

                    let client_options = ClientOptions::parse(mongo_url)
                        .await
                        .expect("Error creating db connection");

                    let mongo_client =
                        Client::with_options(client_options).expect("Error creating db client");

                    let db = mongo_client.database("usrbg-staging");

                    let usrbg_collection = db.collection::<Usrbg>("usrbg-staging");

                    let client = reqwest::Client::new();

                    // Upload image to imgur
                    let res = client
                        .post("https://api.imgur.com/3/image")
                        .header("Authorization", imgur_id)
                        // Clone to avoid shared reference issues
                        .body(image_url.clone())
                        .send()
                        .await;

                    // Serialize json from string to retrieve image id
                    let json: serde_json::Value =
                        serde_json::from_str(&res.unwrap().text().await.unwrap())
                            .expect("Error parsing json");

                    // Get the banner id from the serialized json, and then convert it back to a string for entry into the database
                    // Also trims the first and last chars from the string which are quotes -- Better fix??
                    let banner_id = remove_quoutes(serde_json::to_string(&json["data"]["id"]).expect("Error converting json to string"));

                    let entry = Usrbg {
                        uid: uid.to_string(),
                        img: banner_id,
                    };

                    usrbg_collection.insert_one(entry, None).await.expect("Error inserting entry");

                    // Update message with approval
                    message
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
                    message
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
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id: u64 = env::var("APPLICATION_ID")
        .unwrap()
        .trim()
        .parse()
        .expect("Expected an application id in the environment");

    let mut client = serenity::Client::builder(&token)
        .application_id(application_id)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

fn remove_quoutes(string: String) -> String {
    let mut string = string.chars();
    string.next();
    string.next_back();
    string.as_str().to_string()
}