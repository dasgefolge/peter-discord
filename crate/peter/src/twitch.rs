use {
    std::{
        collections::BTreeMap,
        convert::Infallible as Never,
        time::Duration,
    },
    futures::prelude::*,
    serde::Deserialize,
    serenity::{
        model::prelude::*,
        prelude::*,
        utils::MessageBuilder,
    },
    serenity_utils::RwFuture,
    tokio::time::delay_for,
    twitch_helix::{
        Client,
        model::Stream,
    },
    crate::Error,
};

const CHANNEL: ChannelId = ChannelId(668518137334857728);
const ROLE: RoleId = RoleId(668534306515320833);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "clientID")]
    client_id: String,
    oauth_token: String,
    users: BTreeMap<UserId, twitch_helix::model::UserId>
}

async fn client_and_users(ctx_fut: &RwFuture<Context>) -> Result<(Client, BTreeMap<UserId, twitch_helix::model::UserId>), Error> {
    let ctx = ctx_fut.read().await;
    let ctx_data = (*ctx).data.read().await;
    let config = ctx_data.get::<crate::Config>().ok_or(Error::MissingConfig)?;
    Ok((Client::new(concat!("peter-discord/", env!("CARGO_PKG_VERSION")), &config.twitch.client_id, &config.twitch.oauth_token)?, config.twitch.users.clone())) //TODO automatically renew OAuth token
}

async fn get_users(ctx_fut: &RwFuture<Context>) -> Result<BTreeMap<UserId, twitch_helix::model::UserId>, Error> {
    let ctx = ctx_fut.read().await;
    let ctx_data = (*ctx).data.read().await;
    let config = ctx_data.get::<crate::Config>().ok_or(Error::MissingConfig)?;
    Ok(config.twitch.users.clone())
}

/// Notifies #twitch when a Gefolge member starts streaming.
pub async fn alerts(ctx_fut: RwFuture<Context>) -> Result<Never, Error> {
    let (client, users) = client_and_users(&ctx_fut).await?;
    let first_status = status(&client, users).await?;
    let mut last_status = first_status.keys().cloned().collect::<Vec<_>>();
    loop {
        let users = get_users(&ctx_fut).await?;
        let new_status = status(&client, users.clone()).await?;
        for (user_id, stream) in &new_status {
            if !last_status.iter().any(|iter_uid| user_id == iter_uid) {
                let game = stream.game(&client).await?;
                let ctx = ctx_fut.read().await;
                CHANNEL.send_message(&*ctx, |m| m
                    .content(MessageBuilder::default().mention(user_id).push(" streamt jetzt auf ").mention(&ROLE))
                    .embed(|e| e
                        .color((0x77, 0x2c, 0xe8))
                        .title(stream)
                        .url(stream.url())
                        .description(game)
                    )
                ).await?;
            }
        }
        last_status = new_status.keys().cloned().collect();
        delay_for(Duration::from_secs(60)).await;
    }
}

/// Returns the set of Gefolge members who are currently live on Twitch.
async fn status(client: &Client, users: BTreeMap<UserId, twitch_helix::model::UserId>) -> Result<BTreeMap<UserId, Stream>, Error> {
    let (discord_ids, twitch_ids) = users.into_iter().unzip::<_, _, Vec<_>, _>();
    Ok(
        discord_ids.into_iter()
            .zip(Stream::list(client, None, Some(twitch_ids), None).try_collect::<Vec<_>>().await?)
            .collect()
    )
}
