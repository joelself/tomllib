use crate::internals::ast::structs::{TableType, WSKeySep, Table, CommentNewLines, CommentOrNewLines, ArrayValue, Array,
                              TOMLValue, InlineTable, WSSep, TableKeyVal, ArrayType, HashValue, format_tt_keys};
use crate::internals::parser::Parser;
use crate::internals::primitives::Key;
use crate::types::{ParseError, Children};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::rc::Rc;
use std::cell::Cell;
use nom::IResult;

#[inline(always)]
fn map_val_to_array_type(val: &TOMLValue) -> ArrayType {
  match *val {
    TOMLValue::Integer(_)        => ArrayType::Integer,
    TOMLValue::Float(_)          => ArrayType::Float,
    TOMLValue::Boolean(_)        => ArrayType::Boolean,
    TOMLValue::DateTime(_)       => ArrayType::DateTime,
    TOMLValue::Array(_)          => ArrayType::Array,
    TOMLValue::String(_,_)       => ArrayType::String,
    TOMLValue::InlineTable(_)    => ArrayType::InlineTable,
    TOMLValue::Table             => panic!("Cannot have a table in an array"),
  }
}

impl<'a> Parser<'a> {

  pub fn insert(vector: &RefCell<Vec<String>>, insert: String) -> bool {
    for s in vector.borrow().iter() {
      if s == &insert {
        return false;
      }
    }
    vector.borrow_mut().push(insert);
    true
  }

  fn contains(vector: &RefCell<Vec<String>>, find: &str) -> bool {
    for s in vector.borrow().iter() {
      if s == find {
        return true;
      }
    }
    false
  }

  fn is_top_std_table(tables: &RefCell<Vec<Rc<TableType<'a>>>>) -> bool {
    if tables.borrow().len() ==  0 {
      false
    } else {
      let len = tables.borrow().len();
      matches!(*tables.borrow()[len - 1], TableType::Standard(_))
    }
  }

  fn equal_key_length(table: Rc<TableType<'a>>, tables: &RefCell<Vec<Rc<TableType<'a>>>>) -> bool {
    if tables.borrow().len() ==  0 {
      false
    } else {
      match *table {
        TableType::Array(ref t1) | TableType::Standard(ref t1) => {
          let len = tables.borrow().len();
          match *tables.borrow()[len - 1] {
            TableType::Array(ref t2) | TableType::Standard(ref t2) => {
              t1.keys.len() == t2.keys.len()
            }
          }
        }
      }
    }
  }

  fn add_implicit_tables(map: &RefCell<&mut HashMap<String, HashValue<'a>>>,
    tables: &RefCell<Vec<Rc<TableType<'a>>>>,
    tables_index: &RefCell<Vec<usize>>, table: Rc<TableType<'a>>) {
    let (valid, mut last_key, _) = Parser::get_array_table_key(map, tables, tables_index);
    if !valid {
      return;
    }
    let mut len = tables.borrow().len();
    let mut pop = false;
    if len == 0 {
      tables.borrow_mut().push(Rc::new(TableType::Standard(
        Table::new_str(WSSep::new_str("", ""), "$Root$", vec![])
      )));
      pop = true;
      len = 1;
      last_key.push_str("$Root$");
    }
    match *tables.borrow()[len - 1] {
      TableType::Array(ref last_at) | TableType::Standard(ref last_at) => {
        match *table {
          TableType::Array(ref tb) | TableType::Standard(ref tb) => {
            let mut first = true;
            debug!("last_at.keys.len(): {}, tb.keys.len(): {}", last_at.keys.len(), tb.keys.len());
            for i in 0..last_at.keys.len() {
              debug!("key {}: {}", i, last_at.keys[i].key);
            }
            let mut start = last_at.keys.len();
            if last_key == "$Root$" {
              start -= 1;
            }
            for i in start..tb.keys.len() {
              let mut borrow = map.borrow_mut();
              let mut insert = false;
              debug!("index: {}, last_key: {}", i, last_key);
              if let Entry::Occupied(mut o) = borrow.entry(last_key.clone()) {
                if first {

                  insert = match o.get_mut().subkeys {
                    Children::Keys(ref vec_rf) => {debug!("Inserting subkey: {}", tb.keys[i].key); Parser::insert(vec_rf, tb.keys[i].key.clone().into_owned())},
                    Children::Count(ref cell) => { debug!("Incrementing subkey count: {}", cell.get() + 1); cell.set(cell.get() + 1); true },
                  };
                  first = false;
                } else {
                  debug!("Inserting subkey: {}", tb.keys[i].key);
                  insert = match o.get_mut().subkeys {
                    Children::Keys(ref vec_rf) => Parser::insert(vec_rf, tb.keys[i].key.clone().into_owned()),
                    _ => panic!("Implicit tables can only be Standard Tables: \"{}\"", format!("{}.{}", last_key, tb.keys[i].key)),
                  };
                }
              }
              if i < tb.keys.len() - 1 {
                if last_key == "$Root$" {
                  last_key.truncate(0);
                } else {
                  last_key.push('.');
                }
                last_key.push_str(&tb.keys[i].key);
                if insert {
                  debug!("insert last_key {}", last_key);
                  if i == tb.keys.len() - 1 {
                    if let TableType::Array(_) = *table {
                      borrow.insert(last_key.clone(), HashValue::one_count());
                    } else {
                      borrow.insert(last_key.clone(), HashValue::none_keys());
                    }
                  } else {
                    borrow.insert(last_key.clone(), HashValue::none_keys());
                  }
                }
              }
            }
          },
        }
      }
    }
    if pop {
      tables.borrow_mut().pop();
    }
    debug!("Returning from add_implicit_tables");
  }

