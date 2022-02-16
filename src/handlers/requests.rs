use bson::doc;
use serenity::{client::Context, model::channel::Message};

use crate::{imgur::upload_image_to_imgur, responses::create_request, COLLECTIONS, CONFIG};

pub async fn handle_request(ctx: Context, msg: Message) {
    let mut has_valid_image_url: Option<String> = None;

    // Allow for links to images - Need to implement image type parsing (security risk)
    // let message_content_has_url: Result<Url, url::ParseError> = Url::parse(&msg.content);
    // match message_content_has_url {
    //     Ok(image_url) => {
    //         has_valid_image_url = Some(String::from(image_url));
    //     }
    //     _ => {}
    // }

    let blacklist_search_query = doc! { "uid": msg.author.id.0.to_string() };
    let blacklist_search_result = COLLECTIONS.blacklist.find_one(blacklist_search_query, None);
    let is_blacklisted = blacklist_search_result.unwrap().is_some();
    if is_blacklisted {
        return;
    };

    let has_valid_attachment = msg.attachments.first();

    match has_valid_attachment {
        Some(attachment) => {
            let attachment_content_type = &attachment.content_type.as_ref().unwrap()[6..];
            let message_has_valid_attachment = CONFIG
                .settings
                .image_types
                .contains(&attachment_content_type.to_string());
            if message_has_valid_attachment {
                has_valid_image_url = Some(attachment.url.clone());
            }
        }
        _ => {}
    }

    match has_valid_image_url {
        Some(image_url) => {
            let upload_response = upload_image_to_imgur(image_url)
                .await
                .expect("Error uploading image to imgur");

            let uploaded_image_url = upload_response.data.link;
            create_request(&ctx, &msg, uploaded_image_url).await;
        }
        _ => {}
    }
}
