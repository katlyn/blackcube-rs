pub mod defs {
    pub use serde::{Deserialize, Serialize};

    pub use serenity::model::id::{ChannelId, RoleId};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Usrbg {
        pub uid: String,
        pub img: String,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ImgurResponse {
        pub data: ImgurData,
        pub status: u32,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ImgurData {
        pub id: String,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Config {
        pub bot: Bot,
        pub api: Api,
        pub server: Server,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Bot {
        pub application_id: u64,
        pub discord_token: String,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Api {
        pub imgur_id: String,
        pub mongo_url: String,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Server {
        pub request_channel_id: u64,
        pub log_channel_id: ChannelId,
        pub auth_role_id: RoleId,
    }
}
