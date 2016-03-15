extern crate tomllib;
extern crate env_logger;
use tomllib::TOMLParser;
use tomllib::types::{ParseResult, Value, ParseError, Children};
use std::rc::Rc;
use std::cell::{Cell, RefCell};



#[test]
fn test_output_after_set() {
  let _ = env_logger::init();
  let p = TOMLParser::new();
  let (mut p, _) = p.parse(r#"animal = "bear"

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
"#);
  p.set_value("car.interior.seats.type", Value::basic_string("leather").unwrap());
  p.set_value("car.interior.seats.type", Value::basic_string("vinyl").unwrap());
  p.set_value("car.owners[0].Age", Value::float_from_str("19.5").unwrap());
  p.set_value("car.owners[1].Name", Value::ml_basic_string("Steve Parker").unwrap());
  p.set_value("car.drivers[4].banned", Value::datetime_from_int(2013, 9, 23, 17, 34, 2).unwrap());
  p.set_value("car.properties.color", Value::int(19));
  p.set_value("car.properties.accident_dates[2]", Value::float(3443.34));
  p.set_value("car.drivers[1]", Value::ml_literal_string("Mark").unwrap());
  p.set_value("car.properties", Value::InlineTable(Rc::new(
    vec![("make".into(), Value::literal_string("Honda").unwrap()),
          ("transmission".into(), Value::bool(true))]
  )));
  p.set_value("car.drivers", Value::Array(Rc::new(
    vec![Value::basic_string("Phil").unwrap(), Value::basic_string("Mary").unwrap()]
  )));
  p.set_value("car.properties", Value::InlineTable(Rc::new(
    vec![("prop1".into(), Value::bool_from_str("TrUe").unwrap()),
          ("prop2".into(), Value::bool_from_str("FALSE").unwrap()),
          ("prop3".into(), Value::bool_from_str("truE").unwrap()),
          ("prop4".into(), Value::bool_from_str("false").unwrap())]
  )));
  p.set_value("car.drivers", Value::Array(Rc::new(
    vec![Value::int(1), Value::int(2), Value::int(3), Value::int(4),
    Value::int(5), Value::int(6), Value::int(7), Value::int(8)]
  )));
  p.set_value("car.model", Value::literal_string("Accord").unwrap());
  p.set_value("animal", Value::ml_basic_string("shark").unwrap());
  assert_eq!(r#"animal = """shark"""

[[car.owners]]
Name = """Bob Jones"""
Age = 19.5
[[car.owners]]
Name = """Steve Parker"""
Age = 44

[car.interior.seats]
type = "vinyl"
count = 5

[car]
model = 'Accord'
"ωλèèℓƨ" = 4
"ƭôƥ ƨƥèèδ" = 124.56
"Date of Manufacture" = 2007-05-16T10:12:13.2324+04:00
drivers = [1, 2, 3, 4, 5, 6, 7, 8]
properties = { prop1 = true, prop2 = false, prop3 = true, prop4 = false }
"#, format!("{}", p));
}

struct TT;
impl TT {
  fn get<'a>() -> &'a str {
    return r#"fish = "halibut"
[[foo."bar"]]
baz = 12345
qux = 1981-04-15
[[foo."bar"]]
baz = 'something'
qux = '''other'''
[[foo.quality]]
furniture = "chair"
[foo.quality.machine.parts.service]
"ƥèřïôδ" = 24.7
"inline table" = { drink = 5.5, meal = [ { start = 5 }, { start = 1980-05-14, end = 2002-10-19 } ], dessert = '''cake''' }
[[foo.quality.labor]]
Name = 'Rïçλářδ'
[[foo.quality.labor]]
Name = '§ƭèřℓïñϱ Âřçλèř'
[[foo.quality]]
money = 789.0123
[[foo."bar"]]
baz = 2016-03-10T12:31:02+07:30
qux = """ƒáβúℓôúƨ δïñôƨáúř"""
array = [{one = 1}, {two = 2}, {you = ["good", """bye"""], fire = "truck"}, {three = 3}]
[foo]
"δïáϱñôƨïƨ" = true
"ƥřôϱñôƨïƨ" = "not good"
hypnosis = 987654321
"#;
  }
}

#[test]
fn test_mixed_tables() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (_, result) = parser.parse(TT::get());
  assert!(result == ParseResult::Full, "Parse of mixed tables document had errors when it shouldn't have.");
}

fn check_errors(result: &ParseResult) -> (bool, String) {
  let mut leftover = "".to_string();
  let mut opt_errors: Option<&Rc<RefCell<Vec<ParseError>>>> = None;
  match result {
    &ParseResult::FullError(ref e) => {opt_errors = Some(e);},
    &ParseResult::Partial(ref l,_,_) => {leftover = l.clone().into_owned();},
    &ParseResult::PartialError(ref l,_,_,ref e) => {opt_errors = Some(e); leftover = l.clone().into_owned();},
    &ParseResult::Failure(ref line,_) => {assert!(false, "failed at line number: {}", line);},
    _ => (),
  }
  let mut full_error = "".to_string();
  if let Some(errors) = opt_errors {
    for error in errors.borrow().iter() {
      match error {
        &ParseError::MixedArray(ref key, _, _) => full_error.push_str(&format!("MixedArray error: {}\n", key)),
        &ParseError::DuplicateKey(ref key, _, _, _) => full_error.push_str(&format!("Duplicate key error: {}\n", key)),
        &ParseError::InvalidTable(ref key, _, _, _) => full_error.push_str(&format!("Invalid table error: {}\n", key)),
        &ParseError::InvalidDateTime(ref key, _, _, _) => full_error.push_str(&format!("Invalid datetime error: {}\n", key)),
        _ => full_error.push_str("Some other error was encountered."),
      }
    }
  }
  if leftover != "" {
    full_error.push_str(&format!("$$ Leftover =\n\"{}\"", leftover));
    return (false, full_error);
  }
  (true, "".to_string())
}

#[test]
fn test_mixed_mixed_array_fail() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (_, result) = parser.parse(r#"[[foo."bar"]]
baz = 2016-03-10T12:31:02+07:30
qux = """ƒáβúℓôúƨ δïñôƨáúř"""
array = [1, 2, {you = ["good", """bye"""], fire = "truck"}, 3]
[owner]
a_key = "a value"
"#);
  assert!(check_errors(&result).0, "There should have been a mixed array error, but there wasn't.");
  let errors = match result {
    ParseResult::FullError(e) => e,
    ParseResult::PartialError(_,_,_,e) => e,
    _ => panic!("There should have been a mixed array error, but there wasn't."),
  };
  let error = &errors.borrow()[0];
  if let &ParseError::MixedArray(ref key, line, _col) = error {
    assert!(key == "foo.\"bar\"[0].array" && line == 4,
      "key should be \"foo.\"bar\"[0].array\", but is: \"{}\", line number should be 4, but is: {}",
        key, line);
  } else {
    assert!(false, "The first error should have been a mixed array error, but it wasn't.");
  }
}

#[test]
fn test_mixed_mixed_array_in_inline_table_fail() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (_, result) = parser.parse(r#"[foo.quality.machine.parts.service]
"ƥèřïôδ" = 24.7
"inline table" = { drink = 5.5, meal = [ 6, { start = 1980-05-14, end = 2002-10-19 } ], dessert = '''cake''' }
[owner]
a_key = "a value"
"#);
  assert!(check_errors(&result).0, "There should have been a mixed array error, but there wasn't.");
  let errors = match result {
    ParseResult::FullError(e) => e,
    ParseResult::PartialError(_,_,_,e) => e,
    _ => panic!("There should have been a mixed array error, but there wasn't."),
  };
  let error = &errors.borrow()[0];
  if let &ParseError::MixedArray(ref key, line, _col) = error {
    assert!(key == "foo.quality.machine.parts.service.\"inline table\".meal" && line == 3,
      "key should be \"foo.quality.machine.parts.service.\"inline table\".meal\", but is: \"{}\", line number should be 3, but is: {}",
        key, line);
  } else {
    assert!(false, "The first error should have been a mixed array error, but it wasn't.");
  }
}

#[test]
fn test_basic_get_on_mixed_tables() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (parser, _) = parser.parse(TT::get());
  
  // test getting bare key
  assert_eq!(Value::basic_string("halibut").unwrap(), parser.get_value("fish").unwrap());
  // test table key lookup
  assert_eq!(Value::Boolean(true), parser.get_value("foo.\"δïáϱñôƨïƨ\"").unwrap());
  assert_eq!(Value::basic_string("not good").unwrap(), parser.get_value("foo.\"ƥřôϱñôƨïƨ\"").unwrap());
  assert_eq!(Value::int(987654321), parser.get_value("foo.hypnosis").unwrap());
  // test array table key lookup
  assert_eq!(Value::int(12345), parser.get_value("foo.\"bar\"[0].baz").unwrap());
  assert_eq!(Value::date_from_int(1981, 4, 15).unwrap(), parser.get_value("foo.\"bar\"[0].qux").unwrap());
  assert_eq!(Value::literal_string("something").unwrap(), parser.get_value("foo.\"bar\"[1].baz").unwrap());
  assert_eq!(Value::ml_literal_string("other").unwrap(), parser.get_value("foo.\"bar\"[1].qux").unwrap());
  // Test sibling array of tables
  assert_eq!(Value::basic_string("chair").unwrap(), parser.get_value("foo.quality[0].furniture").unwrap());
  // Test standard table nested in array of tables
  assert_eq!(Value::float(24.7), parser.get_value("foo.quality[0].machine.parts.service.\"ƥèřïôδ\"").unwrap());
  // Test inline table value
  assert_eq!(Value::float(5.5), parser.get_value("foo.quality[0].machine.parts.service.\"inline table\".drink").unwrap());
  // Test nested inline table in array in inline table
  assert_eq!(Value::int(5), parser.get_value("foo.quality[0].machine.parts.service.\"inline table\".meal[0].start").unwrap());
  assert_eq!(Value::date_from_int(2002, 10, 19).unwrap(), parser.get_value("foo.quality[0].machine.parts.service.\"inline table\".meal[1].end").unwrap());
  // Test child array of tables
  assert_eq!(Value::literal_string("Rïçλářδ").unwrap(), parser.get_value("foo.quality[0].labor[0].Name").unwrap());
  assert_eq!(Value::literal_string("§ƭèřℓïñϱ Âřçλèř").unwrap(), parser.get_value("foo.quality[0].labor[1].Name").unwrap());
  // Test new parent array of tables
  assert_eq!(Value::float(789.0123), parser.get_value("foo.quality[1].money").unwrap());
  // Test restarting adding tables to previously defined array of tables
  assert_eq!(Value::datetime_offset_from_int(2016, 3, 10, 12, 31, 2, "+", 7, 30).unwrap(), parser.get_value("foo.\"bar\"[2].baz").unwrap());
  assert_eq!(Value::ml_basic_string("ƒáβúℓôúƨ δïñôƨáúř").unwrap(), parser.get_value("foo.\"bar\"[2].qux").unwrap());
  // Test inline table nested in array
  assert_eq!(Value::int(1), parser.get_value("foo.\"bar\"[2].array[0].one").unwrap());
  // Test array nested in inline table in array
  assert_eq!(Value::basic_string("good").unwrap(), parser.get_value("foo.\"bar\"[2].array[2].you[0]").unwrap());
  assert_eq!(Value::ml_basic_string("bye").unwrap(), parser.get_value("foo.\"bar\"[2].array[2].you[1]").unwrap());
  assert_eq!(Value::basic_string("truck").unwrap(), parser.get_value("foo.\"bar\"[2].array[2].fire").unwrap());
  assert_eq!(Value::int(3), parser.get_value("foo.\"bar\"[2].array[3].three").unwrap());
}

