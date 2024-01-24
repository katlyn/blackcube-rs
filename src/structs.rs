use std::collections::HashMap;

use reqwest::Client;
pub use serde::{Deserialize, Serialize};

pub use serenity::model::id::{ChannelId, RoleId};
use serenity::{
    all::{MessageId, UserId},
    prelude::TypeMapKey,
};

pub struct Collections {
    pub usrbg: mongodb::sync::Collection<Usrbg>,
    pub blacklist: mongodb::sync::Collection<Blacklist>,
}

impl TypeMapKey for Collections {
    type Value = Collections;
}

pub struct HttpClient {
    pub client: Client,
}

impl TypeMapKey for HttpClient {
    type Value = HttpClient;
}

pub struct PendingRequestUidStore;

impl TypeMapKey for PendingRequestUidStore {
    type Value = HashMap<UserId, MessageId>;
}

pub struct PendingRequestMidStore;

impl TypeMapKey for PendingRequestMidStore {
    type Value = HashMap<MessageId, MessageId>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usrbg {
    pub uid: String,
    pub img: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Blacklist {
    pub uid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImgurResponse {
    pub data: ImgurData,
    pub status: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImgurData {
    pub id: String,
    pub link: String,
    // remove link later after fixing compiler
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub bot: Bot,
    pub api: Api,
    pub database: Database,
    pub server: Server,
    pub settings: Settings,
}

impl TypeMapKey for Config {
    type Value = Config;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub image_types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bot {
    pub application_id: u64,
    pub discord_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Api {
    pub imgur_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub url: String,
    pub name: String,
    pub usrbg_collection: String,
    pub blacklist_collection: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Server {
    pub request_channel_id: ChannelId,
    pub log_channel_id: ChannelId,
    pub command_channel_id: ChannelId,
    pub auth_role_id: RoleId,
}
