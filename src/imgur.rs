use anyhow::Context as AnyhowContext;
use serenity::prelude::Context;

use crate::structs::{Config, HttpClient, ImgurResponse};

pub async fn upload_image_to_imgur(
    ctx: &Context,
    image_url: String,
) -> Result<ImgurResponse, anyhow::Error> {
    let data = ctx.data.read().await;
    let config = data.get::<Config>().context("Could not get config")?;
    let http_client = &data
        .get::<HttpClient>()
        .context("Could not get http client")?
        .client;

    let request = http_client
        .post("https://api.imgur.com/3/image")
        .header("Authorization", &config.api.imgur_id)
        .body(image_url);

    let response = request.send().await?;
    let raw_json_response = response.text().await?;
    let json = serde_json::from_str::<ImgurResponse>(&raw_json_response)?;

    Ok(json)
}