  #[allow(clippy::match_ref_pats)]
  fn increment_array_table_index(map: &RefCell<&mut HashMap<String, HashValue<'a>>>,
    tables: &RefCell<Vec<Rc<TableType<'a>>>>, tables_index: &RefCell<Vec<usize>>,) {
    let parent_key = Parser::get_key_parent(tables, tables_index);
    debug!("increment_array_table_index: {}", parent_key);
    let mut borrow = map.borrow_mut();
    let entry = borrow.entry(parent_key);
    if let Entry::Occupied(mut o) = entry {
      if let Children::Count(ref c) = o.get_mut().subkeys {
         c.set(c.get() + 1);
      }
    }
    let len = tables_index.borrow().len();
    let last_index = tables_index.borrow()[len - 1];
    tables_index.borrow_mut()[len - 1] = last_index + 1;
  }

  fn add_to_table_set(map: &RefCell<&mut HashMap<String, HashValue<'a>>>,
    tables: &RefCell<Vec<Rc<TableType<'a>>>>, tables_index: &RefCell<Vec<usize>>, key: &str) -> bool{
    let parent_key = Parser::get_key_parent(tables, tables_index);
    debug!("add_to_table_set: {}", parent_key);
    let mut borrow = map.borrow_mut();
    let entry = borrow.entry(parent_key);
    if let Entry::Occupied(mut o) = entry {
      if let Children::Keys(ref keys) = o.get_mut().subkeys {
        let contains = Parser::contains(keys, key);
        if contains {
          debug!("key already exists");
          return false;
        }
        debug!("add_to_table_set--> {}", key);
        Parser::insert(keys, key.to_string());
      }
    }
    true
  }

  // Table
  method!(pub table<Parser<'a>, &'a str, Rc<TableType> >, mut self,
    alt!(
      complete!(call_m!(self.array_table)) |
      complete!(call_m!(self.std_table))
    )
  );

  method!(table_subkeys<Parser<'a>, &'a str, Vec<WSKeySep> >, mut self, many0!(call_m!(self.table_subkey)));

