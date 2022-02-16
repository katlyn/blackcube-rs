use crate::structs::{Blacklist, Collections, Config, Usrbg};
use crate::TypeInfo;

use bson::doc;
use mongodb::options::FindOneAndUpdateOptions;

// pub fn create() {}

// pub fn read() {}

pub fn upsert(
    collections: &Collections,
    uid: &String,
    entry: impl TypeInfo + serde::Serialize,
) -> Result<(), ()> {
    let options = FindOneAndUpdateOptions::builder()
        .upsert(Some(true))
        .build();

    match entry.type_of() {
        "Usrbg" => {
            collections
                .usrbg
                .find_one_and_update(
                    doc! { "uid": uid },
                    doc! { "$set": bson::to_bson(&entry).unwrap() },
                    Some(options),
                )
                .unwrap();
        }
        "Blacklist" => {
            collections
                .blacklist
                .find_one_and_update(
                    doc! { "uid": uid },
                    doc! { "$set": bson::to_bson(&entry).unwrap() },
                    Some(options),
                )
                .unwrap();
        }
        _ => {}
    }
    Ok(())
}

pub fn delete(
    collections: &Collections,
    uid: String,
) -> Result<mongodb::results::DeleteResult, mongodb::error::Error> {
    collections.usrbg.delete_one(doc! { "uid": uid }, None)
}

use mongodb::sync::Client;

pub fn connect_database(config: &Config) -> Collections {
    let client = Client::with_uri_str(&config.database.url).expect("Error connecting to database");
    let db = client.database(&config.database.name);
    let usrbg_collection = db.collection::<Usrbg>(&config.database.usrbg_collection);
    let blacklist_collection = db.collection::<Blacklist>(&config.database.blacklist_collection);
    let collections = Collections {
        usrbg: usrbg_collection,
        blacklist: blacklist_collection,
    };
    collections
}
