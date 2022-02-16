use serenity::{client::Context, model::interactions::message_component};

use crate::{
    database,
    responses::{edit_request, send_interaction_reply},
    structs::Usrbg,
    HasAuth, COLLECTIONS,
};

pub fn handle_component_interaction(
    ctx: Context,
    component_interaction: message_component::MessageComponentInteraction,
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
                let entry = Usrbg {
                    uid: uid.to_string(),
                    // .id - replace later after fixing compiler to take just imgur ids
                    img: image_url.clone(),
                };

                database::upsert(&*COLLECTIONS, &uid, entry)
                    .expect("Error upserting user into database");
                tokio::spawn(edit_request(
                    ctx,
                    component_interaction,
                    uid,
                    image_url,
                    "Request Approved",
                ));
            } else {
                tokio::spawn(send_interaction_reply(ctx, component_interaction));
            }
        }
        "Deny" => {
            if has_auth {
                tokio::spawn(edit_request(
                    ctx,
                    component_interaction,
                    uid,
                    image_url,
                    "Request Denied",
                ));
            } else {
                tokio::spawn(send_interaction_reply(ctx, component_interaction));
            }
        }
        "Cancel" => {
            if component_interaction.user.id.0 == uid.trim().parse::<u64>().unwrap() {
                tokio::spawn(edit_request(
                    ctx,
                    component_interaction,
                    uid,
                    image_url,
                    "Request Cancelled",
                ));
            }
        }
        &_ => {}
    }
}