  method!(table_subkey<Parser<'a>, &'a str, WSKeySep>, mut self,
    chain!(
      ws1: call_m!(self.ws)         ~
           tag_s!(".")~
      ws2: call_m!(self.ws)         ~
      key: call_m!(self.key)        ,
      ||{
        WSKeySep::new_str(WSSep::new_str(ws1, ws2), key)
      }
    )
  );
  // Standard Table
  method!(std_table<Parser<'a>, &'a str, Rc<TableType> >, mut self,
    chain!(
           tag_s!("[")    ~
      ws1: call_m!(self.ws)             ~
      key: call_m!(self.key)            ~
  subkeys: call_m!(self.table_subkeys)  ~
      ws2: call_m!(self.ws)             ~
           tag_s!("]")    ,
      ||{
        let keys_len = subkeys.len() + 1;
        let res = Rc::new(TableType::Standard(Table::new_str(
          WSSep::new_str(ws1, ws2), key, subkeys
        )));
        let mut error = false;
        let keychain_len = self.keychain.borrow().len();
        self.keychain.borrow_mut().truncate(keychain_len - keys_len);
        if Parser::is_top_std_table(&self.last_array_tables) || Parser::equal_key_length(res.clone(), &self.last_array_tables) {
          self.last_array_tables.borrow_mut().pop();
          self.last_array_tables_index.borrow_mut().pop();
        }
        let map = RefCell::new(&mut self.map);
        let mut table_key = "".to_string();
        let mut parent_key = "$Root$".to_string();
        debug!("Before get len");
        let mut len = self.last_array_tables.borrow().len();
        let current_key = format_tt_keys(&*res);
        if len > 0 {
          let last_key = format_tt_keys(&self.last_array_tables.borrow()[len - 1]);
          if current_key == last_key {
            error = true;
          } else {
            debug!("Check if subtable");
            let subtable = res.is_subtable_of(&self.last_array_tables.borrow()[len - 1]);
            if !subtable {
              loop {
                debug!("Not subtable pop {}", self.last_array_tables.borrow()[self.last_array_tables.borrow().len() - 1]);
                self.last_array_tables.borrow_mut().pop();
                self.last_array_tables_index.borrow_mut().pop();
                len -= 1;
                debug!("check array_tables len and subtable");
                if len == 0 || res.is_subtable_of(&self.last_array_tables.borrow()[len - 1]) {
                  break;
                }
              }
            }
          }
        }
        if len > 0 && current_key == format_tt_keys(&self.last_array_tables.borrow()[len - 1]) {
          error = true;
        } else {
          self.last_array_tables.borrow_mut().push(res.clone());
          self.last_array_tables_index.borrow_mut().push(0);
          let tuple = Parser::get_array_table_key(&map, &self.last_array_tables, &self.last_array_tables_index);
          table_key = tuple.1;
          parent_key = tuple.2;
          self.last_array_tables.borrow_mut().pop();
          self.last_array_tables_index.borrow_mut().pop();
          if !tuple.0 || Parser::key_has_children_with_values(&parent_key, &map) {
            error = true;
          } else {
            debug!("Standard Table Key: {}", table_key);
            if map.borrow().contains_key(&table_key) {
              let map_borrow = map.borrow();
              let hash_val_opt = map_borrow.get(&table_key);
              if let Some(ref hash_val) = hash_val_opt {
                if let Some(ref rc_rc_val) = hash_val.value {
                  if let TOMLValue::Table = *rc_rc_val.borrow() {
                    debug!("Table error {}, table already defined", table_key);
                    error = true;
                  }
                }
              }
            }
          }
        }
        debug!("Before error check");
        if error {
          self.last_array_tables.borrow_mut().push(res.clone());
          self.last_array_tables_index.borrow_mut().push(0);
          let array_table_key = Parser::get_array_table_key(&map, &self.last_array_tables,
            &self.last_array_tables_index).1;
          debug!("Setting Invalid Table {} in Standard Table", array_table_key);
          self.errors.borrow_mut().push(ParseError::InvalidTable(
            array_table_key, self.line_count.get(), 0,
            RefCell::new(HashMap::new())
          ));
          self.last_array_tables.borrow_mut().pop();
          self.last_array_tables_index.borrow_mut().pop();
          self.array_error.set(true);
        } else {
          Parser::add_implicit_tables(&map, &self.last_array_tables,
            &self.last_array_tables_index, res.clone());
          if let TableType::Standard(ref tbl) = *res {
            Parser::add_to_table_set(&map, &self.last_array_tables,
              &self.last_array_tables_index, &tbl.keys[keys_len - 1].key);
            self.array_error.set(false);
            debug!("insert table_key: {}", table_key);
            let contains_key = map.borrow().contains_key(&table_key);
            if contains_key {
              debug!("contains table key {}", table_key);
              if let Entry::Occupied(mut o) = map.borrow_mut().entry(table_key.clone()) {
                o.get_mut().value = Some(Rc::new(RefCell::new(TOMLValue::Table)));
                debug!("table key {}'s children: {:?}", table_key, o.get().subkeys)
              }
            } else {
              debug!("insert new table key {}", table_key);
              map.borrow_mut().insert(table_key.clone(), HashValue::table_keys());
            }
            if let Entry::Occupied(mut o) = map.borrow_mut().entry(parent_key) {
              match o.get_mut().subkeys {
                Children::Keys(ref vec_rf) => {Parser::insert(vec_rf, tbl.keys[keys_len - 1].key.clone().into_owned());},
                _ => panic!("Trying to add a key to an array: \"{}\"", table_key),
              }
            }
            self.last_array_tables.borrow_mut().push(res.clone());
            self.last_array_tables_index.borrow_mut().push(0);
            self.last_table = Some(res.clone());
          }
        }
        res
      }
    )
  );

  // Array
  method!(array_sep<Parser<'a>, &'a str, WSSep>, mut self,
    chain!(
      ws1: call_m!(self.ws)         ~
           tag_s!(",")~
      ws2: call_m!(self.ws)         ,
      ||{
        WSSep::new_str(ws1, ws2)
      }
    )
  );

