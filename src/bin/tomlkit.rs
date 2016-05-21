extern crate pirate;
extern crate tomllib;
extern crate csv;
extern crate env_logger;
use std::fs::File;
use std::env;
use std::io;
use std::io::{Read, Error, Write};
use pirate::{Matches, Match, Vars, matches, usage, vars};
use tomllib::TOMLParser;
use tomllib::types::{ParseResult, Children, Value, TOMLError, TimeOffset, DateTime, Date, Time, TimeOffsetAmount,
                     PosNeg};
use csv::Reader;

macro_rules! usage(
  ($tval:expr) => (
    usage($tval);
    return;
  );
  ($submac:ident!( $($args:tt)* ), $tval:expr) => (
    $submac!($($args)*);
    usage($tval);
    std::process::exit(-1);
  );
);
// Bugs: Automatic Options group with -h/--help
//       Positional arguments look like flags/options in the description area
//       Required argument overrides --help
// Improvements: Remove bar when there's only a short or long name
//               Add ability to specify a name instead of using long/short name or description
//
fn main() {
  let options = vec![
    "#File-preserving commands",
    "g/get-value#Given a key or comma separated list of keys, retrieve the values for those keys. If multiple keys are \
      specified and any one of them fails to retrieve a value, the whole command will fail with an error message.:",
    "/has-value#Given a key or comma separated list of keys, print \"true\" if the key has a value and \"false\" if \
      the key doesn't have child keys. Optionally use the --set-true and --set-false flags to change what values are \
      printed instead of \"true\" and \"false\".:",
    "c/get-children#Given a key or comma separated list of keys, retrieve each key's set of child keys. If multiple \
      keys are specified and any one of them fails to retrieve a child keys, the whole command will fail with an error \
      message.:",
    "/has-children#Given a key or comma separated list of keys, print \"true\" if the key has child keys and \"false\" \
      if the key doesn't have child keys. Optionally use the --set-true and --set-false flags to change what values \
      are printed instead of \"true\" and \"false\".:",
    "#File-modifying commands",
    "s/set-value#Given a comma separated list of key followed by value followed by type, set the key's value to the \
      specified value. For instance: \"foo.bar,hello,basic-string,baz.qux,82374,int\" will set the key \"foo.bar\" to \
      a basic string value of \"hello\" and the key \"baz.qux\" to an integer value of 82374. If multiple keys are \
      specified and any one of them fails to set a value, the whole command will fail with an error message.:",
    "#Pre-command Options",
    "h/help#Show this screen.",
    "/set-true#For commands that print \"true\" or \"false\", this will change what value is printed for \"true\", \
      i.e. --set-true=1 will cause tomlkit to print \"1\" for \"true\" instead of \"true\".:",
    "/set-false#For commands that print \"true\" or \"false\", this will change what value is printed for \"false\", \
      i.e. --set-false=0 will cause tomlkit to print \"0\" for \"false\" instead of \"false\".:",
    "p/separator#Set the string that will separate multiple results. The default is \" ,\".:",
    "q/quiet#For commands that modify rather than return a result, turn off printing \"Success\" for each successful \
    modification.",
    "#Post-command Options",
    "/print-doc#Print out the resultant TOML document after all requested changes have been made.",
    "#Required arguments",
    "i/input-file#The path to the TOML document to parse and manipulate. If this isn't used then tomlkit will expect \
    the names of input files to come through stdin.:",
    "o/output-file#The path to write the finished TOML document to. If not specified any changes will be \
    written back to the INPUT_FILE.:",
  ];
  let _ = env_logger::init();

  let mut vars: Vars = match vars("tomlkit", &options) {
    Ok(v) => v,
    Err(e) => panic!("There was an error parsing argument definitions: {}", e)
  };

  let args: Vec<String> = env::args().collect();
  let matches: Matches = match matches(&args, &mut vars) {
    Ok(m) => m,
    Err(e) => {
      usage!(println!("Error: {}", e), &vars);
    }
  };

  if matches.has_match("help") {
    usage!(&vars);
  }

  // The file we're operating on
  if matches.has_match("input-file") {
    if let Some(f) = matches.get("input-file") {
      process_document(f, &matches, &vars);
    } else {
      usage!(println!("Error: A required argument is missing for input-file."), &vars);
    }
  } else {
    // No input-file specified so read files from stdin
    let mut input = String::new();
    loop {
      match io::stdin().read_line(&mut input) {
        Ok(n) => {
          if n == 0 {
            break;
          }
          process_document(&input.trim().to_string(), &matches, &vars);
          input.clear();
        },
        Err(err) => {
          println!("Unable to read input file names from stdin: {}", err);
          std::process::exit(-1);
        },
      }
    }
  }
}

