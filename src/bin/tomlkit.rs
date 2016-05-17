extern crate pirate;
use pirate::{Matches, Match, Vars, matches, usage, vars};
use std::env;

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
    "#Options.",
    "h/help#Show this screen.",
    "/set-true#For commands that print \"true\" or \"false\", this will change what value is printed for \"true\", \
      i.e. --set-true=1 will cause tomlkit to print \"1\" for \"true\" instead of \"true\".:",
    "/set-false#For commands that print \"true\" or \"false\", this will change what value is printed for \"false\", \
      i.e. --set-false=0 will cause tomlkit to print \"0\" for \"false\" instead of \"false\".:",
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

  // Switch statement
  if matches.has_match("get-value") {
    println!("You requested get-value: {}", matches.get("get-value").unwrap());
  } else if matches.has_match("has-value") {
    println!("You requested has-value: {}", matches.get("has-value").unwrap());
  } else if matches.has_match("get-children") {
    println!("You requested get-children: {}", matches.get("get-children").unwrap());
  } else if matches.has_match("has-children") {
    println!("You requested has-children: {}", matches.get("has-children").unwrap());
  } else if matches.has_match("set-value") {
    println!("You requested set-value: {}", matches.get("set-value").unwrap());
  } else if matches.has_match("set-true") {
    println!("You requested set-true: {}", matches.get("set-true").unwrap());
  } else if matches.has_match("set-false") {
    println!("You requested set-false: {}", matches.get("set-false").unwrap());
  } else {
    // No command specified print usage
    usage(&vars);
  }
}
