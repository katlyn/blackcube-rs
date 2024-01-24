use anyhow::Context as AnyhowContext;
use bson::doc;
use serenity::{
    model::{
        prelude::{Member, PartialMember},
        user::User,
    },
    prelude::Context,
};

use crate::structs::{Collections, Config};
pub trait HasAuth {
    async fn has_auth(&self, ctx: &Context) -> anyhow::Result<bool>;
}

impl HasAuth for PartialMember {
    async fn has_auth(&self, ctx: &Context) -> anyhow::Result<bool> {
        let data = ctx.data.read().await;
        let config = data.get::<Config>().context("Could not get config")?;
        Ok(self.roles.contains(&config.server.auth_role_id))
    }
}
impl HasAuth for Member {
    async fn has_auth(&self, ctx: &Context) -> anyhow::Result<bool> {
        let data = ctx.data.read().await;
        let config = data.get::<Config>().context("Could not get config")?;
        Ok(self.roles.contains(&config.server.auth_role_id))
    }
}

pub trait IsBlacklisted {
    async fn is_blacklisted(&self, ctx: &Context) -> anyhow::Result<bool>;
}

impl IsBlacklisted for User {
    async fn is_blacklisted(&self, ctx: &Context) -> anyhow::Result<bool> {
        let data = ctx.data.read().await;
        let collections = data
            .get::<Collections>()
            .context("Could not get collections")?;

        let blacklist_search_query = doc! { "uid": self.id.get().to_string() };
        let blacklist_search_result = collections.blacklist.find_one(blacklist_search_query, None);
        Ok(blacklist_search_result
            .context("Could not complete blacklist search")?
            .is_some())
    }
}
