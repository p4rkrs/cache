use crate::resp_impl::RespValueExt;
use redis_async::{
    error::Error,
    resp::{FromResp, RespValue},
};
use serde::de::DeserializeOwned;
use serde_aux::prelude::*;
use serde_json::{Map, Number, Value};
use serenity::model::permissions::Permissions;
use std::collections::HashSet;

fn convert<T: DeserializeOwned>(resp: RespValue) -> Result<T, Error> {
    let values = match resp {
        RespValue::Array(x) => x,
        _ => return Err(Error::RESP("Expected an array".to_owned(), None)),
    };

    let map = create_hashmap(values);

    // Should this really not panic? Is it better to let the user handle the error, or should we
    // force unwinds for them to realise what happened?
    //
    // Ok(serde_json::from_value(Value::from(map)).expect("err deserializing"))

    match serde_json::from_value(Value::from(map)) {
        Ok(deserialized) => Ok(deserialized),
        Err(err) => Err(Error::Unexpected(format!("Couldn't deserialize a cached value: err={:?}", err))),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Guild {
    pub afk_channel_id: Option<u64>,
    pub channels: HashSet<u64>,
    pub features: HashSet<String>,
    pub members: HashSet<u64>,
    pub name: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub owner_id: u64,
    pub region: String,
    pub roles: HashSet<u64>,
    pub voice_states: HashSet<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GuildChannel {
    pub bitrate: Option<u64>,
    pub category_id: Option<u64>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub kind: u64,
    pub name: String,
    pub permission_overwrites: Vec<PermissionOverwrite>,
    pub user_limit: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Member {
    pub deaf: bool,
    pub nick: Option<String>,
    pub roles: Vec<u64>,
    pub user: User,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PermissionOverwrite {
    pub allow: Permissions,
    pub deny: Permissions,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub kind: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Role {
    #[serde(deserialize_with = "deserialize_string_from_number")]
    pub name: String,
    pub permissions: Permissions,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub bot: bool,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub discriminator: u16,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: u64,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VoiceState {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub channel_id: u64,
    #[serde(deserialize_with = "deserialize_string_from_number")]
    pub session_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Choice {
    pub blobs: Vec<String>,
}

fn create_hashmap(resp: Vec<RespValue>) -> Map<String, Value> {
    let mut map = Map::with_capacity(resp.len() / 2);
    let mut iter = resp.into_iter();

    loop {
        let key = match iter.next() {
            Some(key) => key,
            None => break,
        };
        let value = iter.next().unwrap();
        let v = resp_to_value(value);
        map.insert(key.into_string(), v);
    }

    map
}

fn resp_to_value(resp: RespValue) -> Value {
    match resp {
        RespValue::Nil => Value::Null,
        RespValue::Array(resps) => Value::Array(resps.into_iter().map(resp_to_value).collect()),
        RespValue::BulkString(bytes) => {
            let string = String::from_utf8(bytes).unwrap();

            if let Ok(v) = string.parse::<u64>() {
                Value::Number(Number::from(v))
            } else {
                Value::String(string)
            }
        },
        RespValue::Error(why) => panic!("{:?}", why),
        RespValue::Integer(integer) => Value::Number(Number::from(integer)),
        RespValue::SimpleString(string) => Value::String(string),
    }
}


macro from_resp_impls($($struct:ident,)+) {
    $(
        impl FromResp for $struct {
            fn from_resp_int(resp: RespValue) -> Result<Self, Error> {
                convert(resp)
            }
        }
    )+
}

from_resp_impls![
    Guild,
    GuildChannel,
    Member,
    PermissionOverwrite,
    Role,
    User,
    VoiceState,
    Choice,
];

#[cfg(test)]
mod tests {
    use redis_async::resp::{FromResp, RespValue};
    use super::*;

    #[test]
    fn test_role() {
        let value = RespValue::Array(vec![
            RespValue::BulkString(b"name".to_vec()),
            RespValue::BulkString(b"test".to_vec()),
            RespValue::BulkString(b"permissions".to_vec()),
            RespValue::BulkString(b"8".to_vec()),
        ]);

        assert!(Role::from_resp(value).is_ok());

        let value = RespValue::Array(vec![
            RespValue::BulkString(b"name".to_vec()),
            RespValue::BulkString(b"0123456".to_vec()),
            RespValue::BulkString(b"permissions".to_vec()),
            RespValue::BulkString(b"8".to_vec()),
        ]);

        assert!(Role::from_resp(value).is_ok());
    }

    #[test]
    fn test_voice_state() {
        let value = RespValue::Array(vec![
            RespValue::BulkString(b"channel_id".to_vec()),
            RespValue::BulkString(b"500000000000000000".to_vec()),
            RespValue::BulkString(b"session_id".to_vec()),
            RespValue::BulkString(b"946f395aa3c194fda2aa7baa2e402d2b".to_vec()),
            RespValue::BulkString(b"token".to_vec()),
            RespValue::BulkString(b"450d2eedffbdad13".to_vec()),
        ]);

        assert!(VoiceState::from_resp(value).is_ok());
    }

    #[test]
    fn test_voice_state_numeric_fields() {
        let value = RespValue::Array(vec![
            RespValue::BulkString(b"channel_id".to_vec()),
            RespValue::BulkString(b"500000000000000000".to_vec()),
            RespValue::BulkString(b"session_id".to_vec()),
            RespValue::BulkString(b"946f395aa3c194fda2aa7baa2e402d2b".to_vec()),
        ]);

        assert!(VoiceState::from_resp(value).is_ok());

        let value = RespValue::Array(vec![
            RespValue::BulkString(b"channel_id".to_vec()),
            RespValue::BulkString(b"500000000000000000".to_vec()),
            RespValue::BulkString(b"session_id".to_vec()),
            RespValue::BulkString(b"11111111111111111111111111111111".to_vec()),
        ]);

        assert!(VoiceState::from_resp(value).is_ok());
    }

    #[test]
    fn test_choice() {
        let value = RespValue::Array(vec![
            RespValue::BulkString(b"blobs".to_vec()),
            RespValue::Array(vec![
                RespValue::BulkString(b"thisi s not actually valid but whatever".to_vec()),
            ]),
        ]);

        assert!(Choice::from_resp(value).is_ok());

        let mut value = RespValue::Array(vec![
            RespValue::BulkString(b"blobs".to_vec()),
            RespValue::BulkString(b"this should".to_vec()),
            RespValue::BulkString(b"err the test".to_vec()),
        ]);

        assert!(std::panic::catch_unwind(|| Choice::from_resp(value.clone())).is_err());

        value.push(RespValue::BulkString(b"should still err the test".to_vec()));

        assert!(Choice::from_resp(value).is_err());
    }
}
