use anyhow::bail;
use anyhow::Context as AnyhowContext;

use serenity::all::MessageId;
use serenity::all::UserId;
use serenity::{
    builder::CreateInteractionResponse, client::Context, model::application::ComponentInteraction,
};
use url::Url;

use crate::responses::delete_user_request;
use crate::structs::Collections;
use crate::structs::PendingRequestMidStore;
use crate::structs::PendingRequestUidStore;
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
        .context("Could not get first embed")?;

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

    component_interaction
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
        .context("Could not acknowledge component interaction")?;

    match component_interaction.data.custom_id.as_str() {
        "Approve" => {
            if has_auth {
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
            } else {
                send_ephemeral_interaction_reply(
                    ctx.clone(),
                    component_interaction.clone(),
                    "You must wait for a moderator to approve/deny this banner",
                )
                .await
                .context("Could not notify user of lack of auth")?;
            }
        }
        "Deny" => {
            if has_auth {
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
            } else {
                tokio::spawn(send_ephemeral_interaction_reply(
                    ctx.clone(),
                    component_interaction.clone(),
                    "You must wait for a moderator to approve/deny this banner",
                ));
            }
        }
        "Cancel" => {
            if component_interaction.user.id.get() == uid.trim().parse::<u64>().unwrap() {
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
            }
        }
        &_ => {
            bail!("Invalid component ID");
        }
    }

    let mut data = ctx.data.write().await;
    let pending_request_store = data
        .get_mut::<PendingRequestUidStore>()
        .context("Could not get pending request store")?;

    let search_uid = UserId::new(uid.parse()?);
    pending_request_store.remove(&search_uid);

    let pending_request_mid_store = data
        .get_mut::<PendingRequestMidStore>()
        .context("Could not get pending request store")?;

    match &embed_link {
        Some(embed_link) => {
            let embed_link = Url::parse(&embed_link).context("Could not parse embed link")?;

            let segments = embed_link
                .path_segments()
                .context("could not get segments from embed link")?;
            let message_id = segments
                .into_iter()
                .last()
                .context("Could not get message ID from link")?;

            let message_id: u64 = message_id.parse().context("Error parsing message id")?;

            pending_request_mid_store.remove(&MessageId::new(message_id));
        }
        None => {}
    }

    drop(data);

    delete_user_request(&ctx, &embed_link.context("could not get embed link")?)
    .await
    .context("Could not delete original request")?;

    Ok(())
}
