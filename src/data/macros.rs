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
          crate::data::macros::forward_ints::try_from_unsigned!(i32: u64);

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
