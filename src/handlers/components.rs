use anyhow::bail;
use anyhow::Context as AnyhowContext;

use serenity::{
    builder::CreateInteractionResponse, client::Context, model::application::ComponentInteraction,
};

use crate::responses::delete_user_request;
use crate::structs::Collections;
use crate::{
    auth::HasAuth,
    database,
    imgur::upload_image_to_imgur,
    responses::{edit_request, send_ephemeral_interaction_reply},
    structs::Usrbg,
};

pub async fn handle_component_interaction(
    ctx: Context,
    mut component_interaction: ComponentInteraction,
) -> anyhow::Result<()> {
    let has_auth = component_interaction
        .member
        .as_ref()
        .context("Could not retrieve user from interaction")?
        .has_auth(&ctx)
        .await?;

    let embed = component_interaction
        .message
        .embeds
        .first()
        .context("Could not get first embed")?
        .clone();

    let embed_link = embed.url.clone();

    let image_url = embed
        .thumbnail
        .clone()
        .context("Error parsing image url")?
        .url;

    let mut uid: Option<String> = None;

    for field in &embed.fields {
        match field.name.as_str() {
            "UID" => {
                uid = Some(field.value.clone());
                break;
            }
            _ => {}
        }
    }

    let uid: String = uid.context("Could not parse uid from embed")?;

    match component_interaction.data.custom_id.as_str() {
        "Approve" => {
            if has_auth {
                component_interaction
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await
                    .context("Could not acknowledge component interaction")?;

                edit_request(
                    &ctx,
                    &mut component_interaction.message,
                    "Uploading...",
                    Some(&image_url),
                    embed_link.as_deref(),
                    false,
                )
                .await
                .context("Could not update message to show loading state")?;

                let imgur_response = upload_image_to_imgur(&ctx, image_url.clone())
                    .await
                    .context("Could not upload image to Imgur")?;

                let entry = Usrbg {
                    uid: uid.clone(),
                    // .id - replace later after fixing compiler to take just imgur ids | HELP ME
                    img: imgur_response.data.link.clone(),
                };

                let data = ctx.data.read().await;
                let collections = data
                    .get::<Collections>()
                    .context("Could not get collections")?;

                database::upsert(&collections.usrbg, &uid, entry)
                    .context("Could not upsert into database")?;

                edit_request(
                    &ctx,
                    &mut component_interaction.message,
                    "Request Approved",
                    Some(&imgur_response.data.link),
                    None,
                    false,
                )
                .await
                .context("could not edit request message")?;

                drop(data);

                delete_user_request(&ctx, &embed)
                    .await
                    .context("Could not delete original request")?;
            } else {
                send_ephemeral_interaction_reply(
                    &ctx,
                    component_interaction.clone(),
                    "You must wait for a moderator to approve/deny this background",
                )
                .await
                .context("Could not notify user of lack of auth")?;
            }
        }
        "Deny" => {
            if has_auth {
                component_interaction
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await
                    .context("Could not acknowledge component interaction")?;

                edit_request(
                    &ctx,
                    &mut component_interaction.message,
                    "Request Denied",
                    None,
                    None,
                    false,
                )
                .await
                .context("Could not edit request message")?;
                delete_user_request(&ctx, &embed)
                    .await
                    .context("Could not delete original request")?;
            } else {
                send_ephemeral_interaction_reply(
                    &ctx,
                    component_interaction.clone(),
                    "You must wait for a moderator to approve/deny this background",
                )
                .await
                .context("Could not tell user to wait for moderator approval")?;
            }
        }
        "Cancel" => {
            if component_interaction.user.id.get() == uid.trim().parse::<u64>().unwrap() {
                component_interaction
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await
                    .context("Could not acknowledge component interaction")?;

                edit_request(
                    &ctx,
                    &mut component_interaction.message,
                    "Request Cancelled",
                    None,
                    None,
                    false,
                )
                .await
                .context("Could not edit request message")?;

                delete_user_request(&ctx, &embed)
                    .await
                    .context("Could not delete original request")?;
            } else {
                send_ephemeral_interaction_reply(
                    &ctx,
                    component_interaction.clone(),
                    "You cannot cancel someone else's background request",
                )
                .await
                .context("Could not tell user they cannot cancel someone else's background")?;
            }
        }
        &_ => {
            bail!("Invalid component ID");
        }
    }
    Ok(())
}
