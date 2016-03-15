use std::fs;
use std::fs::File;
extern crate tomllib;
extern crate env_logger;
use tomllib::TOMLParser;
use tomllib::types::ParseResult;
use std::io::{Read, BufReader};

fn verify_valid(input: String) -> (bool, Option<(String, String)>) {
  let input_copy = input.clone();
  let parser = TOMLParser::new();
  let (parser, _) = parser.parse(&input_copy);
  let result = format!("{}", parser);
  if result != input  {
    return (false, Some((input, result)));
  } else {
    return (true, None);
  }
}

fn verify_invalid(input: String) -> (bool, Option<(String, String)>) {
  let input_copy = input.clone();
  let parser = TOMLParser::new();
  let (_, result) = parser.parse(&input_copy);
  match result {
    ParseResult::Full => (false, Some((input, "".to_string()))),
    _                 => (true, None),
  }
}

fn test_all_assets(valid: bool) {
	let paths;
  if valid {
    paths = fs::read_dir("./assets/valid/").unwrap();
  } else {
    paths = fs::read_dir("./assets/invalid/").unwrap();
  }
  let mut failed: Vec<(String, Option<(String, String)>)> = vec![];

  for path in paths {
    let dir_entry = path.unwrap(); // DirEntry
    if dir_entry.file_type().unwrap().is_file() {
      let filename_osstr = dir_entry.file_name();
      let filename = filename_osstr.to_str().
      unwrap_or_else(|| "").to_string();
      if filename == "" {
        failed.push((format!("{:?}", filename_osstr), None));
        continue;
      }
      let pathbuf = dir_entry.path();
      let filepath = pathbuf.to_str().unwrap_or_else(|| {failed.push((filename.clone(), None)); ""});
      if filepath == "" {
        continue;
      }
      let file_res = File::open(filepath);
      if !file_res.is_ok() {
        failed.push((filename.clone(), None));
        continue;
      }
      let file = file_res.unwrap();
      let mut contents = BufReader::new(&file);
      let mut buffer = String::new();
      if contents.read_to_string(&mut buffer).is_ok() {
        if valid {
          let (success, in_out) = verify_valid(buffer);
          if !success {
            failed.push((filename.clone(), in_out));
          }
        } else {
          let (success, in_out) = verify_invalid(buffer);
          if !success {
            failed.push((filename.clone(), in_out));
          }
        }
      }
      else {
        failed.push((filename.clone(), None));
      }
    }
  }
  if failed.len() > 0 {
    let mut panic_string = String::new();
    for &(ref filename, ref in_out) in &failed {
      let s;
      match in_out {
        &Some((ref input, ref output)) =>
        { if valid {
          s = format!("Failed to correctly parse file \"{}\".\nExpected:\n\"{}\"\nGot:\n\"{}\"", filename, input, output);
        } else {
          s = format!("Successfully parsed invalid file \"{}\".\nInvalid input:\n\"{}\"", filename, input)
        }
      },
      &None => {
        if valid {
          s = format!("Failed to correctly parse file \"{}\"",filename);
        } else {
          s = format!("Successfully parsed invalid file \"{}\"", filename)
        }
      },
    }
    panic_string.push_str(&s);
  }
  assert!(false, panic_string);
}
}

#[test]
fn test_valid_assets() {
  let _ = env_logger::init();
  test_all_assets(true /*valid*/);
}

#[test]
fn test_invalid_assets() {
  let _ = env_logger::init();
  test_all_assets(false /*valid*/);
}