fn process_document(file_path: &String, matches: &Matches, vars: &Vars) {
  let mut true_val = &"true".to_string();
  let mut false_val = &"false".to_string();
  let mut separator = &", ".to_string();
  let mut quiet = false;
  let mut file = String::new();

  match get_file(file_path, &mut file)  {
    Ok(()) => (),
    Err(err) => {
      println!("Error \"{}\": Unable to open file: {}", file_path, err);
      std::process::exit(-1);
    }
  }
  // Parse the document
  let parser: TOMLParser = TOMLParser::new();
  let (mut parser, result) = parser.parse(&file);
  match result {
    ParseResult::Partial(_,_,_) => {
      println!("Error \"{}\": Document only partially parsed. Please correct any errors before trying again.",
        file_path);
      std::process::exit(-1);
    },
    ParseResult::PartialError(_,_,_,_) => {
      println!("Error \"{}\": Document only partially parsed with errors. Please correct any errors before trying \
        again.", file_path);
      std::process::exit(-1);
    },
    ParseResult::Failure(_,_) => {
      println!("Error \"{}\": Completely failed to parse document. Please correct any error before trying again.",
        file_path);
      std::process::exit(-1);
    },
    ParseResult::FullError(errors) => {
      println!("Error \"{}\": Parsed entire document, but with errors: {:?}.", file_path, errors);
      std::process::exit(-1);
    },
    _ => (), // If verbose output Full or FullError
  }

  // Pre-command options
  if matches.has_match("set-true") {
    if let Some(t) = matches.get("set-true") {
      true_val = t;
    } else {
      usage!(println!("Error \"{}\": A required argument is missing for set-true.", file_path), &vars);
    }
  }
  if matches.has_match("set-false") {
    if let Some(f) = matches.get("set-false") {
      false_val = f;
    } else {
      usage!(println!("Error \"{}\": A required argument is missing for set-false.", file_path), &vars);
    }
  }
  if matches.has_match("separator") {
    if let Some(s) = matches.get("separator") {
      separator = s;
    } else {
      usage!(println!("Error \"{}\": A required argument is missing for separator.", file_path), &vars);
    }
  }

  let mut command: bool = false;
  let mut result: Vec<Result<String, String>> = vec![Ok("".to_string())];
  let mut out_file = file_path;
  // Commands only one command allowed per invocation for this version
  if matches.has_match("get-value") {
    command = true;
    if let Some(k) = matches.get("get-value") {
      result.push(get_value(k, separator, &parser));
    } else {
      usage!(println!("Error \"{}\": A required argument is missing for g/get-value.", file_path), &vars);
    }
  }
  if let Ok(_) = result[result.len() - 1] {
    if matches.has_match("has-value") {
      command = true;
      if let Some(k) = matches.get("has-value") {
        result.push(has_value(k, separator, &parser, true_val, false_val));
      } else {
        usage!(println!("Error \"{}\": A required argument is missing for has-value.", file_path), &vars);
      }
    }
  }
  if let Ok(_) = result[result.len() - 1] {
    if matches.has_match("get-children") {
      command = true;
      if let Some(k) = matches.get("get-children") {
        result.push(get_children(k, separator, &parser));
      } else {
        usage!(println!("Error \"{}\": A required argument is missing for c/get-children.", file_path), &vars);
      }
    }
  }
  if let Ok(_) = result[result.len() - 1] {
    if matches.has_match("has-children") {
      command = true;
      if let Some(k) = matches.get("has-children") {
        result.push(has_children(k, separator, &parser, true_val, false_val));
      } else {
        usage!(println!("Error \"{}\": A required argument is missing for has-children.", file_path), &vars);
      }
    }
  }
  if let Ok(_) = result[result.len() - 1] {
    if matches.has_match("set-value") {
      command = true;
      if let Some(kv) = matches.get("set-value") {
        if  matches.has_match("quiet") {
          quiet = true;
        }
        result.push(set_value(kv, separator, quiet, &mut parser));
        if matches.has_match("output-file") {
          match matches.get("output-file") {
            Some(out) => out_file = out,
            None => {
              usage!(println!("Error \"{}\": A required argument is missing for output-file.", file_path), &vars);
            },
          }
        }
        if let Ok(_) = result[result.len() - 1] {
          // Write back out to the file
          match write_to_file(out_file, &parser) {
            Ok(()) => (),
            Err(err) => {
              println!("Error \"{}\": Unable to write to file: \"{}\". Reason: {}", file_path, out_file, err);
              std::process::exit(-1);
            },
          }
        }
      } else {
        usage!(println!("Error \"{}\": A required argument is missing for s/set-value.", file_path), &vars);
      }
    }
  }
  if !command {
    // No command specified print usage
    usage!(println!("Error \"{}\": No command was specified.", file_path), &vars);
  }

  // ************** Print output here! *******************
  let _ = result.remove(0);
  for i in 0..result.len() {
    match &result[i]  {
      &Ok(ref val) => {
        if !quiet {
          print!("{}", val);
        }
      },
      &Err(ref err) => {
        print!("Error \"{}\": {}", file_path, err);
      }
    }
    if i < result.len() - 1 {
      print!("{}", separator);
    }
    if i == result.len() - 1 {
      println!("");
    }
  }

  // Post-command options
  if matches.has_match("print-doc") {
    println!("\n||  ||  ||  ||  ||  ||  ||  ||  ||  DOCUMENT  ||  ||  ||  ||  ||  ||  ||  ||  ||");
    println!("\\/  \\/  \\/  \\/  \\/  \\/  \\/  \\/  \\/            \\/  \\/  \\/  \\/  \\/  \\/  \\/  \\/  \\/\n");
    print_doc(&parser);
  }
}

