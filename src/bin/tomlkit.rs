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
    "p/separator#Set the string that will separate multiple results. The default is \", \".:",
    "array-separator#Set the string that will separate multiple results within an array. The default is \", \". NOTE! \
    This option is likely to go away in version 0.2.0 in favor of specifying a format string.:",
    "array-begin#Set the string that will begin a set of results in an array. The default is \"[\". **NOTE!** \
    This option is likely to go away in version 0.2.0 in favor of specifying a format string.:",
    "array-end#Set the string that will end a set of results in an array. The default is \"]\". **NOTE!** \
    This option is likely to go away in version 0.2.0 in favor of specifying a format string.:",
    "array-separator#Set the string that will separate multiple results within an array. The default is \", \". \
    **NOTE!** This option is likely to go away in version 0.2.0 in favor of specifying a format string.:",
    "array-length#Rather than beginning and ending an array with special characters, print the length of the array \
    as an unsigned 64 bit integer before printing the contents of the array, e.g. 3, \"Alice\", \"Bob\", \"Eve\".",
    "strip-quotes#All types of string values will be printed without single or double quotes. \
    **NOTE!** This option is likely to go away in version 0.2.0 in favor of specifying a format string.:",
    "/strip-quotes#Removes all single and double quotes from returned string values. It does not strip quotes from \
    keys as that would make them unusable. Warning: This will make it impossible to distinguish between each type of \
    string and it can make it impossible to tell the return type e.g. Is 1234 without quotes an integer or some kind \
    of string? Is true a string or a boolean?",
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

  let opts = Options::new(&matches, &vars);
  // The file we're operating on
  if matches.has_match("input-file") {
    if let Some(f) = matches.get("input-file") {
      process_document(f, &opts, &matches, &vars);
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
          process_document(&input.trim().to_string(), &opts, &matches, &vars);
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

struct Options {
  true_vals: String,
  false_vals: String,
  separator: String,
  arr_start: String,
  arr_end: String,
  arr_sep: String,
  quiet: bool,
  arr_len: bool,
  strip_quotes: bool,
  print_doc: bool,
}

impl Options {
  fn new(matches: &Matches, vars: &Vars) -> Options {
    let mut opts = Options{true_vals: "true".to_string(), false_vals: "false".to_string(),
                   separator: ", ".to_string(), arr_start: "[".to_string(),
                   arr_end: "]".to_string(), arr_sep: ", ".to_string(),
                   quiet: false, arr_len: false, strip_quotes: false, print_doc: false};
    // Pre-command options
    if matches.has_match("set-true") {
      if let Some(t) = matches.get("set-true") {
        opts.true_vals = t.clone();
      } else {
        usage!(println!("Error: A required argument is missing for set-true."), &vars);
      }
    }
    if matches.has_match("set-false") {
      if let Some(f) = matches.get("set-false") {
        opts.false_vals = f.clone();
      } else {
        usage!(println!("Error: A required argument is missing for set-false."), &vars);
      }
    }
    if matches.has_match("separator") {
      if let Some(s) = matches.get("separator") {
        opts.separator = s.clone();
      } else {
        usage!(println!("Error: A required argument is missing for separator."), &vars);
      }
    }
    if matches.has_match("array-begin") {
      if let Some(b) = matches.get("array-begin") {
        opts.arr_start = b.clone();
      } else {
        usage!(println!("Error: A required argument is missing for array-begin."), &vars);
      }
    }
    if matches.has_match("array-end") {
      if let Some(e) = matches.get("array-end") {
        opts.arr_end = e.clone();
      } else {
        usage!(println!("Error: A required argument is missing for separator."), &vars);
      }
    }
    if matches.has_match("array-separator") {
      if let Some(s) = matches.get("array-separator") {
        opts.arr_sep = s.clone();
      } else {
        usage!(println!("Error: A required argument is missing for separator."), &vars);
      }
    }
    if matches.has_match("array-length") {
      opts.arr_len = true;
    }
    if matches.has_match("strip-quotes") {
      opts.strip_quotes = true;
    }
    // Post-command options
    if matches.has_match("print-doc") {
      opts.print_doc = true;
    }
    opts
  }
}

fn process_document(file_path: &str, opts: &Options, matches: &Matches, vars: &Vars) {
  let mut file: String = "".to_string();

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

  let mut command: bool = false;
  let mut result: Vec<Result<String, String>> = vec![Ok("".to_string())];
  let mut out_file = file_path;
  let mut hasval_keycount = 0;
  // Commands only one command allowed per invocation for this version
  if matches.has_match("get-value") {
    command = true;
    if let Some(k) = matches.get("get-value") {
      result.push(get_value(k, &opts.separator, opts.strip_quotes, &parser));
    } else {
      usage!(println!("Error \"{}\": A required argument is missing for g/get-value.", file_path), &vars);
    }
  }
  if result[result.len() - 1].is_ok() && matches.has_match("has-value") {
    command = true;
    if let Some(k) = matches.get("has-value") {
      result.push(has_value(k, &opts.separator, &opts.true_vals, &opts.false_vals, &mut hasval_keycount, &parser));
    } else {
      usage!(println!("Error \"{}\": A required argument is missing for has-value.", file_path), &vars);
    }
  }
  if result[result.len() - 1].is_ok() && matches.has_match("get-children") {
    command = true;
    if let Some(k) = matches.get("get-children") {
      result.push(get_children(k, &opts.separator, &opts.arr_start, &opts.arr_end, &opts.arr_sep, opts.arr_len,
        &parser));
    } else {
      usage!(println!("Error \"{}\": A required argument is missing for c/get-children.", file_path), &vars);
    }
  }
  if result[result.len() - 1].is_ok() && matches.has_match("has-children") {
    command = true;
    if let Some(k) = matches.get("has-children") {
      result.push(has_children(k, &opts.separator, &opts.true_vals, &opts.false_vals, &hasval_keycount, &parser));
    } else {
      usage!(println!("Error \"{}\": A required argument is missing for has-children.", file_path), &vars);
    }
  }
  if result[result.len() - 1].is_ok() && matches.has_match("set-value") {
    command = true;
    if let Some(kv) = matches.get("set-value") {
      result.push(set_value(kv, &opts.separator, opts.quiet, &mut parser));
      if matches.has_match("output-file") {
        match matches.get("output-file") {
          Some(out) => out_file = out,
          None => {
            usage!(println!("Error \"{}\": A required argument is missing for output-file.", file_path), &vars);
          },
        }
      }
      if result[result.len() - 1].is_ok() {
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
  if !command {
    // No command specified print usage
    usage!(println!("Error \"{}\": No command was specified.", file_path), &vars);
  }

  // ************** Print output here! *******************
  let _ = result.remove(0);
  for i in 0..result.len() {
    match result[i]  {
      Ok(ref val) => {
        if !opts.quiet {
          print!("{}", val);
        }
      },
      Err(ref err) => {
        print!("Error \"{}\": {}", file_path, err);
      }
    }
    if i < result.len() - 1 {
      print!("{}", opts.separator);
    }
    if i == result.len() - 1 {
      println!();
    }
  }

  // Post-command options
  if opts.print_doc {
    print_doc(&parser);
  }
}

fn write_to_file(file_path: &str, doc: &TOMLParser) -> Result<(), Error> {
  let mut f = File::create(file_path)?;
  f.write_all(format!("{}",doc).as_bytes())?;
  f.sync_all()?;
  Ok(())
}

fn get_file(file_path: &str, out_file: &mut String) -> Result<(), Error> {
  let mut f = File::open(file_path)?;
  f.read_to_string(out_file)?;
  Ok(())
}

fn get_value(csv: &str, sep: &str, strip_quotes: bool, doc: &TOMLParser) -> Result<String, String> {
  let key_results = csv_to_vec(csv);
  if let Ok(keys) = key_results {
    if keys.is_empty() {
      return Err(format!("No keys specified: \"{}\".", csv));
    }
    let mut result = String::new();
    for i in 0..keys.len() {
      let key: &str = &keys[i];
      if let Some(value) = doc.get_value(key) {
        if strip_quotes {
          result.push_str(&format!("{}", value).trim_matches(|c| c == '\'' || c == '\"'));
        } else {
          result.push_str(&format!("{}", value));
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
  Err(format!("Could not parse keys: \"{:?}\".", csv))
}

fn has_value(csv: &str, sep: &str, true_vals: &str, false_vals: &str, keycount: &mut usize, doc: &TOMLParser)
  -> Result<String, String> {
  let key_results = csv_to_vec(csv);
  let true_results = csv_to_vec(true_vals);
  let false_results = csv_to_vec(false_vals);
  if let Ok(keys) = key_results {
    if keys.is_empty() {
      return Err(format!("No keys specified: \"{}\".", csv));
    }
    *keycount = keys.len();
    let mut result = String::new();
    for i in 0..keys.len() {
      let (true_val, false_val);
      if let Ok(ref true_vals) = true_results {
        if true_vals.len() > 1 {
          true_val = &true_vals[i];
        } else {
          true_val = &true_vals[0];
        }
      } else {
        return Err(format!("Invalid set of true values specified: \"{}\".", true_vals));
      };
      if let Ok(ref false_vals) = false_results {
        if false_vals.len() > 1 {
          false_val = &false_vals[i];
        } else {
          false_val = &false_vals[0];
        }
      } else {
        return Err(format!("Invalid set of false values specified: \"{}\".", false_vals));
      }
      let key: &str = &keys[i];
      if doc.get_value(key).is_some() {
        result.push_str(&true_val);
      } else {
        result.push_str(&false_val);
      }
      if i < keys.len() - 1 {
        result.push_str(sep);
      }
    }
    return Ok(result);
  }
  Err(format!("Could not parse keys: \"{}\".", csv))
}

fn get_children(csv: &str, sep: &str, arr_start: &str, arr_end: &str, arr_sep: &str, arr_len: bool,
  doc: &TOMLParser) -> Result<String, String> {
  let key_results = csv_to_vec(csv);
  if let Ok(keys) = key_results {
    if keys.is_empty() {
      return Err(format!("No keys specified: \"{}\".", csv));
    }
    let mut result = String::new();
    for i in 0..keys.len() {
      let key: &str = &keys[i];
      if let Some(c) = doc.get_children(key) {
        match *c {
          Children::Keys(ref ckeys) => {
            let mut val = String::new();
            if arr_len {
              val.push_str(&format!("{}", ckeys.borrow().len()));
              val.push_str(arr_sep);
            } else {
              val.push_str(arr_start);
            }
            if ckeys.borrow().len() > 0 {
              for i in 0..ckeys.borrow().len() - 1 {
                val.push_str(&ckeys.borrow()[i]);
                val.push_str(arr_sep);
              }
              val.push_str(&ckeys.borrow()[ckeys.borrow().len() - 1]);
            }
            if !arr_len {
              val.push_str(arr_end);
            }
            result.push_str(&val);
          },
          Children::Count(ref size) => {
            if size.get() == 0 {
              return Err(format!("Key \"{}\" has no children.", key));
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

fn has_children(csv: &str, sep: &str, true_vals: &str, false_vals: &str, keycount: &usize, doc: &TOMLParser)
  -> Result<String, String> {
  let key_results = csv_to_vec(csv);
  let true_results = csv_to_vec(true_vals);
  let false_results = csv_to_vec(false_vals);
  if let Ok(keys) = key_results {
    if keys.is_empty() {
      return Err(format!("No keys specified: \"{}\".", csv));
    }
    let mut result = String::new();
    for i in 0..keys.len() {
      let (true_val, false_val);
      if let Ok(ref true_vals) = true_results {
        if true_vals.len() > 1 {
          true_val = &true_vals[i+keycount];
        } else {
          true_val = &true_vals[0];
        }
      } else {
        return Err(format!("Invalid set of true values specified: \"{}\".", true_vals));
      };
      if let Ok(ref false_vals) = false_results {
        if false_vals.len() > 1 {
          false_val = &false_vals[i+keycount];
        } else {
          false_val = &false_vals[0];
        }
      } else {
        return Err(format!("Invalid set of false values specified: \"{}\".", false_vals));
      }
      let key: &str = &keys[i];
      if let Some(children) = doc.get_children(key) {
        match *children {
          Children::Count(ref c) => {
            if c.get() > 0 {
              result.push_str(&true_val);
            } else {
              result.push_str(&false_val);
            }
          },
          Children::Keys(ref ckeys) => {
            if ckeys.borrow().len() > 0 {
              result.push_str(&true_val);
            } else {
              result.push_str(&false_val);
            }
          }
        }
      } else {
        result.push_str(&false_val);
      }
      if i < keys.len() - 1 {
        result.push_str(sep);
      }
    }
    return Ok(result);
  }
  Err(format!("Could not parse keys: \"{}\".", csv))
}

#[allow(clippy::collapsible_match)]
fn set_value(kvs: &str, sep: &str, quiet: bool, doc: &mut TOMLParser) -> Result<String, String> {
  let keyval_results = csv_to_vec(kvs);
  if let Ok(keyvals) = keyval_results {
    if keyvals.len() % 3 != 0 || keyvals.is_empty() {
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
                  if let TimeOffset::Time(ref amount) = *offset {
                    has_offset = true;
                    pos_neg = amount.pos_neg;
                    off_hour = amount.hour.to_string().into();
                    off_minute = amount.minute.to_string().into();
                  }
                }
              }

              let newoffset = if has_offset {
                Some(TimeOffset::Time(TimeOffsetAmount{pos_neg, hour: off_hour, minute: off_minute}))
              } else {
                None
              };
              let newfraction = if has_fraction {
                Some(fraction)
              } else {
                None
              };
              let newtime = if has_time {
                Some(Time{hour, minute, second, fraction: newfraction, offset: newoffset})
              } else {
                None
              };
              new_dt = DateTime{
                date: Date{
                  year,
                  month,
                  day,
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
      if !quiet && i * 3  < keyvals.len() - 3 {
        result.push_str(sep);
      }
    }
    return Ok(result);
  }
  Err(format!("Could not parse keys: \"{}\".", kvs))
}

fn csv_to_vec(csv: &str) -> Result<Vec<String>, csv::Error> {
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
