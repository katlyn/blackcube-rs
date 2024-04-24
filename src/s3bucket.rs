use std::str::FromStr;

use anyhow::{bail, Context as AnyhowContext};
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use s3::{creds::Credentials, Bucket, Region};
use serenity::all::Context;

use crate::structs::{Config, HttpClient, S3Bucket};

pub async fn connect_bucket(config: &Config) -> Result<S3Bucket, anyhow::Error> {
    let region = Region::Custom {
        region: "us-east-1".to_owned(),
        endpoint: config.storage.url.to_owned(),
    };

    let credentials = Credentials::new(
        Some(&config.storage.access_key),
        Some(&config.storage.secret_key),
        None,
        None,
        None,
    )?;

    let bucket = Bucket::new(
        &config.storage.bucket_name,
        region.clone(),
        credentials.clone(),
    )?
    .with_path_style();

    Ok(S3Bucket { bucket })
}

pub async fn upload_image_to_s3bucket(
    ctx: &Context,
    image_url: String,
    uid: String,
) -> Result<String, anyhow::Error> {
    let data = ctx.data.read().await;
    let http_client = &data
        .get::<HttpClient>()
        .context("Could not get http client")?
        .client;

    let response = http_client.get(image_url.clone()).send().await?;
    let content_type_header = response
        .headers()
        .get(CONTENT_TYPE)
        .context("Could not parse content type of image")?
        .clone();
    let parsed_content_type = Mime::from_str(content_type_header.to_str()?)?;
    let extension = parsed_content_type.subtype().to_string();

    let image_bytes = response.bytes().await?;

    let config = data.get::<Config>().context("Could not get config")?;
    let bucket = &data
        .get::<S3Bucket>()
        .context("Could not get bucket")?
        .bucket;

    if !config.settings.image_types.contains(&extension) {
        bail!("Invalid content-type")
    }

    let path = format!("{}{}", config.storage.storage_path, uid);

    let response = bucket
        .put_object_with_content_type(path.clone(), &image_bytes, &parsed_content_type.to_string())
        .await?;

    if response.status_code() != 200 {
        bail!("Error uploading image to minio")
    }

    Ok(format!(
        "{}/{}{}{}",
        config.storage.url, config.storage.bucket_name, config.storage.storage_path, uid
    ))
}

pub async fn delete_image_from_s3_bucket(ctx: &Context, uid: String) -> Result<(), anyhow::Error> {
    let data = ctx.data.read().await;

    let config = data.get::<Config>().context("Could not get config")?;
    let bucket = &data
        .get::<S3Bucket>()
        .context("Could not get bucket")?
        .bucket;

    let path = format!("{}{}", config.storage.storage_path, uid);

    let response = bucket
        .delete_object(path.clone())
        .await?;

    if response.status_code() != 204 {
        bail!("Error deleting image from minio")
    }

    Ok(())
}
