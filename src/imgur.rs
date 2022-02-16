use crate::{structs::ImgurResponse, CONFIG, HTTP_CLIENT};

pub async fn upload_image_to_imgur(image_url: String) -> Result<ImgurResponse, anyhow::Error> {
    let request = HTTP_CLIENT
        .post("https://api.imgur.com/3/image")
        .header("Authorization", &CONFIG.api.imgur_id)
        .body(image_url);

    let response = request.send().await?;
    let raw_json_response = response.text().await?;
    let json = serde_json::from_str::<ImgurResponse>(&raw_json_response)?;

    Ok(json)
}
