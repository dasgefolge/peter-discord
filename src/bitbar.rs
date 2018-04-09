//! Helper functions for the BitBar plugin.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{
        self,
        prelude::*
    }
};

use serenity::model::user::User;

use typemap::Key;

const LOGO: &str = "iVBORw0KGgoAAAANSUhEUgAAACYAAAAmCAYAAACoPemuAAAABGdBTUEAALGPC/xhBQAAACBjSFJNAAB6JgAAgIQAAPoAAACA6AAAdTAAAOpgAAA6mAAAF3CculE8AAAACXBIWXMAABYlAAAWJQFJUiTwAAABWWlUWHRYTUw6Y29tLmFkb2JlLnhtcAAAAAAAPHg6eG1wbWV0YSB4bWxuczp4PSJhZG9iZTpuczptZXRhLyIgeDp4bXB0az0iWE1QIENvcmUgNS40LjAiPgogICA8cmRmOlJERiB4bWxuczpyZGY9Imh0dHA6Ly93d3cudzMub3JnLzE5OTkvMDIvMjItcmRmLXN5bnRheC1ucyMiPgogICAgICA8cmRmOkRlc2NyaXB0aW9uIHJkZjphYm91dD0iIgogICAgICAgICAgICB4bWxuczp0aWZmPSJodHRwOi8vbnMuYWRvYmUuY29tL3RpZmYvMS4wLyI+CiAgICAgICAgIDx0aWZmOk9yaWVudGF0aW9uPjE8L3RpZmY6T3JpZW50YXRpb24+CiAgICAgIDwvcmRmOkRlc2NyaXB0aW9uPgogICA8L3JkZjpSREY+CjwveDp4bXBtZXRhPgpMwidZAAABvklEQVRYCe1WiXGDMBDEqSAlUIJLoASXQAnuQOnA6YB04BJSgkuwO0gJya3gmJUiJCHC4MxwM/I97J7Wh8Cuqt32CewTyJ7AUZAmG91jwVnF7tL1e1ivBTvciH8u4P+iaEMIU4OwqywVmvJGieJb4n1SPTvE6HVDCGkp13qJ/5I+MHjl1yjkGkh3WUaWNvhrX1NvCV07uKnNIGALc7S8bKEgZ09HpRC2mpZqHfU87cT+hbCTznNDj9eUtfGeSrb1+RokVVZTya38kA5v2iXDA/vIwE1CUi9QfWtzg06SKR7jNJ7Ccl2x1uf89jkESrgpxwRxQsaE4gujQwC/xniOfRxywwAvDuH9WlVyxrx91kn1qYTilCnWx01x5+K572HOxG7MHOLxvRO4FipNfYkQ1tb8exzLG2Fg4SmN4XCtk4X/cyYDy70E3m/CxWeIk4f/3Upf5+ORatsIIDYl8GPX517DEYDFjkKPkM9OVmyDZkRW1TmBDfW5Et8k+C1hbYhCqCnXIMq3oxSwGVY3+Ea8bycpcK9Q3PokzVPkiwILPKYWEqO1JqdnSCDOxFJTEeybJU3rJeSdu09gn8CMCfwATTVPuywMjbAAAAAASUVORK5CYII=";

/// `typemap` key for the voice state data required by the BitBar plugin: A mapping of voice channel names to users.
pub struct VoiceStates;

impl Key for VoiceStates {
    type Value = BTreeMap<String, Vec<User>>;
}

/// Takes a mapping from voice channel names to users and dumps the output for the plugin to the file `bitbar.txt`.
pub fn dump_info(voice_states: &<VoiceStates as Key>::Value) -> io::Result<()> {
    let total: usize = voice_states.iter().map(|(_, users)| users.len()).sum();
    let mut f = File::create("bitbar.txt")?;
    if total > 0 {
        writeln!(f, "{}|templateImage={}", total, LOGO)?;
        for (channel_name, users) in voice_states {
            writeln!(f, "---")?;
            writeln!(f, "{}|size=10", channel_name)?;
            for user in users {
                writeln!(f, "{}#{:04}", user.name, user.discriminator)?;
            }
        }
    }
    Ok(())
}
