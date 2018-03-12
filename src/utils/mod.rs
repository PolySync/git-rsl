use hex::{self, FromHex};
use serde::{Serializer, Deserialize, Deserializer};


pub mod test_helper;
pub mod git;
pub mod gpg;

/// Serializes `buffer` to a lowercase hex string.
pub fn buffer_to_hex<T, S>(buffer: &T, serializer: S) -> Result<S::Ok, S::Error>
  where T: AsRef<[u8]>,
        S: Serializer
{
  serializer.serialize_str(&hex::encode(buffer))
}

/// Deserializes a lowercase hex string to a `Vec<u8>`.
pub fn hex_to_buffer<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
  where D: Deserializer<'de>
{
  use serde::de::Error;
  String::deserialize(deserializer)
    .and_then(|string| Vec::from_hex(&string).map_err(|err| Error::custom(err.to_string())))
}
