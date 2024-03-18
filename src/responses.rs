use anyhow::Context as AnyhowContext;
use serenity::{
    all::{ButtonStyle, Embed, InteractionResponseFlags, MessageFlags, MessageId, UserId},
    builder::{
        CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage,
        EditMessage,
    },
    client::Context,
    model::{application::ComponentInteraction, channel::Message},
};
use url::Url;

use crate::structs::{Config, PendingRequestMidStore, PendingRequestUidStore};

pub async fn edit_request(
    ctx: &Context,
    msg: &mut Message,
    message: &str,
    thumbnail: Option<&str>,
    link: Option<&str>,
    keep_components: bool,
) -> anyhow::Result<()> {
    let embed = &msg.embeds[0];
    let fields: Vec<(_, _, bool)> = embed
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.value.clone(), field.inline))
        .collect();

    let mut components = vec![];

    if keep_components {
        components = vec![CreateActionRow::Buttons(vec![
            CreateButton::new("Approve")
                .style(ButtonStyle::Success)
                .label("Approve"),
            CreateButton::new("Deny")
                .style(ButtonStyle::Danger)
                .label("Deny"),
            CreateButton::new("Cancel")
                .style(ButtonStyle::Secondary)
                .label("Cancel"),
        ])];
    }

    let mut embed_builder = CreateEmbed::new().title(message).fields(fields);

    match thumbnail {
        Some(thumbnail) => {
            embed_builder = embed_builder.thumbnail(thumbnail);
        }
        None => {}
    }

    match link {
        Some(link) => {
            embed_builder = embed_builder.url(link);
        }
        None => {}
    }

    msg.edit(
        &ctx.http,
        EditMessage::new()
            .components(components)
            .embed(embed_builder),
    )
    .await?;
    Ok(())
}

pub async fn send_ephemeral_interaction_reply(
    ctx: &Context,
    component_interaction: ComponentInteraction,
    message: &str,
) -> anyhow::Result<()> {
    component_interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(message)
                    .flags(InteractionResponseFlags::EPHEMERAL),
            ),
        )
        .await
        .context("could not create ephemeral response")?;
    Ok(())
}

pub async fn send_ephemeral_interaction_followup_reply(
    ctx: &Context,
    component_interaction: ComponentInteraction,
    message: &str,
) -> anyhow::Result<()> {
    component_interaction
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(message)
                .flags(MessageFlags::EPHEMERAL),
        )
        .await
        .context("could not create ephemeral response")?;
    Ok(())
}

pub async fn send_command_reply(
    msg: Message,
    ctx: Context,
    response_text: &str,
) -> anyhow::Result<()> {
    msg.reply(&ctx.http, response_text)
        .await
        .context("could not reply to message")?;
    Ok(())
}

pub async fn create_request_log_message(ctx: &Context, msg: &Message) -> anyhow::Result<MessageId> {
    let image_url = &msg
        .attachments
        .first()
        .context("Could not get attachment from message")?
        .url;

    let data = ctx.data.read().await;
    let config = data.get::<Config>().context("Could not get config")?;

    let created_message = config
        .server
        .log_channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new("Approve")
                        .style(ButtonStyle::Success)
                        .label("Approve"),
                    CreateButton::new("Deny")
                        .style(ButtonStyle::Danger)
                        .label("Deny"),
                    CreateButton::new("Cancel")
                        .style(ButtonStyle::Secondary)
                        .label("Cancel"),
                ])])
                .embed(
                    CreateEmbed::new()
                        .title("Request Pending")
                        .field("User", msg.author.name.clone(), true)
                        .field("UID", msg.author.id.to_string(), true)
                        .thumbnail(image_url)
                        .url(msg.link()),
                ),
        )
        .await
        .context("could not create request log message")?;
    Ok(created_message.id)
}

pub async fn delete_user_request(ctx: &Context, embed: &Embed) -> anyhow::Result<()> {
    let embed_link = embed.url.clone().context("could not get embed link")?;

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

    let mut data = ctx.data.write().await;
    let pending_request_store = data
        .get_mut::<PendingRequestUidStore>()
        .context("Could not get pending request store")?;

    let search_uid = UserId::new(uid.parse()?);
    pending_request_store.remove(&search_uid);

    let pending_request_mid_store = data
        .get_mut::<PendingRequestMidStore>()
        .context("Could not get pending request store")?;

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

    drop(data);

    let segments = embed_link
        .path_segments()
        .context("could not get segments from embed link")?;
    let message_id = segments
        .into_iter()
        .last()
        .context("Could not get message ID from link")?;

    let message_id: u64 = message_id.parse().context("Error parsing message id")?;

    let data = ctx.data.read().await;
    let config = data.get::<Config>().context("Could not get config")?;

    config
        .server
        .request_channel_id
        .delete_message(&ctx.http, message_id)
        .await
        .context("could not delete original message")?;

    drop(data);
    Ok(())
}
