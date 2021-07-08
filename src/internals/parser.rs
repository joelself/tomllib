use std::fmt;
use std::fmt::Display;
use std::collections::{HashMap, BTreeMap};
use std::collections::hash_map::Entry;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::borrow::Cow;
use crate::internals::ast::structs::{HashValue, ArrayType, TOMLValue, Toml, TableType, Array, InlineTable, ArrayValue, WSSep,
                              TableKeyVal};
use crate::types::{ParseError, ParseResult, Value, Children};
use crate::internals::primitives::Key;
use nom::IResult;

pub struct Parser<'a> {
  pub root: RefCell<Toml<'a>>,
  pub map: HashMap<String, HashValue<'a>>,
  pub errors: Rc<RefCell<Vec<ParseError<'a>>>>,
  pub leftover: &'a str,
  pub line_count: Cell<usize>,
  pub last_array_tables: RefCell<Vec<Rc<TableType<'a>>>>,
  pub last_array_tables_index: RefCell<Vec<usize>>,
  pub keychain: RefCell<Vec<Key<'a>>>,
  pub last_table: Option<Rc<TableType<'a>>>,
  pub last_array_type: RefCell<Vec<ArrayType>>,
  pub array_error: Cell<bool>,
  pub mixed_array: Cell<bool>,
  pub failure: Cell<bool>,
}

