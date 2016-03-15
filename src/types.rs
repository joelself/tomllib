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

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ParseResult<'a> {
	Full,
	FullError(Rc<RefCell<Vec<ParseError<'a>>>>),
	Partial(Cow<'a, str>, usize, usize),
	PartialError(Cow<'a, str>, usize, usize, Rc<RefCell<Vec<ParseError<'a>>>>),
	Failure(usize, usize),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ParseError<'a> {
	MixedArray(String, usize, usize),
	DuplicateKey(String, usize, usize, Value<'a>),
	InvalidTable(String, usize, usize, RefCell<HashMap<String, Value<'a>>>),
  InvalidDateTime(String, usize, usize, Cow<'a, str>),
	IntegerOverflow(String, usize, usize, Cow<'a, str>),
	IntegerUnderflow(String, usize, usize, Cow<'a, str>),
	InvalidInteger(String, usize, usize, Cow<'a, str>),
	Infinity(String, usize, usize, Cow<'a, str>),
	NegativeInfinity(String, usize, usize, Cow<'a, str>),
	LossOfPrecision(String, usize, usize, Cow<'a, str>),
	InvalidFloat(String, usize, usize, Cow<'a, str>),
  InvalidBoolean(String, usize, usize, Cow<'a, str>),
  InvalidString(String, usize, usize, Cow<'a, str>, StrType),
  GenericError(String, usize, usize, Option<Cow<'a, str>>, String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value<'a> {
	Integer(Cow<'a, str>),
	Float(Cow<'a, str>),
	Boolean(bool),
	DateTime(DateTime<'a>),
	String(Cow<'a, str>, StrType),
	Array(Rc<Vec<Value<'a>>>),
	InlineTable(Rc<Vec<(Cow<'a, str>, Value<'a>)>>)
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum StrType {
	Basic,
	MLBasic,
	Literal,
	MLLiteral,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Children {
  Count(Cell<usize>),
  Keys(RefCell<Vec<String>>)
}

impl Children {
  pub fn combine_keys<S>(base_key: S, child_key: S) -> String where S: Into<String> {
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
  pub fn combine_keys_index<S>(base_key: S, child_key: usize) -> String where S: Into<String> {
    return format!("{}[{}]", base_key.into(), child_key);
  }
  pub fn combine_child_keys<S>(&self, base_key: S) -> Vec<String> where S: Into<String> {
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

impl<'a> Display for Value<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&Value::Integer(ref v) | &Value::Float(ref v) =>
				write!(f, "{}", v),
			&Value::Boolean(ref b) => write!(f, "{}", b),
			&Value::DateTime(ref v) => write!(f, "{}", v),
			&Value::Array(ref arr) => {
				try!(write!(f, "["));
				for i in 0..arr.len() - 1 {
					try!(write!(f, "{}, ", arr[i]));
				}
				if arr.len() > 0 {
					try!(write!(f, "{}", arr[arr.len()-1]));
				}
				write!(f, "]")
			},
			&Value::String(ref s, ref t) => {
				match t {
					&StrType::Basic => write!(f, "\"{}\"", s),
					&StrType::MLBasic => write!(f, "\"\"\"{}\"\"\"", s),
					&StrType::Literal => write!(f, "'{}'", s),
					&StrType::MLLiteral =>  write!(f, "'''{}'''", s),
				}
			},
			&Value::InlineTable(ref it) => {
				try!(write!(f, "{{"));
				for i in 0..it.len() - 1 {
					try!(write!(f, "{} = {}, ", it[i].0, it[i].1));
				}
				if it.len() > 0 {
					try!(write!(f, "{} = {}", it[it.len()-1].0, it[it.len()-1].1));
				}
				write!(f, "}}")
			}
		}
	}
}

impl<'a> Value<'a> {
  pub fn int(int: i64) -> Value<'a> {
    Value::Integer(format!("{}", int).into())
  }
  pub fn int_from_str<S>(int: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let result = Value::Integer(int.clone().into().into());
    if result.validate() {
      return Result::Ok(result);
    } else {
      return Result::Err(TOMLError::new(format!("Error parsing int. Argument: {}", int.into())));
    }
  }
  pub fn float(float: f64) -> Value<'a> {
    Value::Float(format!("{}", float).into())
  }
  pub fn float_from_str<S>(float: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let result = Value::Float(float.clone().into().into());
    if result.validate() {
      return Result::Ok(result);
    } else {
      return Result::Err(TOMLError::new(format!("Error parsing float. Argument: {}", float.into())));
    }
  }
  pub fn bool(b: bool) -> Value<'a> {
      Value::Boolean(b)
  }
  pub fn bool_from_str<S>(b: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let lower = b.clone().into().to_lowercase();
    if lower == "true" {
      Result::Ok(Value::Boolean(true))
    } else if lower == "false" {
      Result::Ok(Value::Boolean(false))
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing bool. Argument: {}", b.into())
      ))
    }
  }
  
  pub fn date_from_int(year: usize, month: usize, day: usize) -> Result<Value<'a>, TOMLError> {
    let y = format!("{:0>4}", year);
    let m = format!("{:0>2}", month);
    let d = format!("{:0>2}", day);
    let datetime = Value::DateTime(DateTime::new(Date::from_str(y, m, d), None));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing int date. Arguments: {}, {}, {}", year, month, day)
      ));
    }
  }
  pub fn date_from_str<S>(year: S, month: S, day: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let datetime = Value::DateTime(DateTime::new(Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()), None));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing &str date. Arguments: {}, {}, {}", year.into(), month.into(), day.into())
      ));
    }
  }
  pub fn datetime_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize) -> Result<Value<'a>, TOMLError> {
    let y = format!("{:0>4}", year);
    let m = format!("{:0>2}", month);
    let d = format!("{:0>2}", day);
    let h = format!("{:0>2}", hour);
    let min = format!("{:0>2}", minute);
    let s = format!("{:0>2}", second);
    let datetime = Value::DateTime(DateTime::new(Date::from_str(y, m, d), Some(
      Time::from_str(h, min, s, None, None)
    )));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing int datetime. Arguments: {}, {}, {}, {}, {}, {}", year, month, day, hour, minute, second)
      ));
    }
  }
  pub fn datetime_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let datetime = Value::DateTime(DateTime::new(Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()), Some(
      Time::from_str(hour.clone().into(), minute.clone().into(), second.clone().into(), None, None)
    )));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing &str datetime. Arguments: {}, {}, {}, {}, {}, {}", year.into(), month.into(), day.into(), hour.into(), minute.into(), second.into())
      ));
    }
  }

  pub fn datetime_frac_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize, frac: usize) -> Result<Value<'a>, TOMLError> {  
    let y = format!("{:0>4}", year);
    let m = format!("{:0>2}", month);
    let d = format!("{:0>2}", day);
    let h = format!("{:0>2}", hour);
    let min = format!("{:0>2}", minute);
    let s = format!("{:0>2}", second);
    let f = format!("{}", frac);
    let datetime = Value::DateTime(DateTime::new(Date::from_str(y, m, d), Some(
      Time::from_str(h, min, s, Some(f), None)
    )));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing int datetime_frac. Arguments: {}, {}, {}, {}, {}, {}, {}", year, month, day, hour, minute, second, frac)
      ));
    }
  }
  pub fn datetime_frac_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S, frac: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone{ 
    let datetime = Value::DateTime(DateTime::new(Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()), Some(
      Time::from_str(hour.clone().into(), minute.clone().into(), second.clone().into(), Some(frac.clone().into()), None)
    )));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing &str datetime_frac. Arguments: {}, {}, {}, {}, {}, {}, {}", year.into(), month.into(), day.into(), hour.into(), minute.into(), second.into(), frac.into())
      ));
    }
  }
  pub fn datetime_offset_from_int<S>(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize, posneg: S, off_hour: usize, off_minute: usize) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone{  
    let y = format!("{:0>4}", year);
    let m = format!("{:0>2}", month);
    let d = format!("{:0>2}", day);
    let h = format!("{:0>2}", hour);
    let min = format!("{:0>2}", minute);
    let s = format!("{:0>2}", second);
    let oh = format!("{:0>2}", off_hour);
    let omin = format!("{:0>2}", off_minute);
    let pn = &posneg.clone().into();
    let mut error = false;
    if pn != "+" && pn != "-" {
      error = true
    }
    let datetime = Value::DateTime(DateTime::new(Date::from_str(y, m, d), Some(
      Time::from_str(h, min, s, None, Some(
        TimeOffset::Time(TimeOffsetAmount::from_str(posneg.clone().into(), oh, omin))
      ))
    )));
    if !error && datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing int datetime_offset. Arguments: {}, {}, {}, {}, {}, {}, {}, {}, {}", year, month, day, hour, minute, second, posneg.into(), off_hour, off_minute)
      ));
    }
  }
  pub fn datetime_offset_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S, posneg: S, off_hour: S, off_minute: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone{
    let pn = &posneg.clone().into();
    let mut error = false;
    if pn != "+" && pn != "-" {
      error = true
    }
    let datetime = Value::DateTime(DateTime::new(Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()), Some(
      Time::from_str(hour.clone().into(), minute.clone().into(), second.clone().into(), None, Some(
        TimeOffset::Time(TimeOffsetAmount::from_str(posneg.clone().into(), off_hour.clone().into(), off_minute.clone().into()))
      ))
    )));
    if !error && datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing &str datetime_offset. Arguments: {}, {}, {}, {}, {}, {}, {}, {}, {}", year.into(), month.into(), day.into(), hour.into(), minute.into(), second.into(), posneg.into(), off_hour.into(), off_minute.into())
      ));
    }
  }
  
  pub fn datetime_zulu_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize) -> Result<Value<'a>, TOMLError> {  
    let y = format!("{:0>4}", year);
    let m = format!("{:0>2}", month);
    let d = format!("{:0>2}", day);
    let h = format!("{:0>2}", hour);
    let min = format!("{:0>2}", minute);
    let s = format!("{:0>2}", second);
    let datetime = Value::DateTime(DateTime::new(Date::from_str(y, m, d), Some(
      Time::from_str(h, min, s, None, Some(
        TimeOffset::Zulu
      ))
    )));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing int datetime_zulu. Arguments: {}, {}, {}, {}, {}, {}", year, month, day, hour, minute, second)
      ));
    }
  }
  
  pub fn datetime_zulu_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone{ 
    let datetime = Value::DateTime(DateTime::new(Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()), Some(
      Time::from_str(hour.clone().into(), minute.clone().into(), second.clone().into(), None, Some(
        TimeOffset::Zulu
      ))
    )));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing &str datetime_zulu. Arguments: {}, {}, {}, {}, {}, {}", year.into(), month.into(), day.into(), hour.into(), minute.into(), second.into())
      ));
    }
  }
  pub fn datetime_full_zulu_from_int(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize, frac: u64) -> Result<Value<'a>, TOMLError> {  
    let y = format!("{:0>4}", year);
    let m = format!("{:0>2}", month);
    let d = format!("{:0>2}", day);
    let h = format!("{:0>2}", hour);
    let min = format!("{:0>2}", minute);
    let s = format!("{:0>2}", second);
    let f = format!("{}", frac);
    let datetime = Value::DateTime(DateTime::new(Date::from_str(y, m, d), Some(
      Time::from_str(h, min, s, Some(f), Some(
        TimeOffset::Zulu
      ))
    )));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing int datetime_full_zulu. Arguments: {}, {}, {}, {}, {}, {}, {}",
          year, month, day, hour, minute, second, frac)
      ));
    }
  }
  
  pub fn datetime_full_zulu_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S, frac: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone{ 
    let datetime = Value::DateTime(DateTime::new(Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()), Some(
      Time::from_str(hour.clone().into(), minute.clone().into(), second.clone().into(), Some(frac.clone().into()), Some(
        TimeOffset::Zulu
      ))
    )));
    if datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing &str datetime_full_zulu. Arguments: {}, {}, {}, {}, {}, {}, {}",
          year.into(), month.into(), day.into(), hour.into(), minute.into(), second.into(), frac.into())
      ));
    }
  }
  
  pub fn datetime_full_from_int<S>(year: usize, month: usize, day: usize, hour: usize, minute: usize, second: usize, frac: usize, posneg: S, off_hour: usize, off_minute: usize) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {  
    let y = format!("{:0>4}", year);
    let m = format!("{:0>2}", month);
    let d = format!("{:0>2}", day);
    let h = format!("{:0>2}", hour);
    let min = format!("{:0>2}", minute);
    let s = format!("{:0>2}", second);
    let f = format!("{}", frac);
    let oh = format!("{:0>2}", off_hour);
    let omin = format!("{:0>2}", off_minute);
    let pn = &posneg.clone().into();
    let mut error = false;
    if pn != "+" && pn != "-" {
      error = true
    }
    let datetime = Value::DateTime(DateTime::new(Date::from_str(y, m, d), Some(
      Time::from_str(h, min, s, Some(f), Some(
        TimeOffset::Time(TimeOffsetAmount::from_str(posneg.clone().into(), oh, omin))
      ))
    )));
    if !error && datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing int datetime_full. Arguments: {}, {}, {}, {}, {}, {}, {}, {}, {}, {}", year, month, day, hour, minute, second, frac, posneg.into(), off_hour, off_minute)
      ));
    }
  }
  pub fn datetime_full_from_str<S>(year: S, month: S, day: S, hour: S, minute: S, second: S, frac: S, posneg: S, off_hour: S, off_minute: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone { 
    let pn = &posneg.clone().into();
    let mut error = false;
    if pn != "+" && pn != "-" {
      error = true
    }
    let datetime = Value::DateTime(DateTime::new(Date::from_str(year.clone().into(), month.clone().into(), day.clone().into()), Some(
      Time::from_str(hour.clone().into(), minute.clone().into(), second.clone().into(), Some(frac.clone().into()), Some(
        TimeOffset::Time(TimeOffsetAmount::from_str(posneg.clone().into(), off_hour.clone().into(), off_minute.clone().into()))
      ))
    )));
    if !error && datetime.validate() {
      return Result::Ok(datetime);
    } else {
      return Result::Err(TOMLError::new(
        format!("Error parsing &str datetime_full. Arguments: {}, {}, {}, {}, {}, {}, {}, {}, {}, {}", year.into(), month.into(), day.into(), hour.into(), minute.into(), second.into(), frac.into(), posneg.into(), off_hour.into(), off_minute.into())
      ));
    }
  }
  pub fn datetime_parse<S>(dt: S) -> Result<Value<'a>, TOMLError> where S: Into<&'a str> {
    let datetime = dt.into();
    let p = Parser::new();
    match p.date_time(datetime) {
      (_, IResult::Done(i, o)) => {
        let result = Value::DateTime(o);
        if i.len() > 0 || !result.validate() {
          return Result::Err(TOMLError::new(format!("Error parsing string as datetime. Argument: {}", datetime)));
        } else {
          return Result::Ok(result);
        }
      },
      (_,_) => return Result::Err(TOMLError::new(format!("Error parsing string as datetime. Argument: {}", datetime))),
    }
  }
  pub fn basic_string<S>(s: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let result = Value::String(s.clone().into().into(), StrType::Basic);
    if result.validate() {
      return Result::Ok(result);
    } else {
      return Result::Err(TOMLError::new(format!("Error parsing string as basic_string. Argument: {}", s.into())));
    }
  }
  pub fn ml_basic_string<S>(s: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let result = Value::String(s.clone().into().into(), StrType::MLBasic);
    if result.validate() {
      return Result::Ok(result);
    } else {
      return Result::Err(TOMLError::new(format!("Error parsing string as ml_basic_string. Argument: {}", s.into())));
    }
  }
  pub fn literal_string<S>(s: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let result = Value::String(s.clone().into().into(), StrType::Literal);
    if result.validate() {
      return Result::Ok(result);
    } else {
      return Result::Err(TOMLError::new(format!("Error parsing string as literal_string. Argument: {}", s.into())));
    }
  }
  pub fn ml_literal_string<S>(s: S) -> Result<Value<'a>, TOMLError> where S: Into<String> + Clone {
    let result = Value::String(s.clone().into().into(), StrType::MLLiteral);
    if result.validate() {
      return Result::Ok(result);
    } else {
      return Result::Err(TOMLError::new(format!("Error parsing string as ml_literal_string. Argument: {}", s.into())));
    }
  }
  
  pub fn validate(&self) -> bool{
    match self {
      &Value::Integer(ref s) => {
        let p = Parser::new();
        match p.integer(s) {
           (_, IResult::Done(_, _)) => true,
           (_,_) => false,
        }
      },
      &Value::Float(ref s) => {
        let p = Parser::new();
        match p.float(s) {
           (_, IResult::Done(_, _)) => true,
           (_,_) => false,
        }
      },
      &Value::DateTime(ref dt) => {dt.validate()},
      &Value::String(ref s, st) => {
        match st {
          StrType::Basic => {
            match Parser::quoteless_basic_string(s) {
              IResult::Done(i,_) => i.len() == 0,
              _ => false,
            }
          },
          StrType::MLBasic => {
            match Parser::quoteless_ml_basic_string(s) {
              IResult::Done(i,_) => i.len() == 0,
              _ => false,
            }
          },
          StrType::Literal => {
            match Parser::quoteless_literal_string(s) {
              IResult::Done(i,_) => i.len() == 0,
              _ => false,
            }
          },
          StrType::MLLiteral => {
            match Parser::quoteless_ml_literal_string(s) {
              IResult::Done(i,_) => i.len() == 0,
              _ => false,
            }
          },
        }
      },
      _ => true,
    }
  }
}

