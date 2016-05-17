extern crate pirate;
extern crate tomllib;
use std::fs::File;
use std::env;
use std::io::{Read, BufReader, Error};
use pirate::{Matches, Match, Vars, matches, usage, vars};
use tomllib::TOMLParser;
use tomllib::types::ParseResult;

fn main() {
  let options = vec![
    "Required arguments",
    "f/file#The path to the TOML document to parse and manipulate.:",
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
    "#Pre-command Options.",
    "h/help#Show this screen.",
    "/set-true#For commands that print \"true\" or \"false\", this will change what value is printed for \"true\", \
      i.e. --set-true=1 will cause tomlkit to print \"1\" for \"true\" instead of \"true\".:",
    "/set-false#For commands that print \"true\" or \"false\", this will change what value is printed for \"false\", \
      i.e. --set-false=0 will cause tomlkit to print \"0\" for \"false\" instead of \"false\".:",
    "#Post-command Options.",
    "/print-doc#Print out the resultant TOML document after all requested changes have been made.",
  ];

  let mut vars: Vars = match vars("tomlkit", &options) {
    Ok(v) => v,
    Err(e) => panic!("There was an error parsing argument definitions: {}", e)
  };

  let args: Vec<String> = env::args().collect();
  let matches: Matches = match matches(&args, &mut vars) {
    Ok(m) => m,
    Err(e) => {
      println!("Error: {}", e);
      usage(&vars);
      return;
    }
  };

  if matches.has_match("help") {
    usage(&vars);
    return;
  }
  let mut true_val = &"true".to_string();
  let mut false_val = &"false".to_string();
  let mut output: String = "".to_string();

  // The file we're operating on
  let mut file = String::new();
  if matches.has_match("file") {
    if let Some(f) = matches.get("file") {
      match get_file(f, &mut file) {
        Err(err) => {
          println!("Error: Unable to read file: \"{}\". Reason: {}", f, err);
          return;
        },
        _ => ()
      }
    } else {
      println!("Error: A required argument is missing for f/file.");
      usage(&vars);
    }
  } else

  // Pre-command options
  if matches.has_match("set-true") {
    if let Some(t) = matches.get("set-true") {
      true_val = t;
    } else {
      println!("Error: A required argument is missing for set-true.");
      usage(&vars);
    }
  } else if matches.has_match("set-false") {
    if let Some(f) = matches.get("set-false") {
      false_val = f;
    } else {
      println!("Error: A required argument is missing for set-false.");
      usage(&vars);
    }
  }

  // Commands only one command allowed per invocation for this version
  if matches.has_match("get-value") {
    if let Some(k) = matches.get("get-value") {
      output = get_value(k);
    } else {
      println!("Error: A required argument is missing for g/get-value.");
      usage(&vars);
    }
  } else if matches.has_match("has-value") {
    if let Some(k) = matches.get("has-value") {
      output = has_value(k, true_val, false_val);
    } else {
      println!("Error: A required argument is missing for has-value.");
      usage(&vars);
    }
  } else if matches.has_match("get-children") {
    if let Some(v) = matches.get("get-children") {
      output = get_children(v);
    } else {
      println!("Error: A required argument is missing for c/get-children.");
      usage(&vars);
    }
  } else if matches.has_match("has-children") {
    if let Some(k) = matches.get("has-children") {
      output = has_children(k, true_val, false_val);
    } else {
      println!("Error: A required argument is missing for has-children.");
      usage(&vars);
    }
  } else if matches.has_match("set-value") {
    if let Some(kv) = matches.get("set-value") {
      output = set_value(kv);
    } else {
      println!("Error: A required argument is missing for s/set-value.");
      usage(&vars);
    }
  } else {
    // No command specified print usage
    usage(&vars);
    return;
  }

  // Post-command options
  if matches.has_match("print-doc") {
    print_doc(&output);
  }
}

fn get_file(file_path: &String, out_file: &mut String) -> Result<usize, Error> {
  match File::open(file_path) {
    Ok(file) => {
      let mut contents = BufReader::new(&file);
      match contents.read_to_string(out_file) {
        Ok(bytes) => return Ok(bytes),
        Err(err) => return Err(err),
      }
    },
    Err(err) => return Err(err),
  }
}

fn get_value(key: &String) -> String {
  unimplemented!();
}

fn has_value(key: &String, true_val: &String, false_val: &String) -> String {
  unimplemented!();
}

fn get_children(key: &String) -> String {
  unimplemented!();
}

fn has_children(key: &String, true_val: &String, false_val: &String) -> String {
  unimplemented!();
}

fn set_value(key: &String) -> String {
  unimplemented!();
}

fn print_doc(doc: &String) {
  unimplemented!();
}
