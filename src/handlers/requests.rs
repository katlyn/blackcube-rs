use anyhow::{bail, Context as AnyhowContext};
use serenity::{client::Context, model::channel::Message};

use crate::{
    auth::{HasAuth, IsBlacklisted},
    responses::{create_request_log_message, edit_request},
    structs::{Config, PendingRequestMidStore, PendingRequestUidStore},
};

pub async fn handle_user_request(ctx: Context, msg: Message) -> anyhow::Result<()> {
    if msg.author.is_blacklisted(&ctx).await? {
        msg.delete(&ctx.http).await?;
        bail!("User is blacklisted");
    };

    let message_attachment = msg.attachments.first();

    // Check to see if attachment exists

    if message_attachment.is_none() && !msg.author.has_auth(&ctx).await? {
        msg.delete(&ctx.http).await?;
        bail!("No message attachment")
    }

    let message_attachment = message_attachment.context("No message attachment")?; // Better way to do this?

    // Check to make sure image is under size limit

    if message_attachment.size > 10000000 {
        msg.delete(&ctx.http).await?;
        bail!("File size too large");
    }

    // Check for valid image type

    let data = ctx.data.read().await;
    let config = data.get::<Config>().context("Could not get config")?;

    let attachment_content_type = &message_attachment
        .content_type
        .as_ref()
        .context("Could not get content-type")?[6..];

    if !config
        .settings
        .image_types
        .contains(&attachment_content_type.to_string())
        && !msg.author.has_auth(&ctx).await?
    {
        msg.delete(&ctx.http).await?;
        bail!("Invalid file type");
    }

    drop(data);

    // check if user has an existing request, if so, cancel it first

    let mut data = ctx.data.write().await;
    let pending_request_uid_store = data
        .get_mut::<PendingRequestUidStore>()
        .context("Could not get pending request store")?;

    let message_id = pending_request_uid_store.remove(&msg.author.id);

    let pending_request_mid_store = data
        .get_mut::<PendingRequestMidStore>()
        .context("Could not get pending request store")?;

    pending_request_mid_store.remove(&msg.id);

    let config = data.get::<Config>().context("Could not get config")?;

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

    drop(data);

    let created_message_id = create_request_log_message(&ctx, &msg).await?; // Add error handling here (log to channel?)

    // Add new request to local store

    let mut data = ctx.data.write().await;
    let pending_request_store = data
        .get_mut::<PendingRequestUidStore>()
        .context("Could not get pending request store")?;
    pending_request_store.insert(msg.author.id, created_message_id);

    let pending_request_mid_store = data
        .get_mut::<PendingRequestMidStore>()
        .context("Could not get pending request store")?;
    pending_request_mid_store.insert(msg.id, created_message_id);

    Ok(())
}
