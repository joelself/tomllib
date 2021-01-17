macro_rules! to_val(
  ($tval:expr) => (
    match $tval {
      &TOMLValue::Integer(ref v) => Value::Integer(v.clone()),
      &TOMLValue::Float(ref v) => Value::Float(v.clone()),
      &TOMLValue::Boolean(v) => Value::Boolean(v),
      &TOMLValue::DateTime(ref v) => Value::DateTime(v.clone()),
      &TOMLValue::Array(ref arr) => Parser::sanitize_array(arr.clone()),
      &TOMLValue::String(ref s, t) => Value::String(s.clone(), t.clone()),
      &TOMLValue::InlineTable(ref it) => Parser::sanitize_inline_table(it.clone()),
      &TOMLValue::Table => panic!("Cannot convert a Table to a Value"),
    }
  );
);

#[allow(unused_macros)]
macro_rules! call_s(
  ($i:expr, $method:path) => ( $method( $i ) );
);

#[allow(unused_macros)]
macro_rules! res2opt(
  ($i:expr) => (
    match $i {
      Result::Ok(t) => Some(t),
      Result::Err(_) => None,
    }
  );
);