//! Helper functions for maintaining the guild member list on disk, which is used by gefolge.org to verify logins.

use {
    std::{
        collections::BTreeSet,
        fmt,
        future::Future,
        pin::Pin,
    },
    chrono::prelude::*,
    futures::future,
    lazy_regex::regex_is_match,
    serde::{
        Deserialize,
        Deserializer,
        Serialize,
        de::Error as _,
    },
    serde_plain::derive_serialize_from_display,
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

#[derive(Deserialize)]
#[serde(untagged)]
enum SerdeDiscriminator {
    Number(i16),
    String(String),
}

#[derive(Debug, thiserror::Error)]
enum InvalidDiscriminator {
    #[error(transparent)] ParseInt(#[from] std::num::ParseIntError),
    #[error("discriminator must be between 0 and 10000, got {0}")]
    Range(i16),
    #[error("discriminator must be 4 digits 0-9")]
    StringPattern,
}

impl TryFrom<SerdeDiscriminator> for Discriminator {
    type Error = InvalidDiscriminator;

    fn try_from(value: SerdeDiscriminator) -> Result<Self, InvalidDiscriminator> {
        let number = match value {
            SerdeDiscriminator::Number(n) => n,
            SerdeDiscriminator::String(s) => if regex_is_match!("^[0-9]{4}$", &s) {
                s.parse()?
            } else {
                return Err(InvalidDiscriminator::StringPattern)
            },
        };
        if number > 9999 { return Err(InvalidDiscriminator::Range(number)) }
        Ok(Self(number))
    }
}

#[derive(Debug, Clone, Copy, Deserialize, sqlx::Type)]
#[serde(try_from = "SerdeDiscriminator")]
#[sqlx(transparent)]
struct Discriminator(i16);

impl fmt::Display for Discriminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.0)
    }
}

derive_serialize_from_display!(Discriminator);

fn discord_opt_discriminator<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Discriminator>, D::Error> {
    Ok(match SerdeDiscriminator::deserialize(deserializer)? {
        SerdeDiscriminator::Number(0) => None,
        SerdeDiscriminator::String(s) if s == "0" => None,
        disc => Some(disc.try_into().map_err(D::Error::custom)?),
    })
}

#[derive(Deserialize, Serialize)]
struct Profile {
    bot: bool,
    #[serde(deserialize_with = "discord_opt_discriminator")]
    discriminator: Option<Discriminator>,
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
            discriminator: member.user.discriminator.map(|discrim| Discriminator(discrim.get() as i16)),
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