#[test]
fn test_get_children_on_mixed_tables() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (parser, _) = parser.parse(TT::get());
  // test getting bare key
  assert_eq!(&Children::Count(Cell::new(0)), parser.get_children("fish").unwrap());
  // test table key lookup
  assert_eq!(&Children::Count(Cell::new(0)), parser.get_children("foo.\"δïáϱñôƨïƨ\"").unwrap());
  // test array table key lookup
  assert_eq!(&Children::Count(Cell::new(3)), parser.get_children("foo.\"bar\"").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["baz".to_string(), "qux".to_string()])), parser.get_children("foo.\"bar\"[0]").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["baz".to_string(), "qux".to_string()])), parser.get_children("foo.\"bar\"[1]").unwrap());
  assert_eq!(&Children::Count(Cell::new(2)), parser.get_children("foo.quality").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["furniture".to_string(), "machine".to_string(), "labor".to_string()])), parser.get_children("foo.quality[0]").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["\"ƥèřïôδ\"".to_string(), "\"inline table\"".to_string()])), parser.get_children("foo.quality[0].machine.parts.service").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["drink".to_string(), "meal".to_string(), "dessert".to_string()])), parser.get_children("foo.quality[0].machine.parts.service.\"inline table\"").unwrap());
  assert_eq!(&Children::Count(Cell::new(2)), parser.get_children("foo.quality[0].machine.parts.service.\"inline table\".meal").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["start".to_string(), "end".to_string()])), parser.get_children("foo.quality[0].machine.parts.service.\"inline table\".meal[1]").unwrap());
  assert_eq!(&Children::Count(Cell::new(2)), parser.get_children("foo.quality[0].labor").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["Name".to_string()])), parser.get_children("foo.quality[0].labor[0]").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["Name".to_string()])), parser.get_children("foo.quality[0].labor[1]").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["money".to_string()])), parser.get_children("foo.quality[1]").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["baz".to_string(), "qux".to_string(), "array".to_string()])), parser.get_children("foo.\"bar\"[2]").unwrap());
  assert_eq!(&Children::Count(Cell::new(4)), parser.get_children("foo.\"bar\"[2].array").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["one".to_string()])), parser.get_children("foo.\"bar\"[2].array[0]").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["two".to_string()])), parser.get_children("foo.\"bar\"[2].array[1]").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["you".to_string(), "fire".to_string()])), parser.get_children("foo.\"bar\"[2].array[2]").unwrap());
  assert_eq!(&Children::Count(Cell::new(2)), parser.get_children("foo.\"bar\"[2].array[2].you").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["three".to_string()])), parser.get_children("foo.\"bar\"[2].array[3]").unwrap());
  assert_eq!(&Children::Keys(RefCell::new(vec!["\"bar\"".to_string(), "quality".to_string(), "\"δïáϱñôƨïƨ\"".to_string(), "\"ƥřôϱñôƨïƨ\"".to_string(), "hypnosis".to_string()])), parser.get_children("foo").unwrap());
}

