use crate::structs::{Blacklist, Collections, Config, Usrbg};

use anyhow::Context;
use bson::doc;
use mongodb::sync::Collection;
use mongodb::{options::FindOneAndUpdateOptions, sync::Client};
use serde::de::DeserializeOwned;

// pub fn create() {}

// pub fn read() {}

// Update to only take entry and retrieve uid by entry - Should I even do it that way?
pub fn upsert<T>(
    collection: &Collection<T>,
    uid: &String,
    entry: T,
) -> Result<std::option::Option<T>, mongodb::error::Error>
where
    T: DeserializeOwned + serde::Serialize,
{
    let options = FindOneAndUpdateOptions::builder()
        .upsert(Some(true))
        .build();

    collection.find_one_and_update(
        doc! { "uid": uid },
        doc! { "$set": bson::to_bson(&entry).unwrap() },
        Some(options),
    )
}

pub fn delete<T>(
    collection: &Collection<T>,
    uid: String,
) -> Result<mongodb::results::DeleteResult, mongodb::error::Error> {
    collection.delete_one(doc! { "uid": uid }, None)
}

pub fn connect_database(config: &Config) -> anyhow::Result<Collections> {
    let client =
        Client::with_uri_str(&config.database.url).context("Error connecting to database")?;
    let db = client.database(&config.database.name);
    let usrbg_collection = db.collection::<Usrbg>(&config.database.usrbg_collection);
    let blacklist_collection = db.collection::<Blacklist>(&config.database.blacklist_collection);
    let collections = Collections {
        usrbg: usrbg_collection,
        blacklist: blacklist_collection,
    };
    Ok(collections)
}