fn write_to_file(file_path: &String, doc: &TOMLParser) -> Result<(), Error> {
  let mut f = try!(File::create(file_path));
  try!(f.write_all(format!("{}",doc).as_bytes()));
  try!(f.sync_all());
  Ok(())
}

fn get_file(file_path: &String, out_file: &mut String) -> Result<(), Error> {
  let mut f = try!(File::open(file_path));
  try!(f.read_to_string(out_file));
  Ok(())
}

fn get_value(csv: &str, sep: &String, doc: &TOMLParser) -> Result<String, String> {
  let key_results = csv_to_vec(csv);
  if let Ok(keys) = key_results {
    if keys.len() == 0 {
      return Err(format!("No keys specified: \"{}\".", csv));
    }
    let mut result = String::new();
    for i in 0..keys.len() {
      let key: &str = &keys[i];
      if let Some(value) = doc.get_value(key) {
        result.push_str(&format!("{}", value));
        if i < keys.len() - 1 {
          result.push_str(sep);
        }
      } else {
        return Err(format!("Key \"{}\" not found.", key));
      }
    }
    return Ok(result);
  }
  Err(format!("Could not parse keys: \"{:?}\".", csv))
}

fn has_value(csv: &str, sep: &String, doc: &TOMLParser, true_val: &String, false_val: &String)
  -> Result<String, String> {
  let key_results = csv_to_vec(csv);
  if let Ok(keys) = key_results {
    if keys.len() == 0 {
      return Err(format!("No keys specified: \"{}\".", csv));
    }
    let mut result = String::new();
    for i in 0..keys.len() {
      let key: &str = &keys[i];
      if let Some(_) = doc.get_value(key) {
        result.push_str(true_val);
      } else {
        result.push_str(false_val);
      }
      if i < keys.len() - 1 {
        result.push_str(sep);
      }
    }
    return Ok(result);
  }
  Err(format!("Could not parse keys: \"{}\".", csv))
}

fn get_children(csv: &str, sep: &String, doc: &TOMLParser) -> Result<String, String> {
  let key_results = csv_to_vec(csv);
  if let Ok(keys) = key_results {
    if keys.len() == 0 {
      return Err(format!("No keys specified: \"{}\".", csv));
    }
    let mut result = String::new();
    for i in 0..keys.len() {
      let key: &str = &keys[i];
      if let Some(c) = doc.get_children(key) {
        match c {
          &Children::Keys(ref ckeys) => {
            let mut val = String::new();
            val.push('[');
            if ckeys.borrow().len() > 0 {
              for i in 0..ckeys.borrow().len() - 1 {
                val.push_str(&ckeys.borrow()[i]);
                val.push_str(", ");
              }
              val.push_str(&ckeys.borrow()[ckeys.borrow().len() - 1]);
            }
            val.push(']');
            result.push_str(&val);
          },
          &Children::Count(ref size) => {
            if size.get() == 0 {
              return return Err(format!("Key \"{}\" has no children.", key));
            }
            result.push_str(&format!("{}", size.get()))
          },
        }
        if i < keys.len() - 1 {
          result.push_str(sep);
        }
      } else {
        return Err(format!("Key \"{}\" not found.", key));
      }
    }
    return Ok(result);
  }
  Err(format!("Could not parse keys: \"{}\".", csv))
}

fn has_children(csv: &str, sep: &String, doc: &TOMLParser, true_val: &String, false_val: &String)
  -> Result<String, String> {
  let key_results = csv_to_vec(csv);
  if let Ok(keys) = key_results {
    if keys.len() == 0 {
      return Err(format!("No keys specified: \"{}\".", csv));
    }
    let mut result = String::new();
    for i in 0..keys.len() {
      let key: &str = &keys[i];
      if let Some(_) = doc.get_children(key) {
        result.push_str(true_val);
      } else {
        result.push_str(false_val);
      }
      if i < keys.len() - 1 {
        result.push_str(sep);
      }
    }
    return Ok(result);
  }
  Err(format!("Could not parse keys: \"{}\".", csv))
}

