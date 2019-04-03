//! Defines data models the bot deals with.

use std::{
    collections::{
        BTreeMap,
        BTreeSet
    },
    fmt,
    fs::File,
    io::prelude::*,
    ops::Neg,
    path::{
        Path,
        PathBuf
    },
    str::FromStr
};
use bigdecimal::BigDecimal;
use chrono::prelude::*;
use itertools::Itertools;
use serde_derive::{
    Deserialize,
    Serialize
};
use serde_json::Value as Json;
use serenity::model::prelude::*;
use crate::{
    Error,
    Result
};

const EVENTS_DIR: &str = "/usr/local/share/fidera/event";
const USER_DATA_DIR: &str = "/usr/local/share/fidera/userdata";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct Euro(BigDecimal);

impl Neg for Euro {
    type Output = Euro;

    fn neg(self) -> Euro {
        Euro(-self.0)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
pub(crate) enum EventAbrechnungDetail {
    Even {
        amount: Euro,
        label: String,
        people: u16,
        total: Euro
    },
    Flat {
        amount: Euro,
        label: String
    },
    #[serde(rename_all = "camelCase")]
    Weighted {
        amount: Euro,
        label: String,
        nights_attended: u16,
        nights_total: u16,
        total: Euro
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
pub(crate) enum Transaction {
    BankTransfer {
        amount: Euro,
        time: DateTime<Utc>,
        #[serde(rename = "transactionID")]
        transaction_id: String
    },
    Bar {
        amount: Euro,
        time: DateTime<Utc>
    },
    EventAnzahlung {
        amount: Euro,
        default: Option<Euro>,
        event: Event,
        guest: Option<u8>,
        time: DateTime<Utc>
    },
    #[serde(rename_all = "camelCase")]
    EventAnzahlungReturn {
        amount: Euro,
        event: Event,
        extra_remaining: Euro,
        time: DateTime<Utc>
    },
    EventAbrechnung {
        amount: Euro,
        details: Vec<EventAbrechnungDetail>,
        event: Event,
        guest: Option<u8>,
        time: DateTime<Utc>
    },
    #[serde(rename_all = "camelCase")]
    PayPal {
        amount: Euro,
        email: String,
        time: DateTime<Utc>,
        transaction_code: String
    },
    SponsorWerewolfCard {
        amount: Euro,
        faction: String,
        role: String,
        time: DateTime<Utc>
    },
    Transfer {
        amount: Euro,
        comment: Option<String>,
        mensch: UserId,
        time: DateTime<Utc>
    }
}

impl Transaction {
    pub(crate) fn time(&self) -> DateTime<Utc> {
        use Transaction::*;

        match *self {
            BankTransfer { time, .. } => time,
            Bar { time, .. } => time,
            EventAnzahlung { time, .. } => time,
            EventAnzahlungReturn { time, .. } => time,
            EventAbrechnung { time, .. } => time,
            PayPal { time, .. } => time,
            SponsorWerewolfCard { time, .. } => time,
            Transfer { time, .. } => time
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct UserData {
    api_key: Option<String>,
    #[serde(default)]
    pub(crate) transactions: Vec<Transaction>
}

impl UserData {
    fn data_path(user_id: UserId) -> PathBuf {
        Path::new(&format!("{}/{}.json", USER_DATA_DIR, user_id)).into()
    }

    pub(crate) fn load(user_id: UserId) -> Result<UserData> {
        let path = UserData::data_path(user_id);
        if !path.exists() { writeln!(&mut File::create(&path)?, "{{}}")?; }
        Ok(serde_json::from_reader(File::open(path)?)?)
    }

    pub(crate) fn save(&self, user_id: UserId) -> Result<()> {
        Ok(serde_json::to_writer(File::create(UserData::data_path(user_id))?, self)?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventPersonData {
    anzahlung: Option<Euro>,
    food: Json,
    pub(crate) id: u64, // UserId or guest ID < 100
    konto: Option<Json>,
    pub(crate) name: Option<String>, // guests only
    nights: BTreeMap<NaiveDate, String>,
    #[serde(default)]
    orga: BTreeSet<String>,
    signup: String, // Date<Berlin> or DateTime<Berlin>
    pub(crate) via: Option<UserId> // guests only
}

impl EventPersonData {
    fn id(&self) -> Result<UserId, u8> { //TODO use Either instead of Result?
        if self.id < 100 {
            Err(self.id as u8)
        } else {
            Ok(UserId(self.id))
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ProgrammpunktData {
    #[serde(default)]
    closed: bool,
    #[serde(default)]
    description: String,
    end: Option<NaiveDateTime>,
    mayor_election: Option<bool>,
    orga: Option<UserId>,
    pub(crate) signups: Vec<u64>,
    start: Option<NaiveDateTime>,
    strings: BTreeMap<String, String>,
    pub(crate) targets: Option<BTreeMap<u64, u64>>,
    variant: Option<String>,
    votes: Option<BTreeMap<String, Vec<UserId>>>
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventData {
    // This currently uses plain JSON or other types without the proper semantics for some private fields
    anzahlung: Option<Euro>,
    end: NaiveDateTime,
    essen: BTreeMap<NaiveDate, Json>,
    location: String,
    menschen: Vec<EventPersonData>,
    pub(crate) name: String,
    pub(crate) programm: BTreeMap<String, ProgrammpunktData>,
    pub role: RoleId,
    start: NaiveDateTime
}

impl EventData {
    pub(crate) fn person_data_mut(&mut self, id: Result<UserId, u8>) -> Option<&mut EventPersonData> {
        self.menschen.iter_mut()
            .filter(|data| data.id() == id)
            .collect_tuple()
            .map(|(data,)| data)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct Event(String);

impl Event {
    fn data_path(&self) -> PathBuf {
        Path::new(&format!("{}/{}.json", EVENTS_DIR, self.0)).into()
    }

    pub(crate) fn data(&self) -> Result<EventData> {
        Ok(serde_json::from_reader(File::open(self.data_path())?)?)
    }

    pub(crate) fn save_data(&self, data: &EventData) -> Result<()> {
        Ok(serde_json::to_writer(File::create(self.data_path())?, data)?)
    }
}

impl FromStr for Event {
    type Err = Error;

    fn from_str(event_id: &str) -> Result<Event> {
        //TODO check if event exists
        Ok(Event(event_id.to_string()))
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
