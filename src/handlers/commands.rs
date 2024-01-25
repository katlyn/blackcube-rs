use anyhow::Context as AnyhowContext;
use serenity::{client::Context, model::channel::Message};

use crate::{
    auth::HasAuth,
    database,
    responses::send_command_reply,
    structs::{Blacklist, Collections},
};

pub async fn handle_commands(ctx: Context, msg: Message) {
    let message_content = msg.content.clone();
    let mut message_words = message_content.split_whitespace();
    let command = message_words.next();
    match command {
        Some(command) => {
            let command_argument = message_words.next();

            let result = handle_command_auth_level(ctx, msg, command, command_argument).await;
            if result.is_err() {
                println!("{:?}", result);
            }
        }
        None => {}
    }
}

pub async fn handle_command_auth_level(
    ctx: Context,
    msg: Message,
    command: &str,
    command_argument: Option<&str>,
) -> anyhow::Result<()> {
    let has_auth = msg
        .member
        .as_ref()
        .context("could not get auth")?
        .has_auth(&ctx)
        .await?;
    if has_auth && command_argument.is_some() {
        handle_admin_commands(ctx, msg, command, command_argument).await?;
    } else {
        handle_user_commands(ctx, msg, command).await?;
    }
    Ok(())
}

pub async fn handle_admin_commands(
    ctx: Context,
    msg: Message,
    command: &str,
    command_argument: Option<&str>,
) -> anyhow::Result<()> {
    let user_id = match command_argument {
        Some(user_id) => user_id,
        None => "",
    };

    let valid_user_id = user_id.trim().parse::<u64>().is_ok();

    if valid_user_id {
        let data = ctx.data.read().await;
        let collections = data
            .get::<Collections>()
            .context("Could not get collections")?;

        match command {
            "~remove" => {
                let result = database::delete(&collections.usrbg, user_id.to_string());
                drop(data);
                match result {
                    Ok(_) => {
                        send_command_reply(msg, ctx, "usrbg removed").await?;
                    }
                    Err(_) => {
                        send_command_reply(msg, ctx, "failed to remove usrbg").await?;
                    }
                }
            }
            "~ban" => {
                let entry = Blacklist {
                    uid: user_id.to_owned(),
                };
                let result = database::upsert(&collections.blacklist, &user_id.to_string(), entry);
                drop(data);
                match result {
                    Ok(_) => {
                        send_command_reply(msg, ctx, "banned user").await?;
                    }
                    Err(_) => {
                        send_command_reply(msg, ctx, "failed to ban user").await?;
                    }
                }
            }
            "~unban" => {
                let result = database::delete(&collections.blacklist, user_id.to_string());
                drop(data);
                match result {
                    Ok(_) => {
                        send_command_reply(msg, ctx, "unbanned user").await?;
                    }
                    Err(_) => {
                        send_command_reply(msg, ctx, "failed to unban user").await?;
                    }
                }
            }
            &_ => {}
        }
    }
    Ok(())
}

pub async fn handle_user_commands(ctx: Context, msg: Message, command: &str) -> anyhow::Result<()> {
    match command {
        "~remove" => {
            let data = ctx.data.read().await;
            let collections = data
                .get::<Collections>()
                .context("Could not get collections")?;

            let result = database::delete(&collections.usrbg, msg.author.id.to_string());
            drop(data);
            match result {
                Ok(_) => {
                    send_command_reply(msg, ctx, "usrbg removed").await?;
                }
                Err(_) => {
                    send_command_reply(msg, ctx, "failed to remove usrbg").await?;
                }
            }
        }
        &_ => {}
    }
    Ok(())
}