  // Array Table
  method!(array_table<Parser<'a>, &'a str, Rc<TableType> >, mut self,
    chain!(
           tag_s!("[[")   ~
      ws1: call_m!(self.ws)             ~
      key: call_m!(self.key)            ~
  subkeys: call_m!(self.table_subkeys)  ~
      ws2: call_m!(self.ws)             ~
           tag_s!("]]")   ,
      ||{
        let keys_len = subkeys.len() + 1;
        let res = Rc::new(TableType::Array(Table::new_str(
          WSSep::new_str(ws1, ws2), key, subkeys
        )));
        let keychain_len = self.keychain.borrow().len();
        self.keychain.borrow_mut().truncate(keychain_len - keys_len);
        if Parser::is_top_std_table(&self.last_array_tables) {
          self.last_array_tables.borrow_mut().pop();
          self.last_array_tables_index.borrow_mut().pop();
        }
        {
          let map = RefCell::new(&mut self.map);
          self.array_error.set(false);
          let len = self.last_array_tables.borrow().len();
          let current_key = format_tt_keys(&*res);
          if len > 0 {
            let mut len = self.last_array_tables.borrow().len();
            let last_key = format_tt_keys(&self.last_array_tables.borrow()[len - 1]);
            debug!("current_key: {}, last_key: {}", current_key, last_key);
            if current_key == last_key {
              debug!("Increment array table index");
              Parser::increment_array_table_index(&map, &self.last_array_tables,
                &self.last_array_tables_index);
            } else {
              let subtable = res.is_subtable_of(&self.last_array_tables.borrow()[len - 1]);
              if subtable {
                debug!("Is subtable");
                Parser::add_implicit_tables(&map, &self.last_array_tables,
                  &self.last_array_tables_index, res.clone());
                self.last_array_tables.borrow_mut().push(res.clone());
                self.last_array_tables_index.borrow_mut().push(0);
              } else {
                debug!("NOT subtable");
                while self.last_array_tables.borrow().len() > 0 &&
                  current_key != format_tt_keys(&self.last_array_tables.borrow()[self.last_array_tables.borrow().len() - 1]) {
                  debug!("pop table");
                  self.last_array_tables.borrow_mut().pop();
                  self.last_array_tables_index.borrow_mut().pop();
                }
                len = self.last_array_tables.borrow().len();
                if len > 0 {
                  debug!("Increment array table index the second");
                  Parser::increment_array_table_index(&map, &self.last_array_tables,
                    &self.last_array_tables_index);
                } else {
                  debug!("Add implicit tables");
                  Parser::add_implicit_tables(&map, &self.last_array_tables,
                    &self.last_array_tables_index,  res.clone());
                  self.last_array_tables.borrow_mut().push(res.clone());
                  self.last_array_tables_index.borrow_mut().push(0);
                  let parent_key = Parser::get_key_parent(&self.last_array_tables,
                    &self.last_array_tables_index);
                  debug!("Got parent key: {}", parent_key);
                  let contains_key = map.borrow().contains_key(&parent_key);
                  if contains_key {
                    debug!("Increment array table index the third");
                    Parser::increment_array_table_index(&map, &self.last_array_tables,
                      &self.last_array_tables_index);
                  }
                }
              }
            }
          } else {
            debug!("Len == 0 add implicit tables");
            Parser::add_implicit_tables(&map, &self.last_array_tables,
              &self.last_array_tables_index, res.clone());
            self.last_array_tables.borrow_mut().push(res.clone());
            self.last_array_tables_index.borrow_mut().push(0);
          }
          debug!("Before call to get_array_table_key");
          let (valid, full_key, parent_key) = Parser::get_array_table_key(&map, &self.last_array_tables,
            &self.last_array_tables_index);
          if valid {
            debug!("After call to get_array_table_key");
            let contains_key = map.borrow().contains_key(&parent_key);
            if contains_key {
              debug!("Increment existing array of table key: {}", full_key);
            } else {
              debug!("Insert new array of table key: {}", full_key);
              map.borrow_mut().insert(parent_key, HashValue::one_count());
            }
            map.borrow_mut().insert(full_key, HashValue::none_keys());
            self.last_table = Some(res.clone());
          } else {
            debug!("Setting Invalid Table {}", full_key);
            self.errors.borrow_mut().push(ParseError::InvalidTable(
              full_key, self.line_count.get(), 0,
              RefCell::new(HashMap::new())
            ));
          }
        }
        res
      }
    )
  );

