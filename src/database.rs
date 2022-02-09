pub mod actions {
    use crate::structs::defs::{Blacklist, Collections, Config, Usrbg};
    use bson::doc;
    use mongodb::options::FindOneAndUpdateOptions;

    pub fn create() {}

    pub fn read() {}

    pub fn upsert(
        collections: &Collections,
        uid: &String,
        entry: &Usrbg,
    ) -> Result<(), bool> {
        let options = FindOneAndUpdateOptions::builder()
        .upsert(Some(true))
        .build();
        let res = collections.usrbg.find_one_and_update(
            doc! { "uid": uid },
            doc! { "$set": bson::to_bson(entry).unwrap() },
            Some(options),
        );
        match res {
            Ok(res) => {
                Ok(())
            }
            Err(err) => {
                Err(true)
            }
        }
    }

    pub fn delete(
        collections: &Collections,
        uid: String,
    ) -> Result<mongodb::results::DeleteResult, mongodb::error::Error> {
        collections.usrbg.delete_one(doc! { "uid": uid }, None)
    }
}

pub mod init {
    use crate::structs::defs::{Blacklist, Collections, Config, Usrbg};
    use mongodb::sync::Client;

    pub fn connect_database(config: &Config) -> Collections {
        let client =
            Client::with_uri_str(&config.database.url).expect("Error connecting to database");
        let db = client.database(&config.database.name);
        let usrbg_collection = db.collection::<Usrbg>(&config.database.usrbg_collection);
        let blacklist_collection =
            db.collection::<Blacklist>(&config.database.blacklist_collection);
        let collections = Collections {
            usrbg: usrbg_collection,
            blacklist: blacklist_collection,
        };
        collections
    }
}
