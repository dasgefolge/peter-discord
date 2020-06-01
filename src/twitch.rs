use {
    std::{
        collections::BTreeMap,
        sync::Arc,
        thread,
        time::Duration
    },
    futures::prelude::*,
    parking_lot::Condvar,
    serde::Deserialize,
    serenity::{
        model::prelude::*,
        prelude::*,
        utils::MessageBuilder
    },
    twitch_helix::{
        Client,
        model::Stream
    },
    crate::Error
};

const CHANNEL: ChannelId = ChannelId(668518137334857728);
const ROLE: RoleId = RoleId(668534306515320833);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "clientID")]
    client_id: String,
    client_secret: String,
    users: BTreeMap<UserId, twitch_helix::model::UserId>
}

fn client_and_users(ctx_arc: &(Mutex<Option<Context>>, Condvar)) -> Result<(Client, BTreeMap<UserId, twitch_helix::model::UserId>), Error> {
    let (ref ctx_arc, ref cond) = *ctx_arc;
    let mut ctx_guard = ctx_arc.lock(); //TODO async
    if ctx_guard.is_none() {
        cond.wait(&mut ctx_guard); //TODO async
    }
    let ctx = ctx_guard.as_ref().ok_or(Error::MissingContext)?;
    let ctx_data = ctx.data.read(); //TODO async
    let config = ctx_data.get::<crate::Config>().ok_or(Error::MissingConfig)?;
    Ok((Client::new(concat!("peter-discord/", env!("CARGO_PKG_VERSION")), &config.twitch.client_id, &config.twitch.client_secret)?, config.twitch.users.clone()))
}

fn get_users(ctx_arc: &(Mutex<Option<Context>>, Condvar)) -> Result<BTreeMap<UserId, twitch_helix::model::UserId>, Error> {
    //let (ref ctx_arc, _) = *ctx_arc;
    let ctx_guard = ctx_arc.0.lock(); //TODO async
    let ctx = ctx_guard.as_ref().ok_or(Error::MissingContext)?;
    let ctx_data = ctx.data.read(); //TODO async
    let config = ctx_data.get::<crate::Config>().ok_or(Error::MissingConfig)?;
    Ok(config.twitch.users.clone())
}

/// Notifies #twitch when a Gefolge member starts streaming.
pub async fn alerts(ctx_arc: Arc<(Mutex<Option<Context>>, Condvar)>) -> Result<(), Error> { //TODO change return type to Result<!>
    let (client, users) = client_and_users(&ctx_arc)?;
    let first_status = status(&client, users).await?;
    let mut last_status = first_status.keys().cloned().collect::<Vec<_>>();
    loop {
        let users = get_users(&ctx_arc)?;
        let new_status = status(&client, users.clone()).await?;
        for (user_id, stream) in &new_status {
            if !last_status.iter().any(|iter_uid| user_id == iter_uid) {
                let game = stream.game(&client).await?;
                let (ref ctx_arc, _) = *ctx_arc;
                let ctx_guard = ctx_arc.lock();
                let ctx = ctx_guard.as_ref().ok_or(Error::MissingContext)?;
                CHANNEL.send_message(ctx, |m| m
                    .content(MessageBuilder::default().mention(user_id).push(" streamt jetzt auf ").mention(&ROLE))
                    .embed(|e| e
                        .color((0x77, 0x2c, 0xe8))
                        .title(stream)
                        .url(stream.url())
                        .description(game)
                    )
                )?;
            }
        }
        last_status = new_status.keys().cloned().collect();
        thread::sleep(Duration::from_secs(60));
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