// TODO change this to return a parser result
impl<'a> Parser<'a> {
  pub fn new() -> Parser<'a> {
    let mut map = HashMap::new();
    map.insert("$Root$".to_string(), HashValue::none_keys());
    Parser{ root: RefCell::new(Toml{ exprs: vec![] }), map,
            errors: Rc::new(RefCell::new(vec![])), leftover: "",
            line_count: Cell::new(1), last_array_tables: RefCell::new(vec![]),
            last_array_tables_index: RefCell::new(vec![]),
            last_table: None, last_array_type: RefCell::new(vec![]),
            keychain: RefCell::new(vec![]),
            array_error: Cell::new(false), mixed_array: Cell::new(false),
            failure: Cell::new(false)}
  }

  pub fn parse(mut self: Parser<'a>, input: &'a str) -> (Parser<'a>, ParseResult<'a>) {
    let (tmp, res) = self.toml(input);
    self = tmp;
    let line_count = self.line_count.get();
    let leftover = self.leftover.into();
    // let mut res = self.result;
    match res {
      IResult::Done(i, o) => {
        *self.root.borrow_mut() = o;
        self.leftover = i;
      },
      _ => return (self, ParseResult::Failure(line_count, 0)),
    };
    let len = self.errors.borrow().len();
    if !self.leftover.is_empty() {
      if len > 0 {
        let errors = self.errors.clone();
        return (self, ParseResult::PartialError(leftover, line_count, 0, errors));
      }
      return (self, ParseResult::Partial(leftover, line_count, 0));
    } else if len > 0 {
      let errors = self.errors.clone();
      return (self, ParseResult::FullError(errors));
    }
    (self, ParseResult::Full)
  }

  #[allow(dead_code)]
  fn print_keys_and_values_debug(self: &Parser<'a>) {
    let mut btree = BTreeMap::new();
    for (k, v) in &self.map {
      btree.insert(k, v);
    }
    for (k, v) in &btree {
      debug!("key: {} - {}", k, v);
    }
  }

  #[allow(dead_code)]
  fn print_keys_and_values(self: &Parser<'a>) {
    let mut btree = BTreeMap::new();
    for (k, v) in &self.map {
      btree.insert(k, v);
    }
    for (k, v) in &btree {
      println!("key: {} - {}", k, v);
    }
  }

  pub fn get_value<S>(self: &Parser<'a>, key: S) -> Option<Value<'a>> where S: Into<String> {
    let s_key = key.into();
    if self.map.contains_key(&s_key) {
      let hashval = self.map.get(&s_key).unwrap();
      let clone = hashval.clone();
      clone.value.map(|val| to_val!(&*val.borrow()))
    } else {
      None
    }
  }

  pub fn get_children<S>(self: &Parser<'a>, key: S) -> Option<&Children> where S: Into<String> {
    let s_key = key.into();
    let k;
    if s_key.is_empty() {
      k = "$Root$".to_string();
    } else {
      k = s_key;
    }
    if self.map.contains_key(&k) {
      let hashval = self.map.get(&k).unwrap();
      Some(&hashval.subkeys)
    } else {
      None
    }
  }

  pub fn set_value<S>(self: &mut Parser<'a>, key: S, val: Value<'a>) -> bool where S: Into<String> {
    let s_key = key.into();
    {
      let tvalue = match self.map.entry(s_key.clone()) {
        Entry::Occupied(entry) => entry.into_mut(),
        _ => return false,
      };
      let opt_value: &mut Option<Rc<RefCell<TOMLValue<'a>>>> = &mut tvalue.value;
      let val_rf = match *opt_value {
        Some(ref mut v) => v,
        None => return false,
      };
      // if the inline table/array has the same structure the just replace the values
      if Parser::same_structure(val_rf, &val) {
        return Parser::replace_values(val_rf, &val);
      }
    }
    // if the inline table/array has a different structure, delete the existing
    // array/inline table from the map and rebuild it from the new value
    let all_keys = self.get_all_subkeys(&s_key);
    for key in &all_keys {
      self.map.remove(key);
    }
    let new_value_opt = Parser::convert_vector(&val);
    if new_value_opt.is_none() {
      return false;
    }
    let new_value = new_value_opt.unwrap();
    let new_value_clone = match new_value {
      TOMLValue::Array(ref rc_rc) => {
        TOMLValue::Array(rc_rc.clone())
      },
      TOMLValue::InlineTable(ref rc_rc) => {
        TOMLValue::InlineTable(rc_rc.clone())
      },
      unknown => panic!("In Parser.set_value, new_value should only be an Array or InlineTable. Instead it's a {:?}", unknown),
    };
    if self.map.contains_key(&s_key) {
      let existing_value = match self.map.entry(s_key.clone()) {
        Entry::Occupied(entry) => entry.into_mut(),
        _ => panic!("Map contains key, but map.entry returned a Vacant entry is set_value."),
      };
      let opt_value: &mut Option<Rc<RefCell<TOMLValue<'a>>>> = &mut existing_value.value;
      let val_rf = match *opt_value {
        Some(ref mut v) => v,
        None => panic!("existing_value's value is None, when this should've returned false earlier in the function."),
      };
      *val_rf.borrow_mut() = new_value;
    }
    let new_value_rc = Rc::new(RefCell::new(new_value_clone));
    self.rebuild_vector(s_key, new_value_rc.clone(), true);
    true
  }

  fn convert_vector(tval: &Value<'a>) -> Option<TOMLValue<'a>> {
    if !tval.validate() {
      return None;
    }
     match *tval {
      Value::Array(ref arr) => {
        let mut values = vec![];
        for i in 0..arr.len() {
          let subval = &arr[i];
          let value_opt = Parser::convert_vector(subval);
          value_opt.as_ref()?;
          let value = value_opt.unwrap();
          let array_value;
          if i < arr.len() - 1 {
            array_value = ArrayValue::default(Rc::new(RefCell::new(value)));
          } else {
            array_value = ArrayValue::last(Rc::new(RefCell::new(value)));
          }
          values.push(array_value);
        }
        return Some(TOMLValue::Array(Rc::new(RefCell::new(
          Array::new(values, vec![], vec![])
        ))));
      },
      Value::InlineTable(ref it) => {
        let mut key_values = vec![];
        for i in 0..it.len() {
          let subval = &it[i].1;
          let value_opt = Parser::convert_vector(subval);
          let value = value_opt.unwrap();
          let key_value;
          if i < it.len() - 1 {
            key_value = TableKeyVal::default(it[i].0.clone().into_owned(), Rc::new(RefCell::new(value)));
          } else {
            key_value = TableKeyVal::last(it[i].0.clone().into_owned(), Rc::new(RefCell::new(value)));
          }
          key_values.push(key_value);
        }
        return Some(TOMLValue::InlineTable(Rc::new(RefCell::new(
          InlineTable::new(key_values, WSSep::new_str(" ", " "))
        ))));
      },
      Value::Integer(ref s) => {
        if tval.validate() {
          return Some(TOMLValue::Integer(s.clone()))
        }
        None
      },
      Value::Float(ref s) => {
        if tval.validate() {
          return Some(TOMLValue::Float(s.clone()))
        }
        None
      },
      Value::Boolean(b) => return Some(TOMLValue::Boolean(b)),
      Value::DateTime(ref dt) => {
        if tval.validate() {
          return Some(TOMLValue::DateTime(dt.clone()))
        }
        None
      },
      Value::String(ref s, st) => {
        if tval.validate() {
          return Some(TOMLValue::String(s.clone(), st))
        }
        None
      },
    }
  }

  fn same_structure(val_rf: &Rc<RefCell<TOMLValue<'a>>>, tval: &Value<'a>) -> bool {
    if !tval.validate() {
      return false;
    }
    match (&*val_rf.borrow(), tval) {
      (&TOMLValue::Array(ref arr), &Value::Array(ref t_arr)) => {
        let borrow = arr.borrow();
        if borrow.values.len() != t_arr.len() {
          return false;
        }
        let len = borrow.values.len();
        for i in 0..len {
          if !Parser::same_structure(&borrow.values[i].val, &t_arr[i]) {
            return false;
          }
        }
        true
      },
      (&TOMLValue::InlineTable(ref it), &Value::InlineTable(ref t_it)) => {
        let borrow = it.borrow();
        if borrow.keyvals.len() != t_it.len() {
          return false;
        }
        let len = borrow.keyvals.len();
        for i in 0..len {
          if borrow.keyvals[i].keyval.key != t_it[i].0 ||
            !Parser::same_structure(&borrow.keyvals[i].keyval.val, &t_it[i].1) {
            return false;
          }
        }
        true
      },
      (&TOMLValue::Array(_), _)           => false, // Array replaced with scalar
      (&TOMLValue::InlineTable(_), _)     => false, // InlineTable replaced with scalar
      (_, &Value::Array(_))       => false, // scalar replaced with an Array
      (_, &Value::InlineTable(_)) => false, // scalar replaced with an InlineTable
      (_,_)                           => true,  // scalar replaced with any other scalar
    }
  }

  fn rebuild_vector(self: &mut Parser<'a>, key: String, val: Rc<RefCell<TOMLValue<'a>>>, skip: bool) {
    match *val.borrow() {
      TOMLValue::Array(ref arr) => {
        {
          let value = match self.map.entry(key.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            _ => panic!("Map contains array key, but map.entry returned a Vacant entry in set_value."),
          };
          if !skip {
            value.value = Some(val.clone());
          }
          value.subkeys = Children::Count(Cell::new(arr.borrow().values.len()));
        }
        for i in 0..arr.borrow().values.len() {
          let subkey = format!("{}[{}]", key, i);
          self.rebuild_vector(subkey, arr.borrow().values[i].val.clone(), false);
        }
      },
      TOMLValue::InlineTable(ref it) => {
        {
          let value = match self.map.entry(key.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            _ => panic!("Map contains inline table key, but map.entry returned a Vacant entry is set_value."),
          };
          if !skip {
            value.value = Some(val.clone());
          }
          if let Children::Keys(ref child_keys) = value.subkeys {
            for i in 0..it.borrow().keyvals.len() {
              Parser::insert(child_keys, it.borrow().keyvals[i].keyval.key.clone().into_owned());
            }
          }
        }
        for i in 0..it.borrow().keyvals.len() {
          let subkey = format!("{}.{}", key, &it.borrow().keyvals[i].keyval.key);
          self.rebuild_vector(subkey, it.borrow().keyvals[i].keyval.val.clone(), false);
        }
      },
      _ => {
        self.map.entry(key).or_insert_with(|| HashValue::new_count(val.clone()));
      },
    }
  }

  fn get_all_subkeys(self: &Parser<'a>, key: &str) -> Vec<String>{
    let hv_opt = self.map.get(key);
    let mut all_keys = vec![];
    if let Some(hv) = hv_opt {
      let children = &hv.subkeys;
      match *children {
        Children::Count(ref cell) => {
          for i in 0..cell.get() {
            let subkey = format!("{}[{}]", key, i);
            all_keys.push(subkey.clone());
            all_keys.append(&mut self.get_all_subkeys(&subkey));
          }
        },
        Children::Keys(ref rc_hs) => {
          for childkey in rc_hs.borrow().iter(){
            let subkey = format!("{}.{}", key, childkey);
            all_keys.push(subkey.clone());
            all_keys.append(&mut self.get_all_subkeys(&subkey));
          }
        },
      }
    }
    all_keys
  }

  fn replace_values(val_rf: &Rc<RefCell<TOMLValue<'a>>>, tval: &Value<'a>) -> bool {
    let value = match (&*val_rf.borrow(), tval) {
      (&TOMLValue::Array(ref arr), &Value::Array(ref t_arr)) => {
        let borrow = arr.borrow();
        let len = borrow.values.len();
        for i in 0..len {
          if !Parser::replace_values(&borrow.values[i].val, &t_arr[i]) {
            return false;
          }
        }
        return true;
      },
      (&TOMLValue::InlineTable(ref it), &Value::InlineTable(ref t_it)) => {
        let borrow = it.borrow();
        let len = borrow.keyvals.len();
        for i in 0..len {
          if !Parser::replace_values(&borrow.keyvals[i].keyval.val, &t_it[i].1) {
            return false;
          }
        }
        return true;
      },
      (_, &Value::Integer(ref v))   => TOMLValue::Integer(v.clone()),
      (_, &Value::Float(ref v))     => TOMLValue::Float(v.clone()),
      (_, &Value::Boolean(v))       => TOMLValue::Boolean(v),
      (_, &Value::DateTime(ref v))  => TOMLValue::DateTime(v.clone()),
      (_, &Value::String(ref s, t)) => TOMLValue::String(s.clone(), t),
      (v, tv)                             => panic!("Check for the same structure should have eliminated the possibility of replacing {} with {}", v, tv),
    };
    *val_rf.borrow_mut() = value;
    true
  }

  pub fn sanitize_array(arr: Rc<RefCell<Array<'a>>>) -> Value<'a> {
    let mut result: Vec<Value> = vec![];
    for av in &arr.borrow().values {
      result.push(to_val!(&*av.val.borrow()));
    }
    Value::Array(Rc::new(result))
  }

  pub fn sanitize_inline_table(it: Rc<RefCell<InlineTable<'a>>>) -> Value<'a> {
    let mut result: Vec<(Cow<'a, str>, Value)> = vec![];
    for kv in &it.borrow().keyvals {
      result.push((kv.keyval.key.clone(), to_val!(&*kv.keyval.val.borrow())));
    }
    Value::InlineTable(Rc::new(result))
  }
}

impl<'a> Display for Parser<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", *self.root.borrow())
  }
}

#[cfg(test)]
mod test {
  extern crate env_logger;
  use std::cell::{Cell, RefCell};
  use std::rc::Rc;
  use crate::internals::parser::Parser;
  use crate::types::{Value, Children, StrType, Date, Time, DateTime};
  struct TT;
  impl TT {
    fn get<'a>() -> &'a str {
      return r#"animal = "bear"

[[car.owners]]
Name = """Bob Jones"""
Age = 25
[[car.owners]]
Name = 'Jane Doe'
Age = 44

[car.interior.seats]
type = '''fabric'''
count = 5

[car]
model = "Civic"
"ωλèèℓƨ" = 4
"ƭôƥ ƨƥèèδ" = 124.56
"Date of Manufacture" = 2007-05-16T10:12:13.2324+04:00
drivers = ["Bob", "Jane", "John", "Michael", { disallowed = "Chris", banned="Sally"}]
properties = { color = "red", "plate number" = "ABC 345",
               accident_dates = [2008-09-29, 2011-01-16, 2014-11-30T03:13:54]}
"#;
    }
  }

  #[cfg(test)]
  #[ctor::ctor]
  fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
  }

  #[test]
  fn test_bare_key() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_value("animal".to_string()), res2opt!(Value::basic_string("bear")));
  }

  #[test]
  fn test_key_val() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_value("car.model"), res2opt!(Value::basic_string("Civic")));
  }

  #[test]
  fn test_quoted_key_val_int() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_value("car.\"ωλèèℓƨ\""), res2opt!(Value::int_from_str("4")));
  }

  #[test]
  fn test_quoted_key_val_float() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_value("car.\"ƭôƥ ƨƥèèδ\""), res2opt!(Value::float_from_str("124.56")));
  }

  #[test]
  fn test_key_array() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_value("car.drivers[0]"), res2opt!(Value::basic_string("Bob")));
    assert_eq!(p.get_value("car.drivers[1]"), res2opt!(Value::basic_string("Jane")));
    assert_eq!(p.get_value("car.drivers[2]"), res2opt!(Value::basic_string("John")));
    assert_eq!(p.get_value("car.drivers[3]"), res2opt!(Value::basic_string("Michael")));
    assert_eq!(p.get_value("car.drivers[4].disallowed"), res2opt!(Value::basic_string("Chris")));
    assert_eq!(p.get_value("car.drivers[4].banned"), res2opt!(Value::basic_string("Sally")));
  }

  #[test]
  fn test_key_inline_table() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_value("car.properties.color"), res2opt!(Value::basic_string("red")));
    assert_eq!(p.get_value("car.properties.\"plate number\""), res2opt!(Value::basic_string("ABC 345")));
    assert_eq!(p.get_value("car.properties.accident_dates[0]"), res2opt!(Value::date_from_str("2008", "09", "29")));
    assert_eq!(p.get_value("car.properties.accident_dates[1]"), res2opt!(Value::date_from_int(2011, 1, 16)));
    assert_eq!(p.get_value("car.properties.accident_dates[2]"), res2opt!(Value::datetime_from_str("2014", "11", "30", "03", "13", "54")));
  }

  #[test]
  fn test_implicit_table() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_value("car.interior.seats.type"), res2opt!(Value::ml_literal_string("fabric")));
    assert_eq!(p.get_value("car.interior.seats.count"), res2opt!(Value::int_from_str("5")));
  }

  #[test]
  fn test_array_table() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_value("car.owners[0].Name"), res2opt!(Value::ml_basic_string("Bob Jones")));
    assert_eq!(p.get_value("car.owners[0].Age"), Some(Value::int(25)));
    assert_eq!(p.get_value("car.owners[1].Name"), res2opt!(Value::literal_string("Jane Doe")));
    assert_eq!(p.get_value("car.owners[1].Age"), Some(Value::int(44)));
  }

  #[test]
  fn test_get_root_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children(""), Some(&Children::Keys(RefCell::new(vec!["animal".to_string(), "car".to_string()]))));
  }

  #[test]
  fn test_get_table_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car".to_string()),
      Some(&Children::Keys(RefCell::new(vec!["owners".to_string(), "interior".to_string(), "model".to_string(), "\"ωλèèℓƨ\"".to_string(), "\"ƭôƥ ƨƥèèδ\"".to_string(),
        "\"Date of Manufacture\"".to_string(), "drivers".to_string(), "properties".to_string(),
      ]))));
  }

  #[test]
  fn test_get_array_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car.drivers"), Some(&Children::Count(Cell::new(5))));
  }

  #[test]
  fn test_get_inline_table_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car.properties"),
      Some(&Children::Keys(RefCell::new(vec!["color".to_string(), "\"plate number\"".to_string(), "accident_dates".to_string()]))));
  }

  #[test]
  fn test_get_nested_inline_table_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car.drivers[4]"),
      Some(&Children::Keys(RefCell::new(vec!["disallowed".to_string(), "banned".to_string()]))));
  }

  #[test]
  fn test_get_nested_array_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car.properties.accident_dates"), Some(&Children::Count(Cell::new(3))));
  }

  #[test]
  fn test_implicit_table_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car.interior.seats"),
      Some(&Children::Keys(RefCell::new(vec!["type".to_string(), "count".to_string()]))));
  }

  #[test]
  fn test_get_array_of_table_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car.owners"), Some(&Children::Count(Cell::new(2))));
  }

  #[test]
  fn test_get_array_of_table0_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car.owners[0]"),
      Some(&Children::Keys(RefCell::new(vec!["Name".to_string(), "Age".to_string()]))));
  }

  #[test]
  fn test_get_array_of_table1_children() {
    let p = Parser::new();
    let (p, _) = p.parse(TT::get());
    assert_eq!(p.get_children("car.owners[1]"),
      Some(&Children::Keys(RefCell::new(vec!["Name".to_string(), "Age".to_string()]))));
  }

  #[test]
  fn test_set_bare_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("animal", Value::ml_basic_string("shark").unwrap());
    assert_eq!(p.get_value("animal"),
      Some(Value::String("shark".into(), StrType::MLBasic)));
  }

  #[test]
  fn test_set_table_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.model", Value::literal_string("Accord").unwrap());
    assert_eq!(p.get_value("car.model"),
      Some(Value::String("Accord".into(), StrType::Literal)));
  }

  #[test]
  fn test_set_array_element_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.drivers[1]", Value::ml_literal_string("Mark").unwrap());
    assert_eq!(p.get_value("car.drivers[1]"),
      Some(Value::String("Mark".into(), StrType::MLLiteral)));
  }

  #[test]
  fn test_set_nested_aray_element_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.properties.accident_dates[2]", Value::float(3443.34));
    assert_eq!(p.get_value("car.properties.accident_dates[2]"),
      Some(Value::Float("3443.34".into())));
  }

  #[test]
  fn test_set_inline_table_element_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.properties.color", Value::int(19));
    assert_eq!(p.get_value("car.properties.color"),
      Some(Value::Integer("19".into())));
  }

  #[test]
  fn test_set_nested_inline_table_element_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.drivers[4].banned", Value::datetime_from_int(2013, 9, 23, 17, 34, 2).unwrap());
    assert_eq!(p.get_value("car.drivers[4].banned"),
      Some(Value::DateTime(DateTime::new(Date::new_str("2013", "09", "23"),
        Some(Time::new_str("17", "34", "02", None, None))))));
  }

  #[test]
  fn test_truncate_array() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.drivers", Value::Array(Rc::new(
      vec![Value::basic_string("Phil").unwrap(), Value::basic_string("Mary").unwrap()]
    )));
    assert_eq!(p.get_value("car.drivers"),
      Some(Value::Array(Rc::new(
        vec![Value::String("Phil".into(), StrType::Basic),
             Value::String("Mary".into(), StrType::Basic)
        ]
      ))));
  }

  #[test]
  fn test_truncate_inline_table() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.properties", Value::InlineTable(Rc::new(
      vec![("make".into(), Value::literal_string("Honda").unwrap()),
           ("transmission".into(), Value::bool(true))]
    )));
    assert_eq!(p.get_value("car.properties"),
      Some(Value::InlineTable(Rc::new(
        vec![("make".into(), Value::String("Honda".into(), StrType::Literal)),
             ("transmission".into(), Value::Boolean(true))]
      ))));
  }

  #[test]
  fn test_extend_array() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.drivers", Value::Array(Rc::new(
      vec![Value::int(1), Value::int(2), Value::int(3), Value::int(4),
      Value::int(5), Value::int(6), Value::int(7), Value::int(8)]
    )));
    assert_eq!(p.get_value("car.drivers"),
      Some(Value::Array(Rc::new(
        vec![Value::Integer("1".into()),
             Value::Integer("2".into()),
             Value::Integer("3".into()),
             Value::Integer("4".into()),
             Value::Integer("5".into()),
             Value::Integer("6".into()),
             Value::Integer("7".into()),
             Value::Integer("8".into()),
        ]
      ))));
  }

  #[test]
  fn test_extend_inline_table() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.properties", Value::InlineTable(Rc::new(
      vec![("prop1".into(), Value::bool_from_str("TrUe").unwrap()),
           ("prop2".into(), Value::bool_from_str("FALSE").unwrap()),
           ("prop3".into(), Value::bool_from_str("truE").unwrap()),
           ("prop4".into(), Value::bool_from_str("false").unwrap())]
    )));
    assert_eq!(p.get_value("car.properties"),
      Some(Value::InlineTable(Rc::new(
        vec![("prop1".into(), Value::Boolean(true)),
             ("prop2".into(), Value::Boolean(false)),
             ("prop3".into(), Value::Boolean(true)),
             ("prop4".into(), Value::Boolean(false))]
      ))));
  }

  #[test]
  fn test_set_implicit_table_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.interior.seats.type", Value::basic_string("leather").unwrap());
    assert_eq!(p.get_value("car.interior.seats.type"),
      Some(Value::String("leather".into(), StrType::Basic)));
  }

  #[test]
  fn test_set_array_of_table0_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.owners[0].Age", Value::float_from_str("19.5").unwrap());
    assert_eq!(p.get_value("car.owners[0].Age"),
      Some(Value::Float("19.5".into())));
  }

  #[test]
  fn test_set_array_of_table1_key() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("car.owners[1].Name", Value::ml_basic_string("Steve Parker").unwrap());
    assert_eq!(p.get_value("car.owners[1].Name"),
      Some(Value::String("Steve Parker".into(), StrType::MLBasic)));
  }

  #[test]
  fn test_truncate_array_check_keys() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("database.ports", Value::datetime_from_int(2000, 02, 16, 10, 31, 06).unwrap());
    assert_eq!(p.get_value("database.ports[0]"), None);
    assert_eq!(p.get_value("database.ports[1]"), None);
    assert_eq!(p.get_value("database.ports[2]"), None);
    assert_eq!(p.get_value("database.ports[2][0]"), None);
    assert_eq!(p.get_value("database.ports[2][1]"), None);
  }

  #[test]
  fn test_truncate_inline_table_check_keys() {
    let p = Parser::new();
    let (mut p, _) = p.parse(TT::get());
    p.set_value("database.servers", Value::datetime_from_str("4000", "02", "27", "01", "59", "59").unwrap());
    assert_eq!(p.get_value("database.servers.main"), None);
    assert_eq!(p.get_value("database.servers.failover1"), None);
    assert_eq!(p.get_value("database.servers.failover2"), None);
    assert_eq!(p.get_value("database.servers.failover2.something"), None);
    assert_eq!(p.get_value("database.servers.failover2.nothing"), None);
  }
}