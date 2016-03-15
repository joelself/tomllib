#[macro_use]
extern crate nom;
extern crate regex;
#[macro_use]
extern crate log;
#[macro_use]
mod internals;
pub mod types;

use std::fmt;
use std::fmt::Display;
use types::{ParseResult, Value, Children};
use internals::parser::Parser;

pub struct TOMLParser<'a> {
  parser: Parser<'a>,
}

impl<'a> TOMLParser<'a> {
  pub fn new() -> TOMLParser<'a> {
    TOMLParser{parser: Parser::new()}
  }
  pub fn parse(mut self, input: &'a str) -> (TOMLParser<'a>, ParseResult<'a>) {
    let (tmp, result) = self.parser.parse(input);
    self.parser = tmp;
    (self, result)
  }
  pub fn get_value<S>(self: &TOMLParser<'a>, key: S) -> Option<Value<'a>> where S: Into<String> {
    self.parser.get_value(key)
  }
  pub fn get_children<S>(self: &TOMLParser<'a>, key: S) -> Option<&Children> where S: Into<String> {
    self.parser.get_children(key)
  }
  pub fn set_value<S>(self: &mut TOMLParser<'a>, key: S, val: Value<'a>) -> bool where S: Into<String> {
    self.parser.set_value(key, val)
  }
}

impl<'a> Display for TOMLParser<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.parser)
	}
}
