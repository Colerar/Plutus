pub mod info;
pub mod live;
pub(super) mod macros;
pub mod passport;
pub mod share;

trait FromCode {
  fn from_code(code: i32) -> Self;
}
