use {
    std::{
        iter,
        thread,
        time::Duration
    },
    serenity::prelude::*,
    crate::{
        GEFOLGE,
        shut_down
    }
};

serenity_utils::ipc! {
    use serenity::model::prelude::*;

    const PORT: u16 = 18807;

    /// Adds the given role to the given user. No-op if the user already has the role.
    fn add_role(ctx: &Context, user: UserId, role: RoleId) -> Result<(), String> {
        let roles = iter::once(role).chain(GEFOLGE.member(ctx, user).map_err(|e| format!("failed to get member data: {}", e))?.roles.into_iter());
        GEFOLGE.edit_member(ctx, user, |m| m.roles(roles)).map_err(|e| format!("failed to edit roles: {}", e))?;
        Ok(())
    }

    /// Sends the given message, unescaped, to the given channel.
    fn channel_msg(ctx: &Context, channel: ChannelId, msg: String) -> Result<(), String> {
        channel.say(ctx, msg).map_err(|e| format!("failed to send channel message: {}", e))?;
        Ok(())
    }

    /// Sends the given message, unescaped, directly to the given user.
    fn msg(ctx: &Context, rcpt: UserId, msg: String) -> Result<(), String> {
        rcpt.create_dm_channel(ctx).map_err(|e| format!("failed to get/create DM channel: {}", e))?.say(ctx, msg).map_err(|e| format!("failed to send DM: {}", e))?;
        Ok(())
    }

    /// Shuts down the bot and cleanly exits the program.
    fn quit(ctx: &Context) -> Result<(), String> {
        shut_down(&ctx);
        thread::sleep(Duration::from_secs(1)); // wait to make sure websockets can be closed cleanly
        Ok(())
    }

    /// Changes the display name for the given user in the Gefolge guild to the given string.
    ///
    /// If the given string is equal to the user's username, the display name will instead be removed.
    fn set_display_name(ctx: &Context, user_id: UserId, new_display_name: String) -> Result<(), String> {
        let user = user_id.to_user(ctx).map_err(|e| format!("failed to get user for set-display-name: {}", e))?;
        GEFOLGE.edit_member(ctx, &user, |e| e.nickname(if user.name == new_display_name { "" } else { &new_display_name })).map_err(|e| match e {
            serenity::Error::Http(e) => if let HttpError::UnsuccessfulRequest(response) = *e {
                format!("failed to set display name: {:?}", response)
            } else {
                e.to_string()
            },
            _ => e.to_string()
        })
    }
}
