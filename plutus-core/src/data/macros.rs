pub(super) mod forward_ints {
  macro_rules! de_as {
    ( $forward_to:ty : $($int_ty:ty),+ $(,)? ) => {
      $(
        paste::paste! {
          #[inline]
          fn [< visit_ $int_ty >] <E>(
            self,
            v: $int_ty,
          ) -> core::result::Result<Self::Value, E>
          where
            E: serde::de::Error,
          {
            self. [< visit_ $forward_to >] (v as $forward_to)
          }
        }
      )+
    };
  }
  pub(crate) use de_as;

  macro_rules! try_from_signed {
    ( $forward_to:ty : $($int_ty:ty),+ $(,)? ) => {
      $(
        paste::paste! {
          #[inline]
          fn [< visit_ $int_ty >] <E>(
            self,
            v: $int_ty,
          ) -> core::result::Result<Self::Value, E>
          where
            E: serde::de::Error,
          {
            match $forward_to::try_from(v) {
              Ok(int) => self. [< visit_ $forward_to >] (int as $forward_to),
              Err(_) => {
                let unexpected = serde::de::Unexpected::Signed(v as i64);
                Err(serde::de::Error::invalid_type(unexpected, &self))
              }
            }
          }
        }
      )+
    };
  }
  pub(crate) use try_from_signed;

  macro_rules! try_from_unsigned {
    ( $forward_to:ty : $($int_ty:ty),+ $(,)? ) => {
      $(
        paste::paste! {
          #[inline]
          fn [< visit_ $int_ty >] <E>(
            self,
            v: $int_ty,
          ) -> core::result::Result<Self::Value, E>
          where
            E: serde::de::Error,
          {
            match $forward_to::try_from(v) {
              Ok(int) => self. [< visit_ $forward_to >] (int as $forward_to),
              Err(_) => {
                let unexpected = serde::de::Unexpected::Unsigned(v as u64);
                Err(serde::de::Error::invalid_type(unexpected, &self))
              }
            }
          }
        }
      )+
    };
  }
  pub(crate) use try_from_unsigned;
}

macro_rules! de_from_code_impl {
  ($T:ty) => {
    impl<'de> serde::de::Deserialize<'de> for $T {
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: serde::Deserializer<'de>,
      {
        struct De;
        impl<'de> serde::de::Visitor<'de> for De {
          type Value = $T;

          fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
          where
            E: serde::de::Error,
          {
            Ok(<$T as FromCode>::from_code(v))
          }

          crate::data::macros::forward_ints::de_as!(i32: u8, u16, i8, i16);
          crate::data::macros::forward_ints::try_from_signed!(i32: i64);
          crate::data::macros::forward_ints::try_from_unsigned!(i32: u32, u64);

          fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str(concat!(stringify!($T), "(i32)"))
          }
        }
        deserializer.deserialize_i32(De)
      }
    }
  };
}
pub(super) use de_from_code_impl;

pub(super) mod json_value {

  macro_rules! get_value {
    ( $(let $var_name:ident = $info:ident[$idx:expr]);+ $(;)? ) => {
      $(
        let $var_name = $info
          .get($idx)
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an element, but None"))?;
      )+
    };
  }
  pub(crate) use get_value;

  macro_rules! get_as_obj_struct {
    ( $(let $var_name:ident : $struct_ty:ty = $info:ident[$idx:expr]);+ $(;)? ) => {
      $(
        let $var_name = $info
          .get($idx)
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an element, but None"))?;
        anyhow::ensure!(
          $var_name.is_object(),
          concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an object ", stringify!($struct_ty), ", but None"),
        );
        let $var_name = serde_json::from_value($var_name.clone())
          .context(concat!("Failed to deserialize ", stringify!($struct_ty)))?;
      )+
    };
  }
  pub(crate) use get_as_obj_struct;

  macro_rules! get_as_array {
    ( $(let $var_name:ident = $info:ident[$idx:expr]);+ $(;)? ) => {
      $(
        let $var_name = $info
          .get($idx)
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an element, but None"))?
          .as_array()
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected array"))?;
      )+
    };
  }
  pub(crate) use get_as_array;

  macro_rules! get_as_str {
    ( $(let $var_name:ident = $info:ident[$idx:expr]);+ $(;)? ) => {
      $(
        let $var_name = $info
          .get($idx)
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an element, but None"))?
          .as_str()
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected str"))?;
      )+
    };
  }
  pub(crate) use get_as_str;

  macro_rules! get_as_string {
    ( $(let $var_name:ident = $info:ident[$idx:expr]);+ $(;)? ) => {
      $(
        let $var_name = $info
          .get($idx)
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an element, but None"))?
          .as_str()
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected str"))?
          .to_string();
      )+
    };
  }
  pub(crate) use get_as_string;

  macro_rules! get_as_u64 {
    ( $(let $var_name:ident = $info:ident[$idx:expr]);+ $(;)? ) => {
      $(
        let $var_name = $info
          .get($idx)
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an element, but None"))?
          .as_u64()
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected u64"))?;
      )+
    };
  }
  pub(crate) use get_as_u64;

  macro_rules! get_as_u64_as_bool {
    ( $(let $var_name:ident = $info:ident[$idx:expr]);+ $(;)? ) => {
      $(
        let $var_name = $info
          .get($idx)
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an element, but None"))?
          .as_u64()
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected u64"))
          .map(|num| num == 1)?;
      )+
    };
  }
  pub(crate) use get_as_u64_as_bool;

  macro_rules! get_as_i64 {
    ( $(let $var_name:ident = $info:ident[$idx:expr]);+ $(;)? ) => {
      $(
        let $var_name = $info
          .get($idx)
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected an element, but None"))?
          .as_i64()
          .context(concat!("$.", stringify!($idx), " ", stringify!($var_name)," expected i64"))?;
      )+
    };
  }
  pub(crate) use get_as_i64;
}
