
//! Parse and manipulate TOML documents while preserving whitespace and comments with tomllib.
//!
//! tomllib is a Rust library for parsing, manipulating and outputting TOML documents. tomllib strives to preserve the
//! originalof your document including optional whitespace and comments. The code is available on
//! [GitHub](https://github.com/joelself/tomllib).
//!
//! `TOMLParser` is located at the root of the module, while associated types in the `types` module. Because of the way
//! the parse method works it needs to take ownership of the parser. When it returns, it returns ownership of the parser
//! along with a `ParseResult` that contains a result and any errors.
//!
//! Here's a quick example of how you parse a document, then get and set some values:
//!
//! # Examples
//!
//! ```
//! use tomllib::TOMLParser;
//! use tomllib::types::Value;
//!
//! let parser = TOMLParser::new();
//! let toml_doc = r#"
//! [table] # This is a comment
//!   "Key One" = "A Value" # This line is indented
//!   ## Empty line
//!     Key2 = 1918-07-02 # This line is indented twice
//! "#;
//! // Get back the parser and a result from the parse method in a tuple
//! let (mut parser, result) = parser.parse(toml_doc);
//! let value = parser.get_value("table.\"Key One\"");
//! assert_eq!(value.unwrap(), Value::basic_string("A Value").unwrap());
//! parser.set_value("table.\"Key One\"", Value::float(9.876));
//! parser.set_value("table.Key2", Value::bool(false));
//! assert_eq!(&format!("{}", parser), r#"
//! [table] # This is a comment
//!   "Key One" = 9.876 # This line is indented
//!   ## Empty line
//!     Key2 = false # This line is indented twice
//! "#);
//! ```
//!
//! Here's how you would deal with the `ParseResult` and any errors
//!
//! ```
//! use tomllib::TOMLParser;
//! use tomllib::types::{Value, ParseResult, ParseError};
//!
//! let parser = TOMLParser::new();
//! let toml_doc = r#"
//! [[array_of_tables]]
//!   [array_of_tables.has_error]
//!   mixed_array = [5, true]
//! "#;
//! let (mut parser, result) = parser.parse(toml_doc);
//! // For brevity's sake we're only matching `FullError` `ParseResult`s and `MixedArray` `ParseError`s
//! match result {
//!    ParseResult::FullError(rrc_errors) => {
//!      println!("Parsed the full document, but with errors:");
//!      for error in rrc_errors.borrow().iter() {
//!        match error {
//!          &ParseError::MixedArray(ref key, ref line, ref column) => {
//!            println!("A mixed array with key {} was encountered on line {}, column {}.", key, line, column);
//!            assert_eq!("array_of_tables[0].has_error.mixed_array", *key);
//!            assert_eq!(4, *line);
//!            assert_eq!(0, *column); // column reporting is unimplemented so it will always be zero
//!          },
//!          _ => assert!(false),
//!        }
//!      }
//!    },
//!    _ => assert!(false),
//! }
//! ```
//!
//! Documentation and examples for specific types, enumeration values, and functions can be found in the `TOMLParser`
//! docs and the `types` module docs.

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::manual_strip)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::non_ascii_literal)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::pub_enum_variant_names)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]

#[macro_use]
extern crate nom;
extern crate regex;
#[macro_use]
extern crate log;
mod internals;
pub mod types;

use std::fmt;
use std::fmt::Display;
use crate::types::{ParseResult, Value, Children};
use crate::internals::parser::Parser;

/// A parser, manipulator, and outputter of TOML documents.
pub struct TOMLParser<'a> {
  parser: Parser<'a>,
}

