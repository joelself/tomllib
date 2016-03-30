use std::collections::HashMap;
use std::hash::Hasher;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::error::Error;
use std::fmt::Display;
use std::str::FromStr;
use std::borrow::Cow;
use internals::parser::Parser;
use nom::IResult;

/// Conveys the result of a parse operation on a TOML document
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ParseResult<'a> {
    /// The entire input was parsed without error.
    Full,
    /// The entire input was parsed, but there were errors. Contains an `Rc<RefCell<Vec>>` of `ParseError`s.
    FullError(Rc<RefCell<Vec<ParseError<'a>>>>),
    /// Part of the input was parsed successfully without any errors. Contains a `Cow<str>`, with the leftover, unparsed
    /// input, the line number and column (currently column reporting is unimplemented and will always report `0`) where
    /// parsing stopped.
    Partial(Cow<'a, str>, usize, usize),
    /// Part of the input was parsed successfully with errors. Contains a `Cow<str>`, with the leftover, unparsed input,
    /// the line number and column (currently column reporting is unimplemented and will always report `0`) where parsing
    /// stopped, and an `Rc<RefCell<Vec>>` of `ParseError`s.
    PartialError(Cow<'a, str>, usize, usize, Rc<RefCell<Vec<ParseError<'a>>>>),
    /// The parser failed to parse any of the input as a complete TOML document. Contains the line number and column
    /// (currently column reporting is unimplemented and will always report `0`) where parsing stopped.
    Failure(usize, usize),
}

/// Represents a non-failure error encountered while parsing a TOML document.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ParseError<'a> {
    /// An `Array` containing different types was encountered. Contains the `String` key that points to the `Array` and
    /// the line number and column (currently column reporting is unimplemented and will always report `0`) where the
    /// `Array` was found. The `Array` can be retrieved and/or changed by its key using `TOMLParser::get_value` and
    /// `TOMLParser::set_value` methods.
    MixedArray(String, usize, usize),
    /// A duplicate key was encountered. Contains the `String` key that was duplicated in the document, the line number
    /// and column (currently column reporting is unimplemented and will always report `0`) where the duplicate key was
    /// found, and the `Value` that the key points to.
    DuplicateKey(String, usize, usize, Value<'a>),
    /// An invalid table was encountered. Either the key\[s\] that make up the table are invalid or a duplicate table was
    /// found. Contains the `String` key of the invalid table, the line number and column (currently column reporting is
    /// unimplemented and will always report `0`) where the invalid table was found, `RefCell<HashMap<String, Value>>`
    /// that contains all the keys and values belonging to that table.
    InvalidTable(String, usize, usize, RefCell<HashMap<String, Value<'a>>>),
    /// An invalid `DateTime` was encountered. This could be a `DateTime` with:
    ///
    /// * 0 for year
    /// * 0 for month or greater than 12 for month
    /// * 0 for day or greater than, 28, 29, 30, or 31 for day depending on the month and if the year is a leap year
    /// * Greater than 23 for hour
    /// * Greater than 59 for minute
    /// * Greater than 59 for second
    /// * Greater than 23 for offset hour
    /// * Greater than 59 for offset minute
    ///
    /// Contains the `String` key of the invalid `DateTime`, the line number and column (currently column reporting is
    /// unimplemented and will always report `0`) where the invalid `DateTime` was found, and a Cow<str> containing the
    /// invalid `DateTime` string.
    InvalidDateTime(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when an integer overflow is detected.
    IntegerOverflow(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when an integer underflow is detected.
    IntegerUnderflow(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when an invalid integer representation is detected.
    InvalidInteger(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when a float value of infinity is detected.
    Infinity(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when a float value of negative infinity is detected.
    NegativeInfinity(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when a float string conversion to an `f64` would result in a loss
    /// of precision.
    LossOfPrecision(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when an invalid float representation is detected.
    InvalidFloat(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when an invalid `true` or `false` string is detected.
    InvalidBoolean(String, usize, usize, Cow<'a, str>),
    /// *Currently unimplemented*. Reserved for future use when an invalid string representation is detected.
    InvalidString(String, usize, usize, Cow<'a, str>, StrType),
    /// *Currently unimplemented*. Reserved for future use when new error types are added without resorting to a breaking
    /// change.
    GenericError(String, usize, usize, Option<Cow<'a, str>>, String),
}

// Represents the 7 different types of values that can exist in a TOML document.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value<'a> {
    /// An integer value. Contains a `Cow<str>` representing the integer since integers can contain underscores.
    Integer(Cow<'a, str>),
    /// A float value. Contains a `Cow<str>` representing the float since floats can be formatted many different ways and
    /// can contain underscores.
    Float(Cow<'a, str>),
    /// A boolean value. Contains a `bool` value since only `true` and `false` are allowed.
    Boolean(bool),
    /// A `DateTime` value. Contains a `DateTime` struct that has a date and optionally a time, fractional seconds, and
    /// offset from UTC.
    DateTime(DateTime<'a>),
    /// A string value. Contains a `Cow<str>` with the string contents (without quotes) and `StrType` indicating whether
    /// the string is a basic string, multi-line basic string, literal string or multi-line literal string.
    String(Cow<'a, str>, StrType),
    /// An array value. Contains an `Rc<Vec>` of `Value`s contained in the `Array`.
    Array(Rc<Vec<Value<'a>>>),
    /// An inline table value. Contains an `Rc<Vec>` of tuples that contain a `Cow<str>` representing a key, and `Value`
    /// that the key points to.
    InlineTable(Rc<Vec<(Cow<'a, str>, Value<'a>)>>),
}

/// Represents the 4 different types of strings that are allowed in TOML documents.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum StrType {
    /// String is a basic string.
    Basic,
    /// String is a multi-line basic string.
    MLBasic,
    /// String is a literal string.
    Literal,
    /// String is a multi-line literal string.
    MLLiteral,
}

/// Represents the child keys of a key in a parsed TOML document.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Children {
    /// Contains a `Cell<usize>` with the amount of child keys the key has. The key has children that are indexed with an
    /// integer starting at 0. `Array`s and array of tables use integer for their child keys. For example:
    ///
    /// ```text
    /// Array = ["A", "B", "C", "D", "E"]
    /// [[array_of_table]]
    /// key = "val 1"
    /// [[array_of_table]]
    /// key = "val 2"
    /// [[array_of_table]]
    /// key = "val 3"
    /// ```
    ///
    /// "Array" has 5 children. The key of "D" is "Array[3]" because it is fourth element in "Array" and indexing starts
    /// at 0.
    /// "array_of_table" has 3 children. The key of "val 3" is "array_of_table[2].key" because it is in the third
    /// sub-table of "array_of_table" and indexing starts at 0.
    Count(Cell<usize>),
    /// Contains a `RefCell<Vec>` of `String`s with every sub-key of the key. The key has children that are indexed with a
    /// sub-key. Tables and inline-tables use sub-keys for their child keys. For example:
    ///
    /// ```text
    /// InlineTable = {subkey1 = "A", subkey2 = "B"}
    /// [table]
    /// a_key = "val 1"
    /// b_key = "val 2"
    /// c_key = "val 3"
    /// ```
    ///
    /// "InlineTable" has 2 children, "subkey1" and "subkey2". The key of "B" is "InlineTable.subkey2".
    /// "table" has 3 children, "a_key", "b_key", and "c_key". The key of "val 3" is "table.c_key".
    Keys(RefCell<Vec<String>>),
}

/// Contains convenience functions to combine base keys with child keys to make a full key.
impl Children {
    /// Combines string type `base_key` with a string type `child_key` to form a full key.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::TOMLParser;
    /// use tomllib::types::Children;
    /// let toml_doc = r#"
    /// [dependencies]
    /// nom = {version = "^1.2.0", features = ["regexp"]}
    /// regex = {version = "^0.1.48"}
    /// log = {version = "^0.3.5"}
    /// "#;
    /// let parser = TOMLParser::new();
    /// let (parser, result) = parser.parse(toml_doc);
    /// let deps = parser.get_children("dependencies");
    /// if let &Children::Keys(ref subkeys) = deps.unwrap() {
    ///   assert_eq!("dependencies.nom",
    ///     Children::combine_keys("dependencies", &subkeys.borrow()[0]));
    /// }
    /// ```
    pub fn combine_keys<S>(base_key: S, child_key: S) -> String
        where S: Into<String>
    {
        let mut full_key;
        let base = base_key.into();
        let child = child_key.into();
        if base != "" {
            full_key = base.clone();
            full_key.push('.');
            full_key.push_str(&child);
        } else {
            full_key = child.clone();
        }
        return full_key;
    }

    /// Combines string type `base_key` with an integer type `child_key` to form a full key.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::TOMLParser;
    /// use tomllib::types::Children;
    /// let toml_doc = r#"
    /// keywords = ["toml", "parser", "encode", "decode", "nom"]
    /// "#;
    /// let parser = TOMLParser::new();
    /// let (parser, result) = parser.parse(toml_doc);
    /// let kw = parser.get_children("keywords");
    /// if let &Children::Count(ref subkeys) = kw.unwrap() {
    ///   assert_eq!("keywords[4]", Children::combine_keys_index("keywords", subkeys.get() - 1));
    /// }
    /// # else {
    /// #  assert!(false, "{:?}", kw.unwrap());
    /// # }
    /// ```
    pub fn combine_keys_index<S>(base_key: S, child_key: usize) -> String
        where S: Into<String>
    {
        return format!("{}[{}]", base_key.into(), child_key);
    }

    /// Combines string type `base_key` with all subkeys of an instance of `Children` to form a `Vec` of full keys
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::TOMLParser;
    /// use tomllib::types::Children;
    /// let toml_doc = r#"
    /// keywords = ["toml", "parser"]
    /// numbers = {first = 1, second = 2}
    /// "#;
    /// let parser = TOMLParser::new();
    /// let (parser, result) = parser.parse(toml_doc);
    /// let kw = parser.get_children("keywords");
    /// assert_eq!(vec!["keywords[0]".to_string(), "keywords[1]".to_string()],
    ///   kw.unwrap().combine_child_keys("keywords"));
    /// let num = parser.get_children("numbers");
    /// assert_eq!(vec!["numbers.first".to_string(), "numbers.second".to_string()],
    ///   num.unwrap().combine_child_keys("numbers"));
    /// ```
    pub fn combine_child_keys<S>(&self, base_key: S) -> Vec<String>
        where S: Into<String>
    {
        let mut all_keys = vec![];
        let base = base_key.into();
        match self {
            &Children::Count(ref c) => {
                for i in 0..c.get() {
                    all_keys.push(format!("{}[{}]", base, i));
                }
            },
            &Children::Keys(ref hs_rc) => {
                for subkey in hs_rc.borrow().iter() {
                    if base != "" {
                        let mut full_key = base.clone();
                        full_key.push('.');
                        full_key.push_str(&subkey);
                        all_keys.push(full_key);
                    } else {
                        all_keys.push(subkey.clone());
                    }
                }
            },
        }
        return all_keys;
    }
}

/// Formats a `Value` for display. Uses default rust formatting for for `i64` for `Integer`s, `f64` for `Float`s, bool
/// for `Boolean`s. The default formatting for `Array`s and `InlineTable`s is No whitespace after/before
/// opening/closing braces, no whitespace before and one space after all commas, no comments on the same line as the 
/// `Array` or `InlineTable`, and one space before and after an equals sign in an `InlineTable`.
impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Integer(ref v) | &Value::Float(ref v) => write!(f, "{}", v),
            &Value::Boolean(ref b) => write!(f, "{}", b),
            &Value::DateTime(ref v) => write!(f, "{}", v),
            &Value::Array(ref arr) => {
                try!(write!(f, "["));
                for i in 0..arr.len() - 1 {
                    try!(write!(f, "{}, ", arr[i]));
                }
                if arr.len() > 0 {
                    try!(write!(f, "{}", arr[arr.len() - 1]));
                }
                write!(f, "]")
            },
            &Value::String(ref s, ref t) => {
                match t {
                    &StrType::Basic => write!(f, "\"{}\"", s),
                    &StrType::MLBasic => write!(f, "\"\"\"{}\"\"\"", s),
                    &StrType::Literal => write!(f, "'{}'", s),
                    &StrType::MLLiteral => write!(f, "'''{}'''", s),
                }
            },
            &Value::InlineTable(ref it) => {
                try!(write!(f, "{{"));
                for i in 0..it.len() - 1 {
                    try!(write!(f, "{} = {}, ", it[i].0, it[i].1));
                }
                if it.len() > 0 {
                    try!(write!(f, "{} = {}", it[it.len() - 1].0, it[it.len() - 1].1));
                }
                write!(f, "}}")
            },
        }
    }
}

impl<'a> Value<'a> {
    /// Convenience function for creating an `Value::Integer` from an `i64`. Cannot fail since `i64` maps directly onto
    /// TOML integers.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::Value;
    /// 
    /// assert_eq!(Value::Integer("100".into()), Value::int(100));
    /// ```
    pub fn int(int: i64) -> Value<'a> {
        Value::Integer(format!("{}", int).into())
    }

    /// Convenience function for creating an `Value::Integer` from an string type. Returns `Ok(Integer)` on success and
    /// `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::Value;
    /// 
    /// assert_eq!(Value::Integer("200".into()), Value::int_from_str("200").unwrap());
    /// ```
    pub fn int_from_str<S>(int: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        let result = Value::Integer(int.clone().into().into());
        if result.validate() {
            return Result::Ok(result);
        } else {
            return Result::Err(TOMLError::new(format!("Error parsing int. Argument: {}", int.into())));
        }
    }

    /// Convenience function for creating a `Value::Float` from a `f64`. Cannot fail since `f64` maps directly onto TOML
    /// floats.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::Value;
    /// 
    /// assert_eq!(Value::Float("300.3".into()), Value::float(300.3));
    /// ```
    pub fn float(float: f64) -> Value<'a> {
        Value::Float(format!("{}", float).into())
    }

    /// Convenience function for creating a `Value::Float` from an string type. Returns `Ok(Float)` on success and
    /// `Err(TOMLError)`
    /// on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::Value;
    /// 
    /// assert_eq!(Value::Float("400.4".into()), Value::float_from_str("400.4").unwrap());
    /// ```
    pub fn float_from_str<S>(float: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        let result = Value::Float(float.clone().into().into());
        if result.validate() {
            return Result::Ok(result);
        } else {
            return Result::Err(TOMLError::new(format!("Error parsing float. Argument: {}", float.into())));
        }
    }

    /// Convenience function for creating a `Value::Boolean` from a `bool`. Cannot fail since `bool` maps directly onto
    /// TOML booleans.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::Value;
    /// 
    /// assert_eq!(Value::Boolean(true), Value::bool(true));
    /// ```
    pub fn bool(b: bool) -> Value<'a> {
        Value::Boolean(b)
    }
    pub fn bool_from_str<S>(b: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        let lower = b.clone().into().to_lowercase();
        if lower == "true" {
            Result::Ok(Value::Boolean(true))
        } else if lower == "false" {
            Result::Ok(Value::Boolean(false))
        } else {
            return Result::Err(TOMLError::new(format!("Error parsing bool. Argument: {}", b.into())));
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing only a date from integer values. Returns
    /// `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2010", "04", "10").unwrap(), None)),
    ///   Value::date_from_int(2010, 4, 10).unwrap());
    /// ```
    pub fn date_from_int(year: usize, month: usize, day: usize) -> Result<Value<'a>, TOMLError> {
        let y = format!("{:0>4}", year);
        let m = format!("{:0>2}", month);
        let d = format!("{:0>2}", day);
        match Date::from_str(y, m, d) {
            Ok(date) => Ok(Value::DateTime(DateTime::new(date, None))),
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing only a date from string values. Returns
    /// `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2011", "05", "11").unwrap(), None)),
    ///   Value::date_from_str("2011", "05", "11").unwrap());
    /// ```
    pub fn date_from_str<S>(year: S, month: S, day: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        match Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()) {
            Ok(date) => Ok(Value::DateTime(DateTime::new(date, None))),
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time from integer values. Returns
    /// `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2010", "04", "10").unwrap(),
    ///   Some(Time::from_str("01", "02", "03", None, None).unwrap()))),
    ///   Value::datetime_from_int(2010, 4, 10, 1, 2, 3).unwrap());
    /// ```
    pub fn datetime_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize)
                             -> Result<Value<'a>, TOMLError> {
        let y = format!("{:0>4}", year);
        let m = format!("{:0>2}", month);
        let d = format!("{:0>2}", day);
        let h = format!("{:0>2}", hour);
        let min = format!("{:0>2}", minute);
        let s = format!("{:0>2}", second);
        match Date::from_str(y, m, d) {
            Ok(date) => {
                match Time::from_str(h, min, s, None, None) {
                    Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time from string values. Returns
    /// `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2011", "05", "11").unwrap(),
    ///   Some(Time::from_str("02", "03", "04", None, None).unwrap()))),
    ///   Value::datetime_from_str("2011", "05", "11", "02", "03", "04").unwrap());
    /// ```
    pub fn datetime_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        match Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()) {
            Ok(date) => {
                match Time::from_str(hour.clone().into(), minute.clone().into(), second.clone().into(), None, None) {
                    Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with fractional seconds from
    /// integer values. Returns `Ok(DateTime)` on success and `Err(TOMLError)` on failure. Note, you can't represent
    /// leading zeros on the fractional part this way for example: `2016-03-15T08:05:22.00055` is not possible using this
    /// function. 
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2010", "04", "10").unwrap(),
    ///   Some(Time::from_str("01", "02", "03", Some("5432".into()), None).unwrap()))),
    ///   Value::datetime_frac_from_int(2010, 4, 10, 1, 2, 3, 5432).unwrap());
    /// ```
    pub fn datetime_frac_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize,
                                  frac: usize)
                                  -> Result<Value<'a>, TOMLError> {
        let y = format!("{:0>4}", year);
        let m = format!("{:0>2}", month);
        let d = format!("{:0>2}", day);
        let h = format!("{:0>2}", hour);
        let min = format!("{:0>2}", minute);
        let s = format!("{:0>2}", second);
        let f = format!("{}", frac);
        match Date::from_str(y, m, d) {
            Ok(date) => {
                match Time::from_str(h, min, s, Some(f), None) {
                    Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with fractional seconds from
    /// string values. Returns `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2011", "05", "11").unwrap(),
    ///   Some(Time::from_str("02", "03", "04", Some("0043".into()), None).unwrap()))),
    ///   Value::datetime_frac_from_str("2011", "05", "11", "02", "03", "04", "0043").unwrap());
    /// ```
    pub fn datetime_frac_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S, frac: S)
                                     -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        match Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()) {
            Ok(date) => {
                match Time::from_str(hour.clone().into(),
                                     minute.clone().into(),
                                     second.clone().into(),
                                     Some(frac.clone().into()),
                                     None) {
                    Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with a timezone offset from UTC
    /// from integer values, except for the plus/minus sign which is passed as a char `'+'` or `'-'`. Returns
    /// `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset, TimeOffsetAmount};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2010", "04", "10").unwrap(),
    ///   Some(Time::from_str("01", "02", "03", None, Some(TimeOffset::Time(TimeOffsetAmount::from_str(
    ///     "+", "08", "00"
    ///   ).unwrap()))).unwrap()))),
    ///   Value::datetime_offset_from_int(2010, 4, 10, 1, 2, 3, '+', 8, 0).unwrap());
    /// ```
    pub fn datetime_offset_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize,
                                    posneg: char, off_hour: usize, off_minute: usize)
                                    -> Result<Value<'a>, TOMLError> {
        let y = format!("{:0>4}", year);
        let m = format!("{:0>2}", month);
        let d = format!("{:0>2}", day);
        let h = format!("{:0>2}", hour);
        let min = format!("{:0>2}", minute);
        let s = format!("{:0>2}", second);
        let oh = format!("{:0>2}", off_hour);
        let omin = format!("{:0>2}", off_minute);
        let mut pn = "".to_string();
        pn.push(posneg);
        match Date::from_str(y, m, d) {
            Ok(date) => {
                match TimeOffsetAmount::from_str(pn, oh, omin) {
                    Ok(offset) => {
                        match Time::from_str(h, min, s, None, Some(TimeOffset::Time(offset))) {
                            Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                            Err(error) => Err(error),
                        }
                    },
                    Err(error) => Result::Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with a timezone offset from UTC
    /// from string values. Returns `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset, TimeOffsetAmount};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2011", "05", "11").unwrap(),
    ///   Some(Time::from_str("02", "03", "04", None, Some(TimeOffset::Time(TimeOffsetAmount::from_str(
    ///     "+", "09", "30"
    ///   ).unwrap()))).unwrap()))),
    ///   Value::datetime_offset_from_str("2011", "05", "11", "02", "03", "04", "+", "09", "30").unwrap());
    /// ```
    pub fn datetime_offset_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S, posneg: S,
                                       off_hour: S, off_minute: S)
                                       -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        match Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()) {
            Ok(date) => {
                match TimeOffsetAmount::from_str(posneg.clone().into(),
                                                 off_hour.clone().into(),
                                                 off_minute.clone().into()) {
                    Ok(offset) => {
                        match Time::from_str(hour.clone().into(),
                                             minute.clone().into(),
                                             second.clone().into(),
                                             None,
                                             Some(TimeOffset::Time(offset))) {
                            Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                            Err(error) => Err(error),
                        }
                    },
                    Err(error) => Result::Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with a timezone of Zulu from
    /// integer values. Returns Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2010", "04", "10").unwrap(),
    ///   Some(Time::from_str("01", "02", "03", None, Some(TimeOffset::Zulu)).unwrap()))),
    ///   Value::datetime_zulu_from_int(2010, 4, 10, 1, 2, 3).unwrap());
    /// ```
    pub fn datetime_zulu_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize)
                                  -> Result<Value<'a>, TOMLError> {
        let y = format!("{:0>4}", year);
        let m = format!("{:0>2}", month);
        let d = format!("{:0>2}", day);
        let h = format!("{:0>2}", hour);
        let min = format!("{:0>2}", minute);
        let s = format!("{:0>2}", second);
        match Date::from_str(y, m, d) {
            Ok(date) => {
                match Time::from_str(h, min, s, None, Some(TimeOffset::Zulu)) {
                    Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with a timezone of Zulu from
    /// string values. Returns `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2011", "05", "11").unwrap(),
    ///   Some(Time::from_str("02", "03", "04", None, Some(TimeOffset::Zulu)).unwrap()))),
    ///   Value::datetime_zulu_from_str("2011", "05", "11", "02", "03", "04").unwrap());
    /// ```
    pub fn datetime_zulu_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S)
                                     -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        match Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()) {
            Ok(date) => {
                match Time::from_str(hour.clone().into(),
                                     minute.clone().into(),
                                     second.clone().into(),
                                     None,
                                     Some(TimeOffset::Zulu)) {
                    Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with fractional seconds and a
    /// timezone of Zulu from integer values, except for the plus/minus sign which is passed as a string value `"+"` or 
    /// "-"`. Returns `Ok(DateTime)` on success and `Err(TOMLError)` on failure. Note, you can't represent leading zeros
    /// on the fractional part this way for example: `2016-03-15T08:05:22.00055Z` is not possible using this function.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2010", "04", "10").unwrap(),
    ///   Some(Time::from_str("01", "02", "03", Some("5678".into()), Some(TimeOffset::Zulu)).unwrap()))),
    ///   Value::datetime_full_zulu_from_int(2010, 4, 10, 1, 2, 3, 5678).unwrap());
    /// ```
    pub fn datetime_full_zulu_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize,
                                       second: usize, frac: u64)
                                       -> Result<Value<'a>, TOMLError> {
        let y = format!("{:0>4}", year);
        let m = format!("{:0>2}", month);
        let d = format!("{:0>2}", day);
        let h = format!("{:0>2}", hour);
        let min = format!("{:0>2}", minute);
        let s = format!("{:0>2}", second);
        let f = format!("{}", frac);
        match Date::from_str(y, m, d) {
            Ok(date) => {
                match Time::from_str(h, min, s, Some(f), Some(TimeOffset::Zulu)) {
                    Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with fractional seconds and a
    /// timezone of Zulu from string values. Returns `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2011", "05", "11").unwrap(),
    ///   Some(Time::from_str("02", "03", "04", None, Some(TimeOffset::Zulu)).unwrap()))),
    ///   Value::datetime_zulu_from_str("2011", "05", "11", "02", "03", "04").unwrap());
    /// ```
    pub fn datetime_full_zulu_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S, frac: S)
                                          -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        match Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()) {
            Ok(date) => {
                match Time::from_str(hour.clone().into(),
                                     minute.clone().into(),
                                     second.clone().into(),
                                     Some(frac.clone().into()),
                                     Some(TimeOffset::Zulu)) {
                    Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with fractional seconds and a
    /// timezone offset from UTC from integer values, except for the plus/minus sign which is passed as a char `"+"` or
    /// `"-"`. Returns `Ok(DateTime)` on success and `Err(TOMLError)` on failure. Note, you can't represent
    /// leading zeros on the fractional part this way for example: `2016-03-15T08:05:22.00055-11:00` is not possible using
    /// this function. 
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset, TimeOffsetAmount};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2010", "04", "10").unwrap(),
    ///   Some(Time::from_str("01", "02", "03", Some("135".into()), Some(TimeOffset::Time(TimeOffsetAmount::from_str(
    ///     "-", "11", "00"
    ///   ).unwrap()))).unwrap()))),
    ///   Value::datetime_full_from_int(2010, 4, 10, 1, 2, 3, 135, '-', 11, 0).unwrap());
    /// ```
    pub fn datetime_full_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize,
                                  frac: u64, posneg: char, off_hour: usize, off_minute: usize)
                                  -> Result<Value<'a>, TOMLError> {
        let y = format!("{:0>4}", year);
        let m = format!("{:0>2}", month);
        let d = format!("{:0>2}", day);
        let h = format!("{:0>2}", hour);
        let min = format!("{:0>2}", minute);
        let s = format!("{:0>2}", second);
        let f = format!("{}", frac);
        let oh = format!("{:0>2}", off_hour);
        let omin = format!("{:0>2}", off_minute);
        let mut pn = "".to_string();
        pn.push(posneg);
        match Date::from_str(y, m, d) {
            Ok(date) => {
                match TimeOffsetAmount::from_str(pn, oh, omin) {
                    Ok(offset) => {
                        match Time::from_str(h, min, s, Some(f), Some(TimeOffset::Time(offset))) {
                            Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                            Err(error) => Err(error),
                        }
                    },
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` containing a date and time with fractional seconds and a
    /// timezone offset from UTC from string values. Returns `Ok(DateTime)` on success and `Err(TOMLError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset, TimeOffsetAmount};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2011", "05", "11").unwrap(),
    ///   Some(Time::from_str("02", "03", "04", Some("0864".into()), Some(TimeOffset::Time(TimeOffsetAmount::from_str(
    ///     "+", "09", "30"
    ///   ).unwrap()))).unwrap()))),
    ///   Value::datetime_full_from_str("2011", "05", "11", "02", "03", "04", "0864","+", "09", "30").unwrap());
    /// ```
    pub fn datetime_full_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S, frac: S, posneg: S,
                                     off_hour: S, off_minute: S)
                                     -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        match Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()) {
            Ok(date) => {
                match TimeOffsetAmount::from_str(posneg.clone().into(),
                                                 off_hour.clone().into(),
                                                 off_minute.clone().into()) {
                    Ok(offset) => {
                        match Time::from_str(hour.clone().into(),
                                             minute.clone().into(),
                                             second.clone().into(),
                                             Some(frac.clone().into()),
                                             Some(TimeOffset::Time(offset))) {
                            Ok(time) => Ok(Value::DateTime(DateTime::new(date, Some(time)))),
                            Err(error) => Err(error),
                        }
                    },
                    Err(error) => Err(error),
                }
            },
            Err(error) => Err(error),
        }
    }

    /// Convenience function for creating a `Value::DateTime` from a sinle string value.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, DateTime, Date, Time, TimeOffset, TimeOffsetAmount};
    /// 
    /// assert_eq!(Value::DateTime(DateTime::new(Date::from_str("2012", "06", "12").unwrap(),
    ///   Some(Time::from_str("02", "03", "04", Some("0864".into()), Some(TimeOffset::Time(TimeOffsetAmount::from_str(
    ///     "+", "10", "30"
    ///   ).unwrap()))).unwrap()))),
    ///   Value::datetime_parse("2012-06-12T02:03:04.0864+10:30").unwrap());
    /// ```
    pub fn datetime_parse<S>(dt: S) -> Result<Value<'a>, TOMLError>
        where S: Into<&'a str>
    {
        let datetime = dt.into();
        let p = Parser::new();
        match p.date_time(datetime) {
            (_, IResult::Done(i, o)) => {
                let result = Value::DateTime(o);
                if i.len() > 0 || !result.validate() {
                    return Result::Err(TOMLError::new(format!("Error parsing string as datetime. Argument: {}",
                                                              datetime)));
                } else {
                    return Result::Ok(result);
                }
            },
            (_, _) => {
                return Result::Err(TOMLError::new(format!("Error parsing string as datetime. Argument: {}", datetime)))
            },
        }
    }

    /// Convenience function for creating a `Value::String` with `StrType::Basic`. Returns Ok() on success and Err() on
    /// failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, StrType};
    /// 
    /// assert_eq!(Value::String("foobar".into(), StrType::Basic), Value::basic_string("foobar").unwrap());
    /// ```
    pub fn basic_string<S>(s: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        let result = Value::String(s.clone().into().into(), StrType::Basic);
        if result.validate() {
            return Result::Ok(result);
        } else {
            return Result::Err(TOMLError::new(format!("Error parsing string as basic_string. Argument: {}", s.into())));
        }
    }

    /// Convenience function for creating a `Value::String` with `StrType::MLBasic`. Returns Ok() on success and Err() on
    /// failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, StrType};
    /// 
    /// assert_eq!(Value::String("foo\nbar".into(), StrType::MLBasic), Value::ml_basic_string("foo\nbar").unwrap());
    /// ```
    pub fn ml_basic_string<S>(s: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        let result = Value::String(s.clone().into().into(), StrType::MLBasic);
        if result.validate() {
            return Result::Ok(result);
        } else {
            return Result::Err(TOMLError::new(format!("Error parsing string as ml_basic_string. Argument: {}",
                                                      s.into())));
        }
    }

    /// Convenience function for creating a `Value::String` with `StrType::Literal`. Returns Ok() on success and Err() on
    /// failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, StrType};
    /// 
    /// assert_eq!(Value::String("\"foobar\"".into(), StrType::Literal), Value::literal_string("\"foobar\"").unwrap());
    /// ```
    pub fn literal_string<S>(s: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        let result = Value::String(s.clone().into().into(), StrType::Literal);
        if result.validate() {
            return Result::Ok(result);
        } else {
            return Result::Err(TOMLError::new(format!("Error parsing string as literal_string. Argument: {}",
                                                      s.into())));
        }
    }

    /// Convenience function for creating a `Value::String` with `StrType::MLLiteral`. Returns Ok() on success and Err()
    /// on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value, StrType};
    /// 
    /// assert_eq!(Value::String("\"foo\nbar\"".into(), StrType::MLLiteral),
    ///   Value::ml_literal_string("\"foo\nbar\"").unwrap());
    /// ```
    pub fn ml_literal_string<S>(s: S) -> Result<Value<'a>, TOMLError>
        where S: Into<String> + Clone
    {
        let result = Value::String(s.clone().into().into(), StrType::MLLiteral);
        if result.validate() {
            return Result::Ok(result);
        } else {
            return Result::Err(TOMLError::new(format!("Error parsing string as ml_literal_string. Argument: {}",
                                                      s.into())));
        }
    }

    /// Parses and validates a `Value`, returns true if the value is valid and false if it is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{Value};
    ///
    /// assert!(!Value::Integer("_989_721_".into()).validate()); // Integers may have underscores but they must be
    ///                                                           // surrounded by digits
    /// assert!(Value::Float("7.62".into()).validate());
    /// ```
    pub fn validate(&self) -> bool {
        match self {
            &Value::Integer(ref s) => {
                let p = Parser::new();
                match p.integer(s) {
                    (_, IResult::Done(_, _)) => true,
                    (_, _) => false,
                }
            },
            &Value::Float(ref s) => {
                let p = Parser::new();
                match p.float(s) {
                    (_, IResult::Done(_, _)) => true,
                    (_, _) => false,
                }
            },
            &Value::DateTime(ref dt) => dt.validate(),
            &Value::String(ref s, st) => {
                match st {
                    StrType::Basic => {
                        match Parser::quoteless_basic_string(s) {
                            IResult::Done(i, _) => i.len() == 0,
                            _ => false,
                        }
                    },
                    StrType::MLBasic => {
                        match Parser::quoteless_ml_basic_string(s) {
                            IResult::Done(i, _) => i.len() == 0,
                            _ => false,
                        }
                    },
                    StrType::Literal => {
                        match Parser::quoteless_literal_string(s) {
                            IResult::Done(i, _) => i.len() == 0,
                            _ => false,
                        }
                    },
                    StrType::MLLiteral => {
                        match Parser::quoteless_ml_literal_string(s) {
                            IResult::Done(i, _) => i.len() == 0,
                            _ => false,
                        }
                    },
                }
            },
            _ => true,
        }
    }
}

/// Error type returned by `Value` creation convenience functions on invalid input.
#[derive(Debug)]
pub struct TOMLError {
    message: String,
}

impl Error for TOMLError {
    /// Gives a description of the error encountered when validating input to a `Value` creation function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use tomllib::types::Value;
    ///
    /// if let Err(toml_err) = Value::basic_string("foo\n") {
    ///   println!("{}", toml_err.description()); 
    /// }
    /// # else {
    /// #   assert!(false);
    /// # }
    /// ```
    fn description(&self) -> &str {
        &self.message
    }

    /// Returns an `Error` that caused the current `Error`. Always returns `None`.
    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl Display for TOMLError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl TOMLError {
    fn new(msg: String) -> TOMLError {
        warn!("{}", msg);
        TOMLError { message: msg }
    }
}

/// Represents a plus sign or minus sign for positive and negative timezone offsets.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PosNeg {
    /// A plus sign representing a positive timezone offset.
    Pos,
    /// A minus sign representing a negaive timezone offset.
    Neg,
}

impl Display for PosNeg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &PosNeg::Pos => write!(f, "+"),
            &PosNeg::Neg => write!(f, "-"),
        }

    }
}

/// Represents either a timezone of Zulu or or hour plus minute timezone offset from UTC.
#[derive(Debug, Eq, Clone)]
pub enum TimeOffset<'a> {
    // Timezone [Zulu](https://en.wikipedia.org/wiki/List_of_military_time_zones), also known as Greenwich Mean Time
    // or
    // Coordinated Universal Time (UTC).
    Zulu,
    // Contains a `TimeOffsetAmount` with the hours and minutes offset from UTC.
    Time(TimeOffsetAmount<'a>),
}

impl<'a> PartialEq for TimeOffset<'a> {
    fn eq(&self, other: &TimeOffset<'a>) -> bool {
        match (self, other) {
            (&TimeOffset::Zulu, &TimeOffset::Zulu) => true,
            (&TimeOffset::Time(ref i), &TimeOffset::Time(ref j)) if (i == j) => true,
            _ => false,
        }
    }
}

impl<'a> Display for TimeOffset<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &TimeOffset::Zulu => write!(f, "Z"),
            &TimeOffset::Time(ref t) => write!(f, "{}", t),
        }
    }
}

impl<'a> TimeOffset<'a> {
    pub fn validate(&self) -> bool {
        match self {
            &TimeOffset::Zulu => return true,
            &TimeOffset::Time(ref amount) => return amount.validate(),
        }
    }
}

/// A positive or negative amount of hours and minutes offset from UTC.
#[derive(Debug, Eq, Clone)]
pub struct TimeOffsetAmount<'a> {
    /// Represents whether the offset is positive or negative.
    pub pos_neg: PosNeg,
    /// Represents the number of hours that time is offset from UTC.Must be 2 decimal digits between 0 23 inclusive.
    pub hour: Cow<'a, str>,
    /// Represents the number of minutes that time is offset from UTC. Must be 2 decimal digits between 0 59 inclusive.
    pub minute: Cow<'a, str>,
}

impl<'a> PartialEq for TimeOffsetAmount<'a> {
    fn eq(&self, other: &TimeOffsetAmount<'a>) -> bool {
        self.pos_neg == other.pos_neg && self.hour == other.hour && self.minute == other.minute
    }
}

impl<'a> Display for TimeOffsetAmount<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}:{}", self.pos_neg, &self.hour, &self.minute)
    }
}

impl<'a> TimeOffsetAmount<'a> {
    /// Create a new `TimeOffsetAmount` from string type values. Returns `Ok()` on success and `Err()` on failure.
    ///
    /// # Examples
    /// ```
    /// use tomllib::types::TimeOffsetAmount;
    ///
    /// let offset = TimeOffsetAmount::from_str("-", "04", "00").unwrap();
    /// ```
    pub fn from_str<S>(pos_neg: S, hour: S, minute: S) -> Result<TimeOffsetAmount<'a>, TOMLError>
        where S: Into<String>
    {
        let pn = match pos_neg.into().as_ref() {
            "+" => PosNeg::Pos,
            "-" => PosNeg::Neg,
            _ => return Result::Err(TOMLError::new("pos_neg value is neither a '+' or a '-'.".to_string())),
        };
        let offset = TimeOffsetAmount {
            pos_neg: pn,
            hour: hour.into().into(),
            minute: minute.into().into(),
        };
        if offset.validate() {
            return Result::Ok(offset);
        } else {
            return Result::Err(TOMLError::new("Error validating TimeOffsetAmount.".to_string()));
        }
    }

    /// Validates a created `TimeOffsetAmount`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{TimeOffsetAmount, PosNeg};
    ///
    /// let offset_wrong = TimeOffsetAmount{pos_neg: PosNeg::Pos, hour: "31".into(), minute: "30".into()};
    /// let offset_right = TimeOffsetAmount{pos_neg: PosNeg::Pos, hour: "07".into(), minute: "00".into()};
    /// assert!(!offset_wrong.validate());
    /// assert!(offset_right.validate());
    /// ```
    pub fn validate(&self) -> bool {
        if self.hour.len() != 2 || self.minute.len() != 2 {
            return false;
        }
        return self.validate_numbers();
    }

    fn validate_numbers(&self) -> bool {
        if let Ok(h) = usize::from_str(&self.hour) {
            if h > 23 {
                return false;
            }
        } else {
            return false;
        }
        if let Ok(m) = usize::from_str(&self.minute) {
            if m > 59 {
                return false;
            }
        } else {
            return false;
        }
        return true;
    }
}

/// Represents a date value.
// <year>-<month>-<day>
#[derive(Debug, Eq, Clone)]
pub struct Date<'a> {
    /// Represents the year of a date. Must be 4 decimal digits greater than 0".
    pub year: Cow<'a, str>,
    /// Represents the month of a date. Must be 2 decimal digits greater than 0 less than 13.
    pub month: Cow<'a, str>,
    /// Represents the day of a date. Must be 2 decimal digits greater than 0less than 28, 29, 30, or 31 depending on the
    /// month and whether the year is a leap year.
    pub day: Cow<'a, str>,
}

impl<'a> PartialEq for Date<'a> {
    fn eq(&self, other: &Date<'a>) -> bool {
        self.year == other.year && self.month == other.month && self.day == other.day
    }
}

impl<'a> Display for Date<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}-{}", self.year, self.month, self.day)
    }
}

impl<'a> Date<'a> {
    /// Create a new `Date` from string type values. Returns `Ok()` on success and `Err()` on failure.
    ///
    /// # Examples
    /// ```
    /// use tomllib::types::Date;
    ///
    /// let date = Date::from_str("1991", "09", "23").unwrap();
    /// ```
    pub fn from_str<S>(year: S, month: S, day: S) -> Result<Date<'a>, TOMLError>
        where S: Into<String>
    {
        let date = Date {
            year: year.into().into(),
            month: month.into().into(),
            day: day.into().into(),
        };
        if date.validate() {
            Ok(date)
        } else {
            Err(TOMLError::new("Error validating Date.".to_string()))
        }
    }

    /// Validates a created `Date`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::Date;
    ///
    /// let date_wrong = Date{year: "76563".into(), month: "10".into(), day: "20".into()};
    /// let date_right = Date{year: "1763".into(), month: "10".into(), day: "20".into()};
    /// assert!(!date_wrong.validate());
    /// assert!(date_right.validate());
    /// ```
    pub fn validate(&self) -> bool {
        if self.year.len() != 4 || self.month.len() != 2 || self.day.len() != 2 {
            return false;
        }
        return self.validate_numbers();
    }

    fn validate_numbers(&self) -> bool {
        if let Ok(y) = usize::from_str(&self.year) {
            if y == 0 || y > 9999 {
                return false;
            }
            if let Ok(m) = usize::from_str(&self.month) {
                if m < 1 || m > 12 {
                    return false;
                }
                if let Ok(d) = usize::from_str(&self.day) {
                    if d < 1 {
                        return false;
                    }
                    match m {
                        2 => {
                            let leap_year;
                            if y % 4 != 0 {
                                leap_year = false;
                            } else if y % 100 != 0 {
                                leap_year = true;
                            } else if y % 400 != 0 {
                                leap_year = false;
                            } else {
                                leap_year = true;
                            }
                            if leap_year && d > 29 {
                                return false;
                            } else if !leap_year && d > 28 {
                                return false;
                            }
                        },
                        1 | 3 | 5 | 7 | 8 | 10 | 12 => {
                            if d > 31 {
                                return false;
                            }
                        },
                        _ => {
                            if d > 30 {
                                return false;
                            }
                        },
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
        return true;
    }
}

/// Represents the time part of a `DateTime` including optional fractional seconds and timezone offset.
#[derive(Debug, Eq, Clone)]
pub struct Time<'a> {
    /// Represents the hour of the time. Must be 2 decimal digits between 0 and 23 inclusive.
    pub hour: Cow<'a, str>,
    /// Represents the minute of the time. Must be 2 decimal digits between 0 and 59 inclusive.
    pub minute: Cow<'a, str>,
    /// Represent the second of the time. Must be 2 decimal digits between 0 and 59 inclusive.
    pub second: Cow<'a, str>,
    /// Optional fraction of a second of the time. Can be an arbitrary number of decimal digits.
    pub fraction: Option<Cow<'a, str>>,
    /// Optional time zone offset.
    pub offset: Option<TimeOffset<'a>>,
}

impl<'a> PartialEq for Time<'a> {
    fn eq(&self, other: &Time<'a>) -> bool {
        self.hour == other.hour && self.minute == other.minute && self.second == other.second &&
        self.fraction == other.fraction && self.offset == other.offset
    }
}

impl<'a> Display for Time<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.fraction, &self.offset) {
            (&Some(ref frac), &Some(ref offset)) => {
                write!(f, "T{}:{}:{}.{}{}", self.hour, self.minute, self.second, frac, offset)
            },
            (&Some(ref frac), &None) => write!(f, "T{}:{}:{}.{}", self.hour, self.minute, self.second, frac),
            (&None, &Some(ref offset)) => write!(f, "T{}:{}:{}{}", self.hour, self.minute, self.second, offset),
            (&None, &None) => write!(f, "T{}:{}:{}", self.hour, self.minute, self.second),
        }
    }
}

impl<'a> Time<'a> {
    /// Create a new `Time` from string type values. Returns `Ok()` on success and `Err()` on failure.
    ///
    /// # Examples
    /// ```
    /// use tomllib::types::Time;
    ///
    /// let time = Time::from_str("19", "33", "02", None, None).unwrap();
    /// ```
    pub fn from_str<S>(hour: S, minute: S, second: S, fraction: Option<S>, offset: Option<TimeOffset<'a>>)
                       -> Result<Time<'a>, TOMLError>
        where S: Into<String>
    {
        if let Some(s) = fraction {
            let time = Time {
                hour: hour.into().into(),
                minute: minute.into().into(),
                second: second.into().into(),
                fraction: Some(s.into().into()),
                offset: offset,
            };
            if time.validate() {
                return Ok(time);
            } else {
                return Err(TOMLError::new("Error validating Time.".to_string()));
            }
        } else {
            let time = Time {
                hour: hour.into().into(),
                minute: minute.into().into(),
                second: second.into().into(),
                fraction: None,
                offset: offset,
            };
            if time.validate() {
                return Ok(time);
            } else {
                return Err(TOMLError::new("Error validating Time.".to_string()));
            }
        }
    }

    /// Validates a created `Time`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::Time;
    ///
    /// let time_wrong = Time{hour: "23".into(), minute: "79".into(), second: "20".into(),
    ///   fraction: None, offset: None};
    /// let time_right = Time{hour: "11".into(), minute: "53".into(), second: "25".into(),
    ///   fraction: None, offset: None};
    /// assert!(!time_wrong.validate());
    /// assert!(time_right.validate());
    /// ```
    pub fn validate(&self) -> bool {
        if self.hour.len() != 2 || self.minute.len() != 2 || self.second.len() != 2 {
            return false;
        }
        return self.validate_numbers();
    }

    fn validate_numbers(&self) -> bool {
        if let Ok(h) = usize::from_str(&self.hour) {
            if h > 23 {
                return false;
            }
        } else {
            return false;
        }
        if let Ok(m) = usize::from_str(&self.minute) {
            if m > 59 {
                return false;
            }
        } else {
            return false;
        }
        if let Ok(s) = usize::from_str(&self.second) {
            if s > 59 {
                return false;
            }
        } else {
            return false;
        }
        if let Some(ref frac) = self.fraction {
            if u64::from_str(frac).is_err() {
                return false;
            }
        }
        if let Some(ref off) = self.offset {
            if !off.validate() {
                return false;
            }
        }
        return true;
    }
}

/// Represents a`DateTime` including the `Date` and optional `Time`
#[derive(Debug, Eq, Clone)]
pub struct DateTime<'a> {
    pub date: Date<'a>,
    pub time: Option<Time<'a>>,
}

impl<'a> PartialEq for DateTime<'a> {
    fn eq(&self, other: &DateTime<'a>) -> bool {
        self.date == other.date && self.time == other.time
    }
}

impl<'a> Display for DateTime<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.time {
            &Some(ref time) => write!(f, "{}{}", self.date, time),
            &None => write!(f, "{}", self.date),
        }
    }
}

// <hour>:<minute>:<second>(.<fraction>)?
impl<'a> DateTime<'a> {
    pub fn new(date: Date<'a>, time: Option<Time<'a>>) -> DateTime<'a> {
        DateTime {
            date: date,
            time: time,
        }
    }

    /// Validates a created `DateTime`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tomllib::types::{DateTime, Date};
    ///
    /// let dt_wrong = DateTime{ date: Date{ year: "53456".into(), month: "06".into(), day: "20".into() }, time: None};
    /// let dt_right = DateTime{ date: Date{ year: "1995".into(), month: "09".into(), day: "13".into() }, time: None};
    /// assert!(!dt_wrong.validate());
    /// assert!(dt_right.validate());
    /// ```
    pub fn validate(&self) -> bool {
        if self.date.validate() {
            if let Some(ref time) = self.time {
                return time.validate();
            }
        } else {
            return false;
        }
        return true;
    }
}

#[cfg(test)]
mod test {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use types::{Children, Value, Date, Time, DateTime, TimeOffset, TimeOffsetAmount, StrType};

    #[test]
    fn test_combine_keys() {
        assert_eq!("foo.bar.baz".to_string(), Children::combine_keys("foo.bar", "baz"));
    }

    #[test]
    fn test_combine_keys_index() {
        assert_eq!("foo.bar[9]".to_string(), Children::combine_keys_index("foo.bar", 9));
    }

    #[test]
    fn test_combine_child_keys() {
        let kids = Children::Keys(RefCell::new(vec!["baz".to_string(),
                                                    "qux".to_string(),
                                                    "plugh".to_string(),
                                                    "thud".to_string()]));
        assert_eq!(vec!["foo.bar.baz".to_string(),
                        "foo.bar.qux".to_string(),
                        "foo.bar.plugh".to_string(),
                        "foo.bar.thud".to_string()],
                   kids.combine_child_keys("foo.bar".to_string()));
    }

    #[test]
    fn test_combine_child_keys_empty_base() {
        let kids = Children::Keys(RefCell::new(vec!["baz".to_string(),
                                                    "qux".to_string(),
                                                    "plugh".to_string(),
                                                    "thud".to_string()]));
        assert_eq!(vec!["baz".to_string(), "qux".to_string(), "plugh".to_string(), "thud".to_string()],
                   kids.combine_child_keys("".to_string()));
    }

    #[test]
    fn test_combine_child_keys_index() {
        let kids = Children::Count(Cell::new(3));
        assert_eq!(vec!["foo.bar[0]".to_string(), "foo.bar[1]".to_string(), "foo.bar[2]".to_string()],
                   kids.combine_child_keys("foo.bar".to_string()));
    }

    #[test]
    fn test_value_display() {
        let val_int = Value::Integer("7778877".into());
        let val_float = Value::Float("1929.345".into());
        let val_true = Value::Boolean(true);
        let val_false = Value::Boolean(false);
        let val_datetime =
            Value::DateTime(DateTime::new(Date::new_str("9999", "12", "31"),
                                          Some(Time::new_str("23",
                                                             "59",
                                                             "59",
                                                             Some("9999999"),
                                                             Some(TimeOffset::Time(TimeOffsetAmount::new_str("-",
                                                                                                             "0\
                                                                                                              0",
                                                                                                             "0\
                                                                                                              0")))))));
        let val_basic_str = Value::String("foobar1".into(), StrType::Basic);
        let val_literal_str = Value::String("foobar2".into(), StrType::Literal);
        let val_ml_basic_str = Value::String("foobar3".into(), StrType::MLBasic);
        let val_ml_literal_str = Value::String("foobar4".into(), StrType::MLLiteral);
        let val_array = Value::Array(Rc::new(vec![Value::Integer("3000".into()),
                                                  Value::Array(Rc::new(vec![Value::Integer("40000".into()),
                                                                            Value::Float("50.5".into())])),
                                                  Value::String("barbaz".into(), StrType::Literal)]));
        let val_inline_table = Value::InlineTable(Rc::new(vec![("foo".into(), Value::Boolean(true)),
                                                               ("bar".into(),
                                                                Value::InlineTable(Rc::new(vec![
        ("baz".into(), Value::Boolean(false)), ("qux".into(), Value::Integer("2016".into())),
      ]))),
                                                               ("plugh".into(), Value::Float("3333.444".into()))]));

        assert_eq!("7778877", &format!("{}", val_int));
        assert_eq!("1929.345", &format!("{}", val_float));
        assert_eq!("true", &format!("{}", val_true));
        assert_eq!("false", &format!("{}", val_false));
        assert_eq!("9999-12-31T23:59:59.9999999-00:00", &format!("{}", val_datetime));
        assert_eq!("\"foobar1\"", &format!("{}", val_basic_str));
        assert_eq!("'foobar2'", &format!("{}", val_literal_str));
        assert_eq!("\"\"\"foobar3\"\"\"", &format!("{}", val_ml_basic_str));
        assert_eq!("'''foobar4'''", &format!("{}", val_ml_literal_str));
        assert_eq!("[3000, [40000, 50.5], 'barbaz']", &format!("{}", val_array));
        assert_eq!("{foo = true, bar = {baz = false, qux = 2016}, plugh = 3333.444}", &format!("{}", val_inline_table));
    }

    #[test]
    fn test_create_int() {
        assert_eq!(Value::Integer("9223372036854775807".into()), Value::int(9223372036854775807));
    }

    #[test]
    fn test_create_int_from_str() {
        assert_eq!(Value::Integer("-9223372036854775808".into()), Value::int_from_str("-9223372036854775808").unwrap());
    }

    #[test]
    fn test_create_int_from_str_fail() {
        assert!(Value::int_from_str("q-9223$37(203)[]M807").is_err());
    }

    #[test]
    fn test_create_float() {
        assert_eq!(Value::Float("17976900000000000000000000000000000000000000000000000000000000000000000000000000000\
                                 00000000000000000000000000000000000000000000000000000000000000000000000000000000000\
                                 00000000000000000000000000000000000000000000000000000000000000000000000000000000000\
                                 000000000000000000000000000000000000000000000000000000000000"
                                    .into()),
                   Value::float(1.79769e+308));
    }

    #[test]
    fn test_create_float_from_str() {
        assert_eq!(Value::Float("2.22507e-308".into()), Value::float_from_str("2.22507e-308").unwrap());
    }

    #[test]
    fn test_create_float_from_str_fail() {
        assert!(Value::float_from_str("q2.3e++10eipi").is_err());
    }

    #[test]
    fn test_create_bool() {
        assert_eq!(Value::Boolean(false), Value::bool(false));
    }

    #[test]
    fn test_create_bool_from_str() {
        assert_eq!(Value::Boolean(true), Value::bool_from_str("TrUe").unwrap());
    }

    #[test]
    fn test_create_bool_from_str_fail() {
        assert!(Value::bool_from_str("TFraulese").is_err());
    }

    #[test]
    fn test_create_date_from_int() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), None)),
                   Value::date_from_int(2012, 1, 3).unwrap());
    }

    #[test]
    fn test_create_date_from_int_fail() {
        assert!(Value::date_from_int(0, 2, 20).is_err());
        assert!(Value::date_from_int(2016, 0, 20).is_err());
        assert!(Value::date_from_int(2016, 1, 0).is_err());
        assert!(Value::date_from_int(2016, 1, 32).is_err());
        assert!(Value::date_from_int(2016, 4, 31).is_err());
        assert!(Value::date_from_int(2016, 2, 30).is_err());
        assert!(Value::date_from_int(2015, 2, 29).is_err());
        assert!(Value::date_from_int(1900, 2, 29).is_err());
        assert!(Value::date_from_int(2000, 2, 30).is_err());
    }

    #[test]
    fn test_create_date_from_str() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), None)),
                   Value::date_from_str("2012", "01", "03").unwrap());
    }

    #[test]
    fn test_create_date_from_str_fail() {
        assert!(Value::date_from_str("12345", "01", "01").is_err());
        assert!(Value::date_from_str("2016", "012", "01").is_err());
        assert!(Value::date_from_str("2016", "01", "012").is_err());
        assert!(Value::date_from_str("201q", "01", "01").is_err());
        assert!(Value::date_from_str("2016", "0q", "01").is_err());
        assert!(Value::date_from_str("2016", "01", "0q").is_err());
        assert!(Value::date_from_str("201", "01", "01").is_err());
        assert!(Value::date_from_str("2016", "1", "01").is_err());
        assert!(Value::date_from_str("2016", "01", "1").is_err());
    }

    #[test]
    fn test_create_datetime_from_int() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03", "30", "30", None, None)))),
                   Value::datetime_from_int(2012, 1, 3, 3, 30, 30).unwrap());
    }

    #[test]
    fn test_create_datetime_from_int_fail() {
        assert!(Value::datetime_from_int(2012, 1, 3, 24, 30, 30).is_err());
        assert!(Value::datetime_from_int(2012, 1, 3, 3, 60, 30).is_err());
        assert!(Value::datetime_from_int(2012, 1, 3, 3, 30, 60).is_err());
    }

    #[test]
    fn test_create_datetime_from_str() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03", "30", "30", None, None)))),
                   Value::datetime_from_str("2012", "01", "03", "03", "30", "30").unwrap());
    }

    #[test]
    fn test_create_datetime_from_str_fail() {
        assert!(Value::datetime_from_str("2012", "01", "03", "3", "30", "30").is_err());
        assert!(Value::datetime_from_str("2012", "01", "03", "03", "3", "30").is_err());
        assert!(Value::datetime_from_str("2012", "01", "03", "03", "30", "3").is_err());
        assert!(Value::datetime_from_str("2012", "01", "03", "033", "30", "30").is_err());
        assert!(Value::datetime_from_str("2012", "01", "03", "03", "303", "303").is_err());
        assert!(Value::datetime_from_str("2012", "01", "03", "0q", "30", "30").is_err());
        assert!(Value::datetime_from_str("2012", "01", "03", "03", "3q", "30").is_err());
        assert!(Value::datetime_from_str("2012", "01", "03", "03", "30", "3q").is_err());
    }

    #[test]
    fn test_create_datetime_frac_from_int() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03", "30", "30", Some("3030"), None)))),
                   Value::datetime_frac_from_int(2012, 1, 3, 3, 30, 30, 3030).unwrap());
    }

    #[test]
    fn test_create_datetime_frac_from_int_fail() {
        assert!(Value::datetime_frac_from_int(2012, 1, 0, 3, 30, 30, 3030).is_err());
    }

    #[test]
    fn test_create_datetime_frac_from_str() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03", "30", "30", Some("3030"), None)))),
                   Value::datetime_frac_from_str("2012", "01", "03", "03", "30", "30", "3030").unwrap());
    }

    #[test]
    fn test_create_datetime_frac_from_str_fail() {
        assert!(Value::datetime_frac_from_str("2012", "01", "03", "03", "30", "30", "q3030").is_err());
    }

    #[test]
    fn test_create_datetime_offset_from_int() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, Some(TimeOffset::Time(TimeOffsetAmount::new_str(
        "+", "07", "45"
      )))
    )))), Value::datetime_offset_from_int(2012, 1, 3, 3, 30, 30, '+', 7, 45).unwrap());
    }

    #[test]
    fn test_create_datetime_offset_from_int_fail() {
        assert!(Value::datetime_offset_from_int(2012, 1, 3, 3, 30, 30, 'q', 7, 45).is_err());
        assert!(Value::datetime_offset_from_int(2012, 1, 3, 3, 30, 30, '+', 24, 45).is_err());
        assert!(Value::datetime_offset_from_int(2012, 1, 3, 3, 30, 30, '+', 7, 60).is_err());
    }

    #[test]
    fn test_create_datetime_offset_from_str() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, Some(TimeOffset::Time(TimeOffsetAmount::new_str(
        "+", "07", "45"
      )))
    )))), Value::datetime_offset_from_str("2012", "01", "03", "03", "30", "30", "+", "07", "45").unwrap());
    }

    #[test]
    fn test_create_datetime_offset_from_str_fail() {
        assert!(Value::datetime_offset_from_str("2012", "01", "03", "03", "30", "30", "+", "077", "45").is_err());
        assert!(Value::datetime_offset_from_str("2012", "01", "03", "03", "30", "30", "+", "07", "455").is_err());
        assert!(Value::datetime_offset_from_str("2012", "01", "03", "03", "30", "30", "+", "7", "45").is_err());
        assert!(Value::datetime_offset_from_str("2012", "01", "03", "03", "30", "30", "+", "07", "5").is_err());
        assert!(Value::datetime_offset_from_str("2012", "01", "03", "03", "30", "30", "q", "07", "45").is_err());
    }

    #[test]
    fn test_create_datetime_zulu_from_int() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03", "30", "30", None, Some(TimeOffset::Zulu))))),
                   Value::datetime_zulu_from_int(2012, 1, 3, 3, 30, 30).unwrap());
    }

    #[test]
    fn test_create_datetime_zulu_from_int_fail() {
        assert!(Value::datetime_zulu_from_int(2012, 1, 0, 3, 30, 30).is_err());
    }

    #[test]
    fn test_create_datetime_zulu_from_str() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03", "30", "30", None, Some(TimeOffset::Zulu))))),
                   Value::datetime_zulu_from_str("2012", "01", "03", "03", "30", "30").unwrap());
    }

    #[test]
    fn test_create_datetime_zulu_from_str_fail() {
        assert!(Value::datetime_zulu_from_str("q2012", "01", "03", "03", "30", "30").is_err());
    }

    #[test]
    fn test_create_datetime_full_zulu_from_int() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03",
                                                                    "30",
                                                                    "30",
                                                                    Some("3030"),
                                                                    Some(TimeOffset::Zulu))))),
                   Value::datetime_full_zulu_from_int(2012, 1, 3, 3, 30, 30, 3030).unwrap());
    }

    #[test]
    fn test_create_datetime_full_zulu_from_int_fail() {
        assert!(Value::datetime_full_zulu_from_int(2012, 1, 0, 3, 30, 30, 3030).is_err());
    }

    #[test]
    fn test_create_datetime_full_zulu_from_str() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03",
                                                                    "30",
                                                                    "30",
                                                                    Some("3030"),
                                                                    Some(TimeOffset::Zulu))))),
                   Value::datetime_full_zulu_from_str("2012", "01", "03", "03", "30", "30", "3030").unwrap());
    }

    #[test]
    fn test_create_datetime_full_zulu_from_str_fail() {
        assert!(Value::datetime_full_zulu_from_str("q2012", "01", "03", "03", "30", "30", "3030").is_err());
    }

    #[test]
    fn test_create_datetime_full_from_int() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", Some("3030"), Some(TimeOffset::Time(TimeOffsetAmount::new_str(
        "+", "07", "45"
      )))
    )))), Value::datetime_full_from_int(2012, 1, 3, 3, 30, 30, 3030, '+', 7, 45).unwrap());
    }

    #[test]
    fn test_create_datetime_full_from_int_fail() {
        assert!(Value::datetime_full_from_int(2012, 1, 0, 3, 30, 30, 3030, '+', 7, 45).is_err());
        assert!(Value::datetime_full_from_int(2012, 13, 0, 3, 30, 30, 3030, '+', 7, 45).is_err());
        assert!(Value::datetime_full_from_int(2012, 1, 0, 3, 61, 30, 3030, '+', 7, 45).is_err());
        assert!(Value::datetime_full_from_int(2012, 1, 0, 3, 30, 30, 3030, '+', 25, 45).is_err());
        assert!(Value::datetime_full_from_int(2012, 1, 0, 3, 30, 30, 3030, 'q', 7, 45).is_err());
    }

    #[test]
    fn test_create_datetime_full_from_str() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", Some("3030"), Some(TimeOffset::Time(TimeOffsetAmount::new_str(
        "+", "07", "45"
      )))
    )))), Value::datetime_full_from_str("2012", "01", "03", "03", "30", "30", "3030", "+", "07", "45").unwrap());
    }

    #[test]
    fn test_create_datetime_full_from_str_fail() {
        assert!(Value::datetime_full_from_str("2012", "01", "03", "03", "30", "30", "q3030", "+", "07", "45").is_err());
        assert!(Value::datetime_full_from_str("2012", "01", "03", "03", "30", "30", "3030", "q", "07", "45").is_err());
    }

    #[test]
    fn test_datetime_parse() {
        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", Some("3030"), Some(TimeOffset::Time(TimeOffsetAmount::new_str(
        "+", "07", "45"
      )))
    )))), Value::datetime_parse("2012-01-03T03:30:30.3030+07:45").unwrap());

        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03",
                                                                    "30",
                                                                    "30",
                                                                    Some("3030"),
                                                                    Some(TimeOffset::Zulu))))),
                   Value::datetime_parse("2012-01-03T03:30:30.3030Z").unwrap());

        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03", "30", "30", None, Some(TimeOffset::Zulu))))),
                   Value::datetime_parse("2012-01-03T03:30:30Z").unwrap());

        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, Some(TimeOffset::Time(TimeOffsetAmount::new_str(
        "+", "07", "45"
      )))
    )))), Value::datetime_parse("2012-01-03T03:30:30+07:45").unwrap());

        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"),
                                                 Some(Time::new_str("03", "30", "30", None, None)))),
                   Value::datetime_parse("2012-01-03T03:30:30").unwrap());

        assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), None)),
                   Value::datetime_parse("2012-01-03").unwrap());
    }

    #[test]
    fn test_datetime_parse_fail() {
        assert!(Value::datetime_parse("012-01-03T03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-1-03T03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-3T03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T3:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:0:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:0.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30.+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30.303007:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30.3030+7:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30.3030+07:5").is_err());
        assert!(Value::datetime_parse("20123-01-03T03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-013-03T03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-033T03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T033:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:303:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:303.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30.3030+073:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30.3030+07:453").is_err());
        assert!(Value::datetime_parse("2012q01-03T03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01q03T03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03q03:30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03q30:30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30q30.3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30q3030+07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30.3030q07:45").is_err());
        assert!(Value::datetime_parse("2012-01-03T03:30:30.3030+07q45").is_err());
    }

    #[test]
    fn test_create_basic_string() {
        assert_eq!(Value::String("foobar".into(), StrType::Basic), Value::basic_string("foobar").unwrap());
    }

    #[test]
    fn test_create_basic_string_fail() {
        assert!(Value::basic_string("foo\nbar").is_err());
    }

    #[test]
    fn test_create_ml_basic_string() {
        assert_eq!(Value::String("foobar".into(), StrType::MLBasic), Value::ml_basic_string("foobar").unwrap());
    }

    #[test]
    fn test_create_ml_basic_string_fail() {
        assert!(Value::ml_basic_string(r#"foo\qbar"#).is_err());
    }

    #[test]
    fn test_create_literal_string() {
        assert_eq!(Value::String("foobar".into(), StrType::Literal), Value::literal_string("foobar").unwrap());
    }

    #[test]
    fn test_create_literal_string_fail() {
        assert!(Value::literal_string(r#"foo
bar"#)
                    .is_err());
    }

    #[test]
    fn test_create_ml_literal_string() {
        assert_eq!(Value::String("foobar".into(), StrType::MLLiteral), Value::ml_literal_string("foobar").unwrap());
    }

    #[test]
    fn test_create_ml_literal_string_fail() {
        // This string contains an invisible 0xC char between foo and bar. It's visible in
        // Sublime Text, but not in VS Code
        assert!(Value::ml_literal_string("foobar").is_err());
    }

}