#[derive(Debug)]
pub struct TOMLError {
  message: String,
}

impl Error for TOMLError {
    fn description(&self) -> &str {
      &self.message
    }
    fn cause(&self) -> Option<&Error> { None }
}

impl Display for TOMLError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl TOMLError {
  fn new(msg: String) -> TOMLError {
    warn!("{}", msg);
    TOMLError{message: msg}
  }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PosNeg {
	Pos,
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

#[derive(Debug, Eq, Clone)]
pub enum TimeOffset<'a> {
	Zulu,
	Time(TimeOffsetAmount<'a>),
}

impl<'a> PartialEq for TimeOffset<'a> {
	fn eq(&self, other: &TimeOffset<'a>) -> bool {
		match (self, other) {
			(&TimeOffset::Zulu, &TimeOffset::Zulu) => true,
			(&TimeOffset::Time(ref i), &TimeOffset::Time(ref j)) if(i == j) => true,
			_ => false
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

// (+|-)<hour>:<minute>
#[derive(Debug, Eq, Clone)]
pub struct TimeOffsetAmount<'a> {
	pub pos_neg: PosNeg,
	pub hour: Cow<'a, str>,
	pub minute: Cow<'a, str>,
}

impl<'a> PartialEq for TimeOffsetAmount<'a> {
	fn eq(&self, other: &TimeOffsetAmount<'a>) -> bool {
		self.pos_neg == other.pos_neg &&
		self.hour == other.hour &&
		self.minute == other.minute
	}
}

impl<'a> Display for TimeOffsetAmount<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
  	write!(f, "{}{}:{}", self.pos_neg, &self.hour, &self.minute)
  }
}

impl<'a> TimeOffsetAmount<'a> {
  pub fn from_str<S>(pos_neg: S, hour: S, minute: S) -> TimeOffsetAmount<'a> where S: Into<String>{
  	let pn = match pos_neg.into().as_ref() {
  		"+" => PosNeg::Pos,
  		"-"	=> PosNeg::Neg,
      _   => {warn!("PosNeg value is neither a '+' or a '-', defaulting to '+'."); PosNeg::Pos},
  	};
  	TimeOffsetAmount{pos_neg: pn, hour: hour.into().into(), minute: minute.into().into()}
  }
  
  pub fn validate(&self) -> bool {
    if self.hour.len() != 2 || self.minute.len() != 2 {
      return false;
    }
    return self.validate_numbers();
   }
   
  pub fn validate_numbers(&self) -> bool {
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

// <year>-<month>-<day>
#[derive(Debug, Eq, Clone)]
pub struct Date<'a> {
	pub year: Cow<'a, str>,
	pub month: Cow<'a, str>,
	pub day: Cow<'a, str>,
}

impl<'a> PartialEq for Date<'a> {
	fn eq(&self, other: &Date<'a>) -> bool {
		self.year == other.year &&
		self.month == other.month &&
		self.day == other.day
	}
}

impl<'a> Display for Date<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
  	write!(f, "{}-{}-{}", self.year, self.month, self.day)
  }
}