  method!(ws_newline<Parser<'a>, &'a str, &'a str>, self, re_find!("^( |\t|\n|(\r\n))*"));

  method!(comment_nl<Parser<'a>, &'a str, CommentNewLines>, mut self,
    chain!(
   prewsnl: call_m!(self.ws_newline)  ~
   comment: call_m!(self.comment)     ~
  newlines: call_m!(self.ws_newline) ,
      ||{
        CommentNewLines::new_str(prewsnl, comment, newlines)
      }
    )
  );

  method!(comment_or_nl<Parser<'a>, &'a str, CommentOrNewLines>, mut self,
    alt!(
      complete!(call_m!(self.comment_nl))   => {|com| CommentOrNewLines::Comment(com)} |
      complete!(call_m!(self.ws_newline))  => {|nl: &'a str|  CommentOrNewLines::NewLines(nl.into())}
    )
  );

  method!(comment_or_nls<Parser<'a>, &'a str, Vec<CommentOrNewLines> >, mut self,
    many1!(call_m!(self.comment_or_nl)));

  method!(array_value<Parser<'a>, &'a str, ArrayValue>, mut self,
        chain!(
          val: call_m!(self.val)                        ~
    array_sep: complete!(call_m!(self.array_sep))?      ~
  comment_nls: complete!(call_m!(self.comment_or_nls))  ,
          ||{
            let t = map_val_to_array_type(&*val.borrow());
            let len = self.last_array_type.borrow().len();
            if len > 0 && self.last_array_type.borrow()[len - 1] != ArrayType::None &&
               self.last_array_type.borrow()[len - 1] != t {
              let tuple = Parser::get_full_key(&RefCell::new(& mut self.map), &self.last_array_tables,
                &self.last_array_tables_index, &self.keychain
              );
              let err_len = self.errors.borrow().len();
              let mut mixed = false;
              if err_len > 0 {
                if let ParseError::MixedArray(ref key, _, _) = self.errors.borrow()[err_len - 1] {
                  debug!("Check mixed array previous: {}, current: {}", key, tuple.1);
                  if !tuple.1.starts_with(key) {
                    mixed = true;
                    debug!("Mixed array error insert: {}", tuple.1);
                  }
                }
              } else {
                mixed = true;
                debug!("Mixed array error insert: {}", tuple.1);
              }
              if mixed {
                self.errors.borrow_mut().push(ParseError::MixedArray(
                  tuple.2, self.line_count.get(), 0
                ));
              }
            }
            self.last_array_type.borrow_mut().pop();
            self.last_array_type.borrow_mut().push(t);
            let keychain_len = self.keychain.borrow().len();
            self.insert_keyval_into_map(val.clone());
            self.keychain.borrow_mut()[keychain_len - 1].inc();
            ArrayValue::new(val, array_sep, comment_nls)
          }
        )
  );

  method!(array_values<Parser<'a>, &'a str, Vec<ArrayValue> >, mut self,
    chain!(
     vals: many0!(call_m!(self.array_value)) ,
     ||{
        debug!("Finished array values");
        let mut tmp = vec![];
        tmp.extend(vals);
        tmp
      }
    )
  );

  pub fn array(mut self: Parser<'a>, input: &'a str) -> (Parser<'a>, IResult<&'a str, Rc<RefCell<Array>>>) {
    // Initialize last array type to None, we need a stack because arrays can be nested
    // debug!("*** array called on input:\t\t\t{}", input);
    self.last_array_type.borrow_mut().push(ArrayType::None);
    self.keychain.borrow_mut().push(Key::Index(Cell::new(0)));
    let (tmp, res) = self.array_internal(input);
    self = tmp; // Restore self
    self.keychain.borrow_mut().pop();
    self.last_array_type.borrow_mut().pop();
    (self, res)
  }

  method!(pub array_internal<Parser<'a>, &'a str, Rc<RefCell<Array>> >, mut self,
    chain!(
              tag_s!("[")                   ~
         cn1: call_m!(self.comment_or_nls)  ~
  array_vals: call_m!(self.array_values)    ~
         cn2: call_m!(self.comment_or_nls)  ~
              tag_s!("]")                   ,
      ||{
        debug!("Close array");
       let array_result = Rc::new(RefCell::new(Array::new(array_vals, cn1, cn2)));
        array_result
      }
    )
  );

  method!(table_keyval<Parser<'a>, &'a str, TableKeyVal>, mut self,
        chain!(
       keyval: call_m!(self.keyval)                     ~
   keyval_sep: complete!(call_m!(self.array_sep))?      ~
  comment_nls: complete!(call_m!(self.comment_or_nls))  ,
          ||{
            TableKeyVal::new(keyval, keyval_sep, comment_nls)
          }
        )
  );

  method!(inline_table_keyvals_non_empty<Parser<'a>, &'a str, Vec<TableKeyVal> >, mut self, many0!(call_m!(self.table_keyval)));

  method!(pub inline_table<Parser<'a>, &'a str, Rc<RefCell<InlineTable>> >, mut self,
    chain!(
           tag_s!("{")                                ~
      ws1: call_m!(self.ws)                                         ~
  keyvals: complete!(call_m!(self.inline_table_keyvals_non_empty))? ~
      ws2: call_m!(self.ws)                                         ~
           tag_s!("}")                                ,
          ||{
            #[allow(clippy::redundant_pattern_matching)]
            if let Some(_) = keyvals {
              Rc::new(RefCell::new(InlineTable::new(keyvals.unwrap(), WSSep::new_str(ws1, ws2))))
            } else {
              Rc::new(RefCell::new(InlineTable::new(vec![], WSSep::new_str(ws1, ws2))))
            }
          }
    )
  );
}

#[cfg(test)]
mod test {
  use std::rc::Rc;
  use std::cell::{RefCell, Cell};
  use nom::IResult::Done;
  use crate::types::{DateTime, Date, Time, TimeOffset, TimeOffsetAmount, StrType};
  use crate::internals::parser::Parser;
  use crate::internals::ast::structs::{Array, ArrayValue, WSSep, TableKeyVal, InlineTable, WSKeySep, KeyVal, CommentNewLines,
                                Comment, CommentOrNewLines, Table, TableType, TOMLValue};
  use crate::internals::primitives::Key;

  #[test]
  fn test_table() {
    let mut p = Parser::new();
    assert_eq!(p.table("[ _underscore_ . \"-δáƨλèƨ-\" ]").1, Done("",
      Rc::new(TableType::Standard(Table::new_str(
        WSSep::new_str(" ", " "), "_underscore_", vec![
          WSKeySep::new_str(WSSep::new_str(" ", " "), "\"-δáƨλèƨ-\"")
        ]
      ))
    )));
    p = Parser::new();
    assert_eq!(p.table("[[\t NumberOne\t.\tnUMBERtWO \t]]").1, Done("",
      Rc::new(TableType::Array(Table::new_str(
        WSSep::new_str("\t ", " \t"), "NumberOne", vec![
          WSKeySep::new_str(WSSep::new_str("\t", "\t"), "nUMBERtWO")
        ]
      ))
    )));
  }

  #[test]
  fn test_table_subkey() {
    let p = Parser::new();
    assert_eq!(p.table_subkey("\t . \t\"áƭúƨôèλôñèƭúññèôúñôèƭú\"").1, Done("",
      WSKeySep::new_str(WSSep::new_str("\t ", " \t"), "\"áƭúƨôèλôñèƭúññèôúñôèƭú\""),
    ));
  }

  #[test]
  fn test_table_subkeys() {
    let p = Parser::new();
    assert_eq!(p.table_subkeys(" .\tAPPLE.MAC . \"ßÓÓK\"").1, Done("",
      vec![
        WSKeySep::new_str(WSSep::new_str(" ", "\t"), "APPLE"),
        WSKeySep::new_str(WSSep::new_str("", ""), "MAC"),
        WSKeySep::new_str(WSSep::new_str(" ", " "), "\"ßÓÓK\"")
      ]
    ));
  }

  #[test]
  fn test_std_table() {
    let p = Parser::new();
    assert_eq!(p.std_table("[Dr-Pepper  . \"ƙè¥_TWÓ\"]").1, Done("",
      Rc::new(TableType::Standard(Table::new_str(
        WSSep::new_str("", ""), "Dr-Pepper", vec![
          WSKeySep::new_str(WSSep::new_str("  ", " "), "\"ƙè¥_TWÓ\"")
        ]
      )))
    ));
  }

  #[test]
  fn test_array_table() {
    let p = Parser::new();
    assert_eq!(p.array_table("[[\"ƙè¥ôñè\"\t. key_TWO]]").1, Done("",
      Rc::new(TableType::Array(Table::new_str(
        WSSep::new_str("", ""), "\"ƙè¥ôñè\"", vec![
          WSKeySep::new_str(WSSep::new_str("\t", " "), "key_TWO")
        ]
      ))
    )));
  }

  #[test]
  fn test_array_sep() {
    let p = Parser::new();
    assert_eq!(p.array_sep("  ,  ").1, Done("", WSSep::new_str("  ", "  ")));
  }

  #[test]
  fn test_ws_newline() {
    let p = Parser::new();
    assert_eq!(p.ws_newline("\t\n\n").1, Done("", "\t\n\n"));
  }

  #[test]
  fn test_comment_nl() {
    let p = Parser::new();
    assert_eq!(p.comment_nl("\r\n\t#çô₥₥èñƭñèωℓïñè\n \n \n").1, Done("",
      CommentNewLines::new_str(
        "\r\n\t", Comment::new_str("çô₥₥èñƭñèωℓïñè"), "\n \n \n"
      )
    ));
  }

  #[test]
  fn test_comment_or_nl() {
    let mut p = Parser::new();
    assert_eq!(p.comment_or_nl("#ωôřƙωôřƙ\n").1, Done("",
      CommentOrNewLines::Comment(CommentNewLines::new_str(
        "", Comment::new_str("ωôřƙωôřƙ"), "\n"
      ))
    ));
    p = Parser::new();
    assert_eq!(p.comment_or_nl(" \t\n#ωôřƙωôřƙ\n \r\n").1, Done("",
      CommentOrNewLines::Comment(CommentNewLines::new_str(
        " \t\n", Comment::new_str("ωôřƙωôřƙ"), "\n \r\n"
      ))
    ));
    p = Parser::new();
    assert_eq!(p.comment_or_nl("\n\t\r\n ").1, Done("", CommentOrNewLines::NewLines("\n\t\r\n ".into())));
  }

  #[test]
  fn test_array_value() {
    let mut p = Parser::new();
    p.keychain.borrow_mut().push(Key::Index(Cell::new(0)));
    assert_eq!(p.array_value("54.6, \n#çô₥₥èñƭ\n\n").1,
      Done("",ArrayValue::new(
        Rc::new(RefCell::new(TOMLValue::Float("54.6".into()))), Some(WSSep::new_str("", " ")),
        vec![CommentOrNewLines::Comment(CommentNewLines::new_str(
          "\n", Comment::new_str("çô₥₥èñƭ"), "\n\n"
        ))]
      ))
    );
    p = Parser::new();
    p.keychain.borrow_mut().push(Key::Index(Cell::new(0)));
    assert_eq!(p.array_value("\"ƨƥáϱλèƭƭï\"").1,
      Done("",ArrayValue::new(
        Rc::new(RefCell::new(TOMLValue::String("ƨƥáϱλèƭƭï".into(), StrType::Basic))), None, vec![CommentOrNewLines::NewLines("".into())]
      ))
    );
    p = Parser::new();
    p.keychain.borrow_mut().push(Key::Index(Cell::new(0)));
    assert_eq!(p.array_value("44_9 , ").1,
      Done("",ArrayValue::new(
        Rc::new(RefCell::new(TOMLValue::Integer("44_9".into()))), Some(WSSep::new_str(" ", " ")),
        vec![CommentOrNewLines::NewLines("".into())]
      ))
    );
  }

  #[test]
  fn test_array_values() {
    let mut p = Parser::new();
    p.keychain.borrow_mut().push(Key::Index(Cell::new(0)));
    assert_eq!(p.array_values("1, 2, 3").1, Done("", vec![
      ArrayValue::new(Rc::new(RefCell::new(TOMLValue::Integer("1".into()))), Some(WSSep::new_str("", " ")),
      vec![CommentOrNewLines::NewLines("".into())]),
      ArrayValue::new(Rc::new(RefCell::new(TOMLValue::Integer("2".into()))), Some(WSSep::new_str("", " ")),
      vec![CommentOrNewLines::NewLines("".into())]),
      ArrayValue::new(Rc::new(RefCell::new(TOMLValue::Integer("3".into()))), None, vec![CommentOrNewLines::NewLines("".into())])
    ]));
    p = Parser::new();
    p.keychain.borrow_mut().push(Key::Index(Cell::new(0)));
    assert_eq!(p.array_values("1, 2, #çô₥₥èñƭ\n3, ").1, Done("", vec![
      ArrayValue::new(Rc::new(RefCell::new(TOMLValue::Integer("1".into()))), Some(WSSep::new_str("", " ")),
      vec![CommentOrNewLines::NewLines("".into())]),
      ArrayValue::new(Rc::new(RefCell::new(TOMLValue::Integer("2".into()))), Some(WSSep::new_str("", " ")),
        vec![CommentOrNewLines::Comment(CommentNewLines::new_str("", Comment::new_str("çô₥₥èñƭ"), "\n"))]),
      ArrayValue::new(Rc::new(RefCell::new(TOMLValue::Integer("3".into()))), Some(WSSep::new_str("", " ")),
      vec![CommentOrNewLines::NewLines("".into())])
    ]));
  }

  #[test]
  fn test_non_nested_array() {
    let p = Parser::new();
    assert_eq!(p.array("[2010-10-10T10:10:10.33Z, 1950-03-30T21:04:14.123+05:00]").1,
      Done("", Rc::new(RefCell::new(Array::new(
        vec![ArrayValue::new(
          Rc::new(RefCell::new(TOMLValue::DateTime(DateTime::new(
            Date::new_str("2010", "10", "10"), Some(Time::new_str("10", "10", "10", Some("33"),
              Some(TimeOffset::Zulu)
          )))))),
          Some(WSSep::new_str("", " ")),
          vec![CommentOrNewLines::NewLines("".into())]
        ),
        ArrayValue::new(
          Rc::new(RefCell::new(TOMLValue::DateTime(DateTime::new(
            Date::new_str("1950", "03", "30"), Some(Time::new_str("21", "04", "14", Some("123"),
            Some(TimeOffset::Time(TimeOffsetAmount::new_str("+", "05", "00")))
          )))))),
          None, vec![CommentOrNewLines::NewLines("".into())]
        )],
        vec![CommentOrNewLines::NewLines("".into())], vec![CommentOrNewLines::NewLines("".into())]
      ))))
    );
  }

  #[test]
  fn test_nested_array() {
    let p = Parser::new();
    assert_eq!(p.array("[[3,4], [4,5], [6]]").1,
      Done("", Rc::new(RefCell::new(Array::new(
        vec![
          ArrayValue::new(
            Rc::new(RefCell::new(TOMLValue::Array(Rc::new(RefCell::new(Array::new(
              vec![
                ArrayValue::new(
                  Rc::new(RefCell::new(TOMLValue::Integer("3".into()))), Some(WSSep::new_str("", "")),
                  vec![CommentOrNewLines::NewLines("".into())]
                ),
                ArrayValue::new(
                  Rc::new(RefCell::new(TOMLValue::Integer("4".into()))), None, vec![CommentOrNewLines::NewLines("".into())]
                )
              ],
              vec![CommentOrNewLines::NewLines("".into())], vec![CommentOrNewLines::NewLines("".into())]
            )))))),
            Some(WSSep::new_str("", " ")),
            vec![CommentOrNewLines::NewLines("".into())]
          ),
          ArrayValue::new(
            Rc::new(RefCell::new(TOMLValue::Array(Rc::new(RefCell::new(Array::new(
              vec![
                ArrayValue::new(
                  Rc::new(RefCell::new(TOMLValue::Integer("4".into()))), Some(WSSep::new_str("", "")),
                  vec![CommentOrNewLines::NewLines("".into())]
                ),
                ArrayValue::new(
                    Rc::new(RefCell::new(TOMLValue::Integer("5".into()))), None, vec![CommentOrNewLines::NewLines("".into())]
                )
              ],
              vec![CommentOrNewLines::NewLines("".into())], vec![CommentOrNewLines::NewLines("".into())]
            )))))),
            Some(WSSep::new_str("", " ")),
            vec![CommentOrNewLines::NewLines("".into())]
          ),
          ArrayValue::new(
            Rc::new(RefCell::new(TOMLValue::Array(Rc::new(RefCell::new(Array::new(
              vec![
                ArrayValue::new(
                  Rc::new(RefCell::new(TOMLValue::Integer("6".into()))), None, vec![CommentOrNewLines::NewLines("".into())]
                )
              ],
             vec![CommentOrNewLines::NewLines("".into())], vec![CommentOrNewLines::NewLines("".into())]
            )))))),
            None, vec![CommentOrNewLines::NewLines("".into())]
          )
        ],
        vec![CommentOrNewLines::NewLines("".into())], vec![CommentOrNewLines::NewLines("".into())]
      ))))
    );
  }

  #[test]
  fn test_table_keyval() {
    let p = Parser::new();
    assert_eq!(p.table_keyval("\"Ì WúƲ Húϱƨ!\"\t=\t'Mè ƭôô!' ").1, Done("", TableKeyVal::new(
      KeyVal::new_str(
        "\"Ì WúƲ Húϱƨ!\"", WSSep::new_str("\t", "\t"), Rc::new(RefCell::new(TOMLValue::String("Mè ƭôô!".into(), StrType::Literal)))
      ),
      None,
      vec![]
    )));
  }

  #[test]
  fn test_inline_table_keyvals_non_empty() {
    let p = Parser::new();
    assert_eq!(p.inline_table_keyvals_non_empty("Key =\t54,\"Key2\" = '34.99'").1,
      Done("", vec![
        TableKeyVal::new(
          KeyVal::new_str(
            "Key", WSSep::new_str(" ", "\t"),
            Rc::new(RefCell::new(TOMLValue::Integer("54".into())))
          ),
          Some(WSSep::new_str("", "")),
          vec![]
        ),
        TableKeyVal::new(
          KeyVal::new_str(
            "\"Key2\"", WSSep::new_str( " ", " "),
            Rc::new(RefCell::new(TOMLValue::String("34.99".into(), StrType::Literal)))
          ),
          None,
          vec![]
        )
      ])
    );
  }

  #[test]
  fn test_inline_table() {
    let p = Parser::new();
    assert_eq!(p.inline_table("{\tKey = 3.14E+5 , \"Key2\" = '''New\nLine'''\t}").1,
      Done("", Rc::new(RefCell::new(InlineTable::new(
        vec![
          TableKeyVal::new(
            KeyVal::new_str(
              "Key", WSSep::new_str(" ", " "),
              Rc::new(RefCell::new(TOMLValue::Float("3.14E+5".into())))
            ),
            Some(WSSep::new_str(" ", " ")),
            vec![CommentOrNewLines::NewLines("".into())]
          ),
          TableKeyVal::new(
            KeyVal::new_str("\"Key2\"", WSSep::new_str(" ", " "),
              Rc::new(RefCell::new(TOMLValue::String("New\nLine".into(), StrType::MLLiteral)))
            ),
            None,
            vec![CommentOrNewLines::NewLines("\t".into())]
          )
        ],
        WSSep::new_str("\t", "")
      ))))
    );
  }
}