fn set_value<'a>(kvs: &str, sep: &String, quiet: bool, doc: &mut TOMLParser) -> Result<String, String> {
  let keyval_results = csv_to_vec(kvs);
  if let Ok(keyvals) = keyval_results {
    if keyvals.len() % 3 != 0 || keyvals.len() == 0 {
      return Err(format!("No keys or wrong number of keys specified (must be a multiple of 3): \"{}\".", kvs));
    }
    let mut result = String::new();
    for i in 0..keyvals.len() / 3 {
      let key: &str = &keyvals[i*3];
      let val: &str = &keyvals[i*3+1];
      let typ: &str = &keyvals[i*3+2];
      let val_result: Result<Value, TOMLError>;
      match typ {
        "basic-string" | "bs" => val_result = Value::basic_string(val),
        "ml-basic-string" | "mbs" => val_result = Value::ml_basic_string(val),
        "literal-string" | "ls" => val_result = Value::literal_string(val),
        "ml-literal-string" | "mls" => val_result = Value::ml_literal_string(val),
        "integer" | "int" => val_result = Value::int_from_str(val),
        "float" | "flt" => val_result = Value::float_from_str(val),
        "boolean" | "bool" => val_result = Value::bool_from_str(val),
        "datetime" | "dt" => {
          let str_val: &str = &val;
          let tmp_result = Value::datetime_parse(str_val);
          let mut new_dt: DateTime = DateTime{date: Date{year: "".into(), month: "".into(), day: "".into()}, time: None};
          let (year, month, day);
          let (mut hour, mut minute, mut second, mut fraction) = ("".into(), "".into(), "".into(), "".into());
          let (mut off_hour, mut off_minute, mut pos_neg) = ("".into(), "".into(), PosNeg::Pos);
          let (mut has_time, mut has_fraction, mut has_offset) = (false, false, false);
          if let Ok(dtval) = tmp_result {
            if let Value::DateTime(dt) = dtval {
              year = dt.date.year.to_string().into();
              month = dt.date.month.to_string().into();
              day = dt.date.day.to_string().into();
              if let Some(ref time) = dt.time {
                has_time = true;
                hour = time.hour.to_string().into();
                minute = time.minute.to_string().into();
                second = time.second.to_string().into();
                if let Some(ref frac) = time.fraction {
                  has_fraction = true;
                  fraction = frac.to_string().into();
                }
                if let Some(ref offset) = time.offset {
                  if let &TimeOffset::Time(ref amount) = offset {
                    has_offset = true;
                    pos_neg = amount.pos_neg;
                    off_hour = amount.hour.to_string().into();
                    off_minute = amount.minute.to_string().into();
                  }
                }
              }

              let newoffset = if has_offset {
                Some(TimeOffset::Time(TimeOffsetAmount{pos_neg: pos_neg, hour: off_hour, minute: off_minute}))
              } else {
                None
              };
              let newfraction = if has_fraction {
                Some(fraction)
              } else {
                None
              };
              let newtime = if has_time {
                Some(Time{hour: hour, minute: minute, second: second, fraction: newfraction, offset: newoffset})
              } else {
                None
              };
              new_dt = DateTime{
                date: Date{
                  year: year,
                  month: month,
                  day: day,
                },
                time: newtime,
              };
            }
            val_result = Ok(Value::DateTime(new_dt));
          } else {
            return Err(format!("Unable to parse value: \"{}\" as type: \"{}\" for key: \"{}\"", val, typ, key));
          }
        },
        _ => return Err(format!("Type \"{}\" not recognized for key: \"{}\"", typ, key)),
      }
      if let Ok(value) = val_result {
        if doc.set_value(key, value) {
          if !quiet {
            result.push_str("Success");
          }
        } else {
          return Err(format!("Could not set value of key: \"{}\" to value: \"{}\", with type \"{}\"", key, val, typ));
        }
      } else {
        return Err(format!("Unable to parse value: \"{}\" as type: \"{}\" for key: \"{}\"", val, typ, key));
      }
      if !quiet {
        if i * 3  < keyvals.len() - 3 {
          result.push_str(sep);
        }
      }
    }
    return Ok(result);
  }
  Err(format!("Could not parse keys: \"{}\".", kvs))
}

fn csv_to_vec<'a>(csv: &str) -> Result<Vec<String>, csv::Error> {
  let mut fields = vec![];
  let mut rdr = Reader::from_string(csv).has_headers(false).escape(Some(b'\\')).quote(b'\0');
  while !rdr.done() {
    while let Some(result) = rdr.next_str().into_iter_result() {
      match result {
        Ok(field) => fields.push(field.to_string()),
        Err(err)  => return Err(err),
      }
    }
  }
  Ok(fields)
}

fn print_doc(doc: &TOMLParser) {
  //unimplemented!();
  println!("{}", doc);
}