impl<'a> Date<'a> {
  pub fn from_str<S>(year: S, month: S, day: S) -> Date<'a> where S: Into<String> {
  	Date{year: year.into().into(), month: month.into().into(), day: day.into().into()}
  }
  
  pub fn validate(&self) -> bool {
    if self.year.len() != 4 || self.month.len() != 2 || self.day.len() != 2 {
      return false;
    }
    return self.validate_numbers();
  }
  
  pub fn validate_numbers(&self) -> bool {
    if let Ok(y) = usize::from_str(&self.year) {
      if y == 0 {
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
            1 | 3 | 5 | 7 | 8 | 10 | 12 => { if d > 31 { return false; } },
            _ => { if d > 30 { return false; } },
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

// <hour>:<minute>:<second>(.<fraction>)?
#[derive(Debug, Eq, Clone)]
pub struct Time<'a> {
  pub hour: Cow<'a, str>,
	pub minute: Cow<'a, str>,
	pub second: Cow<'a, str>,
	pub fraction: Option<Cow<'a, str>>,
	pub offset: Option<TimeOffset<'a>>,
}

impl<'a> PartialEq for Time<'a> {
	fn eq(&self, other: &Time<'a>) -> bool {
		self.hour == other.hour &&
		self.minute == other.minute &&
		self.second == other.second &&
		self.fraction == other.fraction &&
		self.offset == other.offset
	}
}

impl<'a> Display for Time<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
  	match (&self.fraction, &self.offset) {
  		(&Some(ref frac), &Some(ref offset)) 	=> write!(f, "T{}:{}:{}.{}{}", self.hour, self.minute, self.second, frac, offset),
  		(&Some(ref frac), &None) 							=> write!(f, "T{}:{}:{}.{}", self.hour, self.minute, self.second, frac),
  		(&None, &Some(ref offset)) 						=> write!(f, "T{}:{}:{}{}", self.hour, self.minute, self.second, offset),
  		(&None, &None) 												=> write!(f, "T{}:{}:{}", self.hour, self.minute, self.second),
  	}
  }
}

