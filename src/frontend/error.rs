use std::fmt;

use std::error::Error;


#[derive(Debug)]
pub struct UnimplementedError(pub String);

impl Error for UnimplementedError {}

impl fmt::Display for UnimplementedError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{} unimplemented", self.0)
  }
}

#[derive(Debug)]
pub struct PushKeyError(pub Box<dyn fmt::Debug>);

impl Error for PushKeyError {}

impl fmt::Display for PushKeyError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "key {:#?} already exists", self.0)
  }
}
