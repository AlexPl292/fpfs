use serde::{de, Serialize};
use serde_json::Error;

pub fn to_string<T>(obj: &T) -> Result<String, Error>
where
    T: ?Sized + Serialize,
{
    serde_json::to_string(obj)
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T, Error>
where
    T: de::Deserialize<'a>,
{
    serde_json::from_str(s)
}