impl<'a> Time<'a> {
  pub fn from_str<S>(hour: S, minute: S, second: S, fraction: Option<S>, offset: Option<TimeOffset<'a>>) 
  	-> Time<'a> where S: Into<String> {
  	if let Some(s) = fraction {
  		Time{hour: hour.into().into(), minute: minute.into().into(), second: second.into().into(),
  			fraction: Some(s.into().into()), offset: offset}
  	} else {
  		Time{hour: hour.into().into(), minute: minute.into().into(), second: second.into().into(),
  			fraction: None, offset: offset}
    }
  }
  
  pub fn validate(&self) -> bool {
    if self.hour.len() != 2 || self.minute.len() != 2 || self.second.len() != 2 {
      return false;
    }
    return self.validate_numbers(); 
  }
  
  pub fn validate_numbers(&self) -> bool {
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
      if usize::from_str(frac).is_err() {
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

#[derive(Debug, Eq, Clone)]
pub struct DateTime<'a> {
	pub date: Date<'a>,
	pub time: Option<Time<'a>>,
}

impl<'a> PartialEq for DateTime<'a> {
	fn eq(&self, other: &DateTime<'a>) -> bool {
		self.date == other.date &&
		self.time == other.time
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

impl<'a> DateTime<'a> {
	pub fn new(date: Date<'a>, time: Option<Time<'a>>) -> DateTime<'a> {
		DateTime{date: date, time: time}
	}
  
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
    let kids = Children::Keys(RefCell::new(vec!["baz".to_string(), "qux".to_string(), "plugh".to_string(),
      "thud".to_string()]));
    assert_eq!(vec!["foo.bar.baz".to_string(), "foo.bar.qux".to_string(), "foo.bar.plugh".to_string(),
      "foo.bar.thud".to_string()], kids.combine_child_keys("foo.bar".to_string()));
  }
  
  #[test]
  fn test_combine_child_keys_empty_base() {
    let kids = Children::Keys(RefCell::new(vec!["baz".to_string(), "qux".to_string(), "plugh".to_string(),
      "thud".to_string()]));
    assert_eq!(vec!["baz".to_string(), "qux".to_string(), "plugh".to_string(),
      "thud".to_string()], kids.combine_child_keys("".to_string()));
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
    let val_datetime = Value::DateTime(DateTime::new(Date::new_str("9999", "12", "31"), Some(Time::new_str(
      "23", "59", "59", Some("9999999"), Some(TimeOffset::Time(TimeOffsetAmount::new_str(
        "-", "00", "00"
      )))
    ))));
    let val_basic_str = Value::String("foobar1".into(), StrType::Basic);
    let val_literal_str = Value::String("foobar2".into(), StrType::Literal);
    let val_ml_basic_str = Value::String("foobar3".into(), StrType::MLBasic);
    let val_ml_literal_str = Value::String("foobar4".into(), StrType::MLLiteral);
    let val_array = Value::Array(Rc::new(vec![Value::Integer("3000".into()),
      Value::Array(Rc::new(vec![Value::Integer("40000".into()), Value::Float("50.5".into())])),
      Value::String("barbaz".into(), StrType::Literal)]));
    let val_inline_table = Value::InlineTable(Rc::new(vec![
      ("foo".into(), Value::Boolean(true)), ("bar".into(), Value::InlineTable(Rc::new(vec![
        ("baz".into(), Value::Boolean(false)), ("qux".into(), Value::Integer("2016".into())),
      ]))), ("plugh".into(), Value::Float("3333.444".into()))
    ]));
    
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
    assert_eq!("{foo = true, bar = {baz = false, qux = 2016}, plugh = 3333.444}",
      &format!("{}", val_inline_table));
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
    assert_eq!(Value::Float("179769000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into()), Value::float(1.79769e+308));
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
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, None
    )))), Value::datetime_from_int(2012, 1, 3, 3, 30, 30).unwrap());
  }
  
  #[test]
  fn test_create_datetime_from_int_fail() {
    assert!(Value::datetime_from_int(2012, 1, 3, 24, 30, 30).is_err());
    assert!(Value::datetime_from_int(2012, 1, 3, 3, 60, 30).is_err());
    assert!(Value::datetime_from_int(2012, 1, 3, 3, 30, 60).is_err());
  }
  
  #[test]
  fn test_create_datetime_from_str() {
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, None
    )))), Value::datetime_from_str("2012", "01", "03", "03", "30", "30").unwrap());
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
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", Some("3030"), None
    )))), Value::datetime_frac_from_int(2012, 1, 3, 3, 30, 30, 3030).unwrap());
  }
  
  #[test]
  fn test_create_datetime_frac_from_int_fail() {
    assert!(Value::datetime_frac_from_int(2012, 1, 0, 3, 30, 30, 3030).is_err());
  }
  
  #[test]
  fn test_create_datetime_frac_from_str() {
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", Some("3030"), None
    )))), Value::datetime_frac_from_str("2012", "01", "03", "03", "30", "30", "3030").unwrap());
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
    )))), Value::datetime_offset_from_int(2012, 1, 3, 3, 30, 30, "+", 7, 45).unwrap());
  }
  
  #[test]
  fn test_create_datetime_offset_from_int_fail() {
    assert!(Value::datetime_offset_from_int(2012, 1, 3, 3, 30, 30, "q", 7, 45).is_err());
    assert!(Value::datetime_offset_from_int(2012, 1, 3, 3, 30, 30, "+", 24, 45).is_err());
    assert!(Value::datetime_offset_from_int(2012, 1, 3, 3, 30, 30, "+", 7, 60).is_err());
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
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, Some(TimeOffset::Zulu)
    )))), Value::datetime_zulu_from_int(2012, 1, 3, 3, 30, 30).unwrap());
  }
  
  #[test]
  fn test_create_datetime_zulu_from_int_fail() {
    assert!(Value::datetime_zulu_from_int(2012, 1, 0, 3, 30, 30).is_err());
  }
  
  #[test]
  fn test_create_datetime_zulu_from_str() {
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, Some(TimeOffset::Zulu)
    )))), Value::datetime_zulu_from_str("2012", "01", "03", "03", "30", "30").unwrap());
  }
  
  #[test]
  fn test_create_datetime_zulu_from_str_fail() {
    assert!(Value::datetime_zulu_from_str("q2012", "01", "03", "03", "30", "30").is_err());
  }
  
  #[test]
  fn test_create_datetime_full_zulu_from_int() {
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", Some("3030"), Some(TimeOffset::Zulu)
    )))), Value::datetime_full_zulu_from_int(2012, 1, 3, 3, 30, 30, 3030).unwrap());
  }
  
  #[test]
  fn test_create_datetime_full_zulu_from_int_fail() {
    assert!(Value::datetime_full_zulu_from_int(2012, 1, 0, 3, 30, 30, 3030).is_err());
  }
  
  #[test]
  fn test_create_datetime_full_zulu_from_str() {
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", Some("3030"), Some(TimeOffset::Zulu)
    )))), Value::datetime_full_zulu_from_str("2012", "01", "03", "03", "30", "30", "3030").unwrap());
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
    )))), Value::datetime_full_from_int(2012, 1, 3, 3, 30, 30, 3030, "+", 7, 45).unwrap());
  }
  
  #[test]
  fn test_create_datetime_full_from_int_fail() {
    assert!(Value::datetime_full_from_int(2012, 1, 0, 3, 30, 30, 3030, "+", 7, 45).is_err());
    assert!(Value::datetime_full_from_int(2012, 1, 0, 3, 30, 30, 3030, "q", 7, 45).is_err());
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
    
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", Some("3030"), Some(TimeOffset::Zulu)
    )))), Value::datetime_parse("2012-01-03T03:30:30.3030Z").unwrap());
    
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, Some(TimeOffset::Zulu)
    )))), Value::datetime_parse("2012-01-03T03:30:30Z").unwrap());
    
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, Some(TimeOffset::Time(TimeOffsetAmount::new_str(
        "+", "07", "45"
      )))
    )))), Value::datetime_parse("2012-01-03T03:30:30+07:45").unwrap());
    
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), Some(Time::new_str(
      "03", "30", "30", None, None
    )))), Value::datetime_parse("2012-01-03T03:30:30").unwrap());
    
    assert_eq!(Value::DateTime(DateTime::new(Date::new_str("2012", "01", "03"), None
    )), Value::datetime_parse("2012-01-03").unwrap());
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
bar"#).is_err());
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