impl<'a> TOMLParser<'a> {
  /// Constructs a new `TOMLParser`
  ///
  /// # Examples
  ///
  /// ```
  /// use tomllib::TOMLParser;
  ///
  /// let mut parser = TOMLParser::new();
  /// ```
  pub fn new() -> TOMLParser<'a> {
    TOMLParser{parser: Parser::new()}
  }

  /// Parses the string slice `input` as a TOML document. The method takes ownership of the parser and then returns it,
  /// along with the `ParseResult`, in a tuple.
  ///
  /// # Examples
  ///
  /// ```
  /// use tomllib::TOMLParser;
  ///
  /// let parser = TOMLParser::new();
  /// let (parser, result) = parser.parse("[table]\nAKey=\"A Value\"");
  /// ```
  pub fn parse(mut self, input: &'a str) -> (TOMLParser<'a>, ParseResult<'a>) {
    let (tmp, result) = self.parser.parse(input);
    self.parser = tmp;
    (self, result)
  }

  /// Given a string type `key`, returns the associated `Value` or `None` if the key doesn't exist in the parsed
  /// document.
  ///
  /// # Examples
  ///
  /// ```
  /// use tomllib::TOMLParser;
  /// use tomllib::types::Value;
  ///
  /// let parser = TOMLParser::new();
  /// let toml_doc = r#""A Key" = "A Value"
  /// [[tables]]
  /// SomeKey = 2010-05-18
  /// [tables.subtable]
  /// AnotherKey = 5
  /// "#;
  /// let (parser, result) = parser.parse(toml_doc);
  /// let value1 = parser.get_value("\"A Key\"");
  /// let value2 = parser.get_value("tables[0].SomeKey");
  /// let value3 = parser.get_value("tables[0].subtable.AnotherKey");
  /// assert_eq!(value1.unwrap(), Value::basic_string("A Value").unwrap());
  /// assert_eq!(value2.unwrap(), Value::date_from_int(2010, 5, 18).unwrap());
  /// assert_eq!(value3.unwrap(), Value::int(5));
  /// ```
  pub fn get_value<S>(self: &TOMLParser<'a>, key: S) -> Option<Value<'a>> where S: Into<String> {
    self.parser.get_value(key)
  }

  /// Given a string type `key` and a `Value` `val`, sets `Value` at `key` to `val` and returns true if `key` exists in
  /// the parsed document. If `key` doesn't exist in the parsed document returns false. Setting a value does not alter
  /// the document's format, including whitespace and comments, unless an `Array` or `InlineTable`'s structure is changed
  /// meaning either:
  ///
  /// * The amount of values in an `Array` is changed
  /// * The amount of key-value pairs in an `InlineTable` is changed
  /// * Any of the keys in an `InlineTable` is changed
  ///
  /// In these cases the `Array` or `InlineTable` will revert to default formatting: No whitespace after/before
  /// opening/closing braces, no whitespace before and one space after all commas, no comments on the same line as the
  /// `Array` or `InlineTable`, and one space before and after an equals sign in `InlineTable`s.
  ///
  /// # Examples
  ///
  /// ```
  /// #
  /// use tomllib::TOMLParser;
  /// use tomllib::types::Value;
  ///
  /// let parser = TOMLParser::new();
  /// let (mut parser, result) = parser.parse("[table]\nAKey=\"A Value\"");
  /// let success = parser.set_value("table.AKey", Value::Integer("5_000".into()));
  /// assert!(success);
  /// let value = parser.get_value("table.AKey");
  /// assert_eq!(value.unwrap(), Value::int_from_str("5_000").unwrap());
  /// ```
  pub fn set_value<S>(self: &mut TOMLParser<'a>, key: S, val: Value<'a>) -> bool where S: Into<String> {
    self.parser.set_value(key, val)
  }

  /// Given a string type `key` returns all the child keys of the `key` if it exists in the parsed document, otherwise
  /// returns `None`.
  ///
  /// # Examples
  ///
  /// ```
  /// use tomllib::TOMLParser;
  /// use tomllib::types::Children;
  /// use std::cell::{Cell, RefCell};
  ///
  /// let parser = TOMLParser::new();
  /// let toml_doc = r#"
  /// [table]
  /// "A Key" = "A Value"
  /// SomeKey = "Some Value"
  /// AnotherKey = 5
  /// [[array_of_tables]]
  /// [[array_of_tables]]
  /// [[array_of_tables]]
  /// "#;
  /// let (parser, result) = parser.parse(toml_doc);
  /// let table_child_keys = parser.get_children("table");
  /// assert_eq!(*table_child_keys.unwrap(), Children::Keys(RefCell::new(vec![
  ///   "\"A Key\"".to_string(), "SomeKey".to_string(), "AnotherKey".to_string()
  /// ])));
  /// let aot_child_keys = parser.get_children("array_of_tables");
  /// assert_eq!(*aot_child_keys.unwrap(), Children::Count(Cell::new(3)));
  /// ```
  pub fn get_children<S>(self: &TOMLParser<'a>, key: S) -> Option<&Children> where S: Into<String> {
    self.parser.get_children(key)
  }
}

impl<'a> Default for TOMLParser<'a> {
  fn default() -> Self {
    Self::new()
  }
}

/// Formats a parsed TOML document for display
///
/// # Examples
///
/// ```
/// use tomllib::TOMLParser;
/// use tomllib::types::Value;
///
/// let parser = TOMLParser::new();
/// let toml_doc = r#"
/// [table] # This is a comment
///   "A Key" = "A Value" # This line is indented
///     SomeKey = "Some Value" # This line is indented twice
/// "#;
/// let (mut parser, result) = parser.parse(toml_doc);
/// parser.set_value("table.\"A Key\"", Value::float(9.876));
/// parser.set_value("table.SomeKey", Value::bool(false));
/// assert_eq!(&format!("{}", parser), r#"
/// [table] # This is a comment
///   "A Key" = 9.876 # This line is indented
///     SomeKey = false # This line is indented twice
/// "#);
/// ```
impl<'a> Display for TOMLParser<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.parser)
  }
}
