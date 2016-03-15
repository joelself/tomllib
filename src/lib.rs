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
