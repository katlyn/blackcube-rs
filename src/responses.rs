use serenity::{
    client::Context,
    model::{
        channel::Message,
        interactions::{message_component, InteractionApplicationCommandCallbackDataFlags},
    },
};

use crate::CONFIG;

pub async fn edit_request(
    ctx: Context,
    mut component_interaction: message_component::MessageComponentInteraction,
    uid: String,
    image_url: String,
    message: &str,
) {
    component_interaction
        .message
        .edit(&ctx, |m| {
            m.components(|c| c);
            m.embed(|e| {
                e.title(message);
                e.description(uid);
                e.thumbnail(image_url);
                e
            });
            m
        })
        .await
        .expect("Error editing message");
}

pub async fn send_interaction_reply(
    ctx: Context,
    component_interaction: message_component::MessageComponentInteraction,
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

pub async fn send_command_reply(msg: Message, ctx: Context, response_text: &str) {
    msg.reply(&ctx, response_text).await.unwrap();
}

pub async fn create_request(ctx: &Context, msg: &Message, image_url: String) {
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
                e.thumbnail(image_url);

                e
            });

            m
        })
        .await
        .unwrap();
}
