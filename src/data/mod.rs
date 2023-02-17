pub mod live;
pub mod share;
pub mod passport;
pub mod info;
pub(super) mod macros;

trait FromCode {
  fn from_code(code: i32) -> Self;
}
