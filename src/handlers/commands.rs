use bson::doc;
use serenity::{client::Context, model::channel::Message};

use crate::{database, responses::send_command_reply, structs::Blacklist, HasAuth, COLLECTIONS};

pub fn handle_commands(msg: Message, ctx: Context) {
    let message_content = msg.content.clone();
    let mut message_words = message_content.split_whitespace();
    let command = message_words.next();
    match command {
        Some(command) => {
            let command_argument = message_words.next();

            handle_command_auth_level(msg, ctx, command, command_argument);
        }
        None => {}
    }
}

pub fn handle_command_auth_level(
    msg: Message,
    ctx: Context,
    command: &str,
    command_argument: Option<&str>,
) {
    let has_auth = msg.member.as_ref().unwrap().check_auth();
    if has_auth && command_argument.is_some() {
        handle_admin_commands(msg, ctx, command, command_argument);
    } else {
        handle_user_commands(msg, ctx, command);
    }
}

pub fn handle_admin_commands(
    msg: Message,
    ctx: Context,
    command: &str,
    command_argument: Option<&str>,
) {
    let user_id = match command_argument {
        Some(user_id) => user_id,
        None => "",
    };

    let valid_user_id = user_id.trim().parse::<u64>().is_ok();
    if valid_user_id {
        match command {
            "~remove" => {
                database::delete(&*COLLECTIONS, user_id.to_string())
                    .expect("Error removing self from database");

                tokio::spawn(send_command_reply(msg, ctx, "usrbg removed"));
            }
            "~ban" => {
                let entry = Blacklist {
                    uid: user_id.to_owned(),
                };
                database::upsert(&*COLLECTIONS, &user_id.to_string(), entry)
                    .expect("Error upserting user into database");
                tokio::spawn(send_command_reply(msg, ctx, "banned user"));
            }
            "~unban" => {
                COLLECTIONS
                    .blacklist
                    .delete_one(doc! { "uid": user_id }, None)
                    .expect("Error unbanning user");
                tokio::spawn(send_command_reply(msg, ctx, "unbanned user"));
            }
            &_ => {}
        }
    }
}

pub fn handle_user_commands(msg: Message, ctx: Context, command: &str) {
    match command {
        "~remove" => {
            database::delete(&*COLLECTIONS, msg.author.id.to_string())
                .expect("Error removing self from database");
            tokio::spawn(send_command_reply(msg, ctx, "usrbg removed"));
        }
        &_ => {}
    }
}