#[test]
fn test_basic_set_then_get_on_mixed_tables() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (mut parser, _) = parser.parse(TT::get());
  // test setting bare key
  assert!(parser.set_value("fish", Value::int(1)));
  // test table key set
  assert!(parser.set_value("foo.\"δïáϱñôƨïƨ\"", Value::int(2)));
  assert!(parser.set_value("foo.\"ƥřôϱñôƨïƨ\"", Value::float(1.2)));
  assert!(parser.set_value("foo.hypnosis", Value::float(2.3)));
  // test array table key set
  assert!(parser.set_value("foo.\"bar\"[0].baz", Value::float(3.4)));
  assert!(parser.set_value("foo.\"bar\"[0].qux", Value::Boolean(true)));
  assert!(parser.set_value("foo.\"bar\"[1].baz", Value::Boolean(false)));
  assert!(parser.set_value("foo.\"bar\"[1].qux", Value::basic_string("ONE").unwrap()));
  // Test sibling array of tables set
  assert!(parser.set_value("foo.quality[0].furniture", Value::date_from_int(2000, 1, 1).unwrap()));
  // Test standard table nested in array of tables set
  assert!(parser.set_value("foo.quality[0].machine.parts.service.\"ƥèřïôδ\"", Value::datetime_from_int(2001, 2, 2, 1, 1, 1).unwrap()));
  // Test inline table value set
  assert!(parser.set_value("foo.quality[0].machine.parts.service.\"inline table\"", Value::InlineTable(
    Rc::new(vec![
      ("KEYONE".into(), Value::literal_string("VALUEONE").unwrap()),
      ("KEYTWO".into(), Value::literal_string("VALUETWO").unwrap())
    ])
  )));
  // Test child array of tables set
  assert!(parser.set_value("foo.quality[0].labor[0].Name", Value::ml_basic_string("TWO").unwrap()));
  assert!(parser.set_value("foo.quality[0].labor[1].Name", Value::ml_literal_string("THREE").unwrap()));
  // Test new parent array of tables set
  assert!(parser.set_value("foo.quality[1].money", Value::datetime_from_string("2002-03-03T02:02:02.01234-08:35").unwrap()));
  // Test restarting adding tables to previously defined array of tables set
  assert!(parser.set_value("foo.\"bar\"[2].baz", Value::datetime_from_string("2003-04-04").unwrap()));
  assert!(parser.set_value("foo.\"bar\"[2].qux", Value::datetime_from_string("2004-05-05T03:03:03").unwrap()));
  // Test resetting array
  assert!(parser.set_value("foo.\"bar\"[2].array", Value::Array(Rc::new(vec![
    Value::int(3), Value::int(4), Value::int(5), Value::int(6), Value::int(7)
  ]))));
  
  
  // test getting bare key
  assert_eq!(Value::int(1), parser.get_value("fish").unwrap());
  // test table key lookup
  assert_eq!(Value::int(2), parser.get_value("foo.\"δïáϱñôƨïƨ\"").unwrap());
  assert_eq!(Value::float(1.2), parser.get_value("foo.\"ƥřôϱñôƨïƨ\"").unwrap());
  assert_eq!(Value::float(2.3), parser.get_value("foo.hypnosis").unwrap());
  // test array table key lookup
  assert_eq!(Value::float(3.4), parser.get_value("foo.\"bar\"[0].baz").unwrap());
  assert_eq!(Value::Boolean(true), parser.get_value("foo.\"bar\"[0].qux").unwrap());
  assert_eq!(Value::Boolean(false), parser.get_value("foo.\"bar\"[1].baz").unwrap());
  assert_eq!(Value::basic_string("ONE").unwrap(), parser.get_value("foo.\"bar\"[1].qux").unwrap());
  //Test sibling array of tables
  assert_eq!(Value::date_from_int(2000, 1, 1).unwrap(), parser.get_value("foo.quality[0].furniture").unwrap());
  // Test standard table nested in array of tables
  assert_eq!(Value::datetime_from_int(2001, 2, 2, 1, 1, 1).unwrap(), parser.get_value("foo.quality[0].machine.parts.service.\"ƥèřïôδ\"").unwrap());
  // Test inline table value
  assert_eq!(Value::literal_string("VALUEONE").unwrap(), parser.get_value("foo.quality[0].machine.parts.service.\"inline table\".KEYONE").unwrap());
  assert_eq!(Value::literal_string("VALUETWO").unwrap(), parser.get_value("foo.quality[0].machine.parts.service.\"inline table\".KEYTWO").unwrap());
  // Test child array of tables
  assert_eq!(Value::ml_basic_string("TWO").unwrap(), parser.get_value("foo.quality[0].labor[0].Name").unwrap());
  assert_eq!(Value::ml_literal_string("THREE").unwrap(), parser.get_value("foo.quality[0].labor[1].Name").unwrap());
  // Test new parent array of tables
  assert_eq!(Value::datetime_from_string("2002-03-03T02:02:02.01234-08:35").unwrap(), parser.get_value("foo.quality[1].money").unwrap());
  // Test restarting adding tables to previously defined array of tables
  assert_eq!(Value::datetime_from_string("2003-04-04").unwrap(), parser.get_value("foo.\"bar\"[2].baz").unwrap());
  assert_eq!(Value::datetime_from_string("2004-05-05T03:03:03").unwrap(), parser.get_value("foo.\"bar\"[2].qux").unwrap());
  // Test inline table nested in array
  assert_eq!(Value::int(3), parser.get_value("foo.\"bar\"[2].array[0]").unwrap());
  assert_eq!(Value::int(4), parser.get_value("foo.\"bar\"[2].array[1]").unwrap());
  assert_eq!(Value::int(5), parser.get_value("foo.\"bar\"[2].array[2]").unwrap());
  assert_eq!(Value::int(6), parser.get_value("foo.\"bar\"[2].array[3]").unwrap());
  assert_eq!(Value::int(7), parser.get_value("foo.\"bar\"[2].array[4]").unwrap());
}

