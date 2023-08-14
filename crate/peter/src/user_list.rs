//! Helper functions for maintaining the guild member list on disk, which is used by gefolge.org to verify logins.

use {
    std::{
        collections::BTreeSet,
        future::Future,
        num::NonZeroU16,
        pin::Pin,
    },
    chrono::prelude::*,
    futures::future,
    serde::{
        Deserialize,
        Serialize,
    },
    serenity::{
        model::prelude::*,
        prelude::*,
    },
    sqlx::{
        PgPool,
        types::Json,
    },
    crate::{
        Database,
        GEFOLGE,
    },
};

#[derive(Deserialize, Serialize)]
struct Profile {
    bot: bool,
    discriminator: Option<NonZeroU16>,
    joined: Option<DateTime<Utc>>,
    nick: Option<String>,
    roles: BTreeSet<RoleId>,
    snowflake: UserId,
    username: String,
}

/// Add a Discord account to the list of Gefolge guild members.
pub async fn add(pool: &PgPool, member: &Member) -> sqlx::Result<()> {
    let join_date = sqlx::query_scalar!(r#"SELECT value AS "value: Json<Profile>" FROM json_profiles WHERE id = $1"#, member.user.id.get() as i64)
        .fetch_optional(pool).await?
        .and_then(|value| value.joined);
    sqlx::query!("INSERT INTO json_profiles (id, value) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET value = EXCLUDED.value",
        member.user.id.get() as i64,
        Json(Profile {
            bot: member.user.bot,
            discriminator: member.user.discriminator,
            joined: member.joined_at.map(|joined_at| *joined_at).or(join_date),
            nick: member.nick.clone(),
            roles: member.roles.iter().copied().collect(),
            snowflake: member.user.id,
            username: member.user.name.clone(),
        }) as _,
    ).execute(pool).await?;
    Ok(())
}

pub enum Exporter {}

impl serenity_utils::handler::user_list::ExporterMethods for Exporter {
    fn upsert<'a>(ctx: &'a Context, member: &'a Member) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            if member.guild_id != GEFOLGE { return Ok(()) }
            let data = ctx.data.read().await;
            let pool = data.get::<Database>().expect("missing database connection");
            add(pool, member).await?;
            Ok(())
        })
    }

    fn replace_all<'a>(ctx: &'a Context, members: Vec<&'a Member>) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            let data = ctx.data.read().await;
            let pool = data.get::<Database>().expect("missing database connection");
            for member in members { //TODO parallel?
                if member.guild_id == GEFOLGE {
                    add(pool, member).await?;
                }
            }
            Ok(())
        })
    }

    fn remove<'a>(_: &'a Context, _: UserId, _: GuildId) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        //TODO mark as non-member, remove entirely if no user data exists
        Box::pin(future::ok(()))
    }
}
