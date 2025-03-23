use std::fmt;

use serde::{
  de::{Error as DeError, Unexpected, Visitor},
  Deserializer,
};
use serde_with::DeserializeAs;

pub struct BoolFromIntString;

impl<'de> DeserializeAs<'de, bool> for BoolFromIntString {
  fn deserialize_as<D>(deserializer: D) -> Result<bool, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct U8Visitor;
    impl Visitor<'_> for U8Visitor {
      type Value = bool;

      fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string 0 or 1")
      }

      fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
      where
        E: DeError,
      {
        match v {
          "0" => Ok(false),
          "1" => Ok(true),
          unexp => Err(DeError::invalid_value(Unexpected::Str(unexp), &"0 or 1")),
        }
      }
    }

    deserializer.deserialize_str(U8Visitor)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::Deserialize;
  use serde_json::json;
  use serde_with::serde_as;

  #[test]
  fn bool_from_int_string() {
    #[serde_as]
    #[derive(Deserialize)]
    struct Struct {
      #[serde_as(as = "BoolFromIntString")]
      a_bool: bool,
    }

    let de = serde_json::from_value::<Struct>(json!({"a_bool": "1"}))
      .unwrap()
      .a_bool;
    assert!(de);
  }
}