#[test]
fn test_invalid_table_fail() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (_, result) = parser.parse(r#"[foo.quality]
"ƥèřïôδ" = 24.7
[owner]
a_key = "a value"
[foo.quality]
KEYONE = "VALUEONE"
KEYTWO = "VALUETWO"
"#);
  assert!(check_errors(&result).0, "There should have been an invalid table error, but there wasn't.");
  let errors = match result {
    ParseResult::FullError(e) => e,
    ParseResult::PartialError(_,_,_,e) => e,
    _ => panic!("There should have been an invalid table error, but there wasn't."),
  };
  let error = &errors.borrow()[0];
  if let &ParseError::InvalidTable(ref key, line, _col, ref rc_hm) = error {
    assert!(key == "foo.quality" && line == 5,
      "key should be \"foo.quality\", but is: \"{}\", line number should be 5, but is: {}",
        key, line);
     assert_eq!(&Value::basic_string("VALUEONE").unwrap(), rc_hm.borrow().get("KEYONE").unwrap());
     assert_eq!(&Value::basic_string("VALUETWO").unwrap(), rc_hm.borrow().get("KEYTWO").unwrap());
  } else {
    assert!(false, "The first error should have been an invalid table error, but it wasn't.");
  }
}

#[test]
fn test_duplicate_key_fail() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (_, result) = parser.parse(r#"[foo.quality]
"ƥèřïôδ" = 24.7
[owner]
a_key = "a value"
a_key = 'ANOTHER VALUE'
[foo.quality]
KEYONE = "VALUEONE"
"#);
  assert!(check_errors(&result).0, "There should have been a duplicate key error, but there wasn't.");
  let errors = match result {
    ParseResult::FullError(e) => e,
    ParseResult::PartialError(_,_,_,e) => e,
    _ => panic!("There should have been a duplicate key error, but there wasn't."),
  };
  let error = &errors.borrow()[0];
  if let &ParseError::DuplicateKey(ref key, line, _col, ref val) = error {
    assert!(key == "owner.a_key" && line == 5,
      "key should be \"owner.a_key\", but is: \"{}\", line number should be 5, but is: {}",
        key, line);
     assert_eq!(&Value::literal_string("ANOTHER VALUE").unwrap(), val);
  } else {
    assert!(false, "The first error should have been a duplicate key error, but it wasn't.");
  }
}

#[test]
fn test_invalid_datetime_fail() {
  let _ = env_logger::init();

  let parser = TOMLParser::new();
  let (_, result) = parser.parse(r#"[foo.quality]
"ƥèřïôδ" = 24.7
[owner]
a_key = "a value"
b_key = 2010-02-29T03:03:03.3333Z
[foo.quality]
KEYONE = "VALUEONE"
"#);
  assert!(check_errors(&result).0, "There should have been an invalid datetime error, but there wasn't.");
  let errors = match result {
    ParseResult::FullError(e) => e,
    ParseResult::PartialError(_,_,_,e) => e,
    _ => panic!("There should have been an invalid datetime error, but there wasn't."),
  };
  let error = &errors.borrow()[0];
  if let &ParseError::InvalidDateTime(ref key, line, _col, ref val) = error {
    assert!(key == "owner.b_key" && line == 5 && val == "2010-02-29T03:03:03.3333Z",
      "key should be \"owner.b_key\", but is: \"{}\", line number should be 5, but is: {}, parsed datetime should be 2010-02-29T03:03:03.3333Z, but is {}",
        key, line, val);
  } else {
    assert!(false, "The first error should have been an invalid datetime error, but it wasn't.");
  }
}