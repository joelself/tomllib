![tomllib logo](https://dislocal.com/wp-content/uploads/2016/01/tomllib_logo1.svg)          ![tomlkit logo](https://dislocal.com/wp-content/uploads/2016/01/tomlkit_logo1.svg)
## The first release (0.1.1) is out! Git it on [crates.io](https://crates.io/crates/tomllib)!
#### I wrote a blog post about my adventures in creating method macros in __nom__. [Give it a read](https://wp.me/p7ikGY-3g)!
## `tomllib` is a parser, modifier, and generator for TOML files ***that doesn't judge you***! 

######It is written in Rust using [nom](https://github.com/Geal/nom). `tomlkit` is the command line tool with the same functionality as `tomllib` that is coming soon after the release of tomllib.

[![Build Status](https://travis-ci.org/joelself/tomllib.svg?branch=master)](https://travis-ci.org/joelself/toml_parser)  [![Coverage Status](https://coveralls.io/repos/github/joelself/tomllib/badge.svg?branch=master)](https://coveralls.io/github/joelself/tomllib?branch=master)  [![ghit.me](https://ghit.me/badge.svg?repo=joelself/tomllib)](https://ghit.me/repo/joelself/tomllib)

####What does it mean that it doesn't judge me?###

`tomlib` respects your crazy indentation and whitespace scheme. It respects the order you place things. It doesn't try to reformat the file based on *somebody else's* views on file format purity. It only makes changes that you tell it to make and leaves the rest alone. Want 20 tabs between every key and `=` in a key value pair? Have at it! Want to put a comment and 5 newlines after every array value? We won't try to change your mind! Randomly placed tables and nested tables? It's your file! Do whatever you want and as long as it is within spec; we won't try to change it.

Reference documentation will be found [here once I publish the crate](https://github.com/joelself/tomllib).

###`tomllib`###

Based on [my version](https://github.com/joelself/toml/blob/abnf/toml.abnf) of the official [TOML ABNF](https://github.com/toml-lang/toml/blob/abnf/toml.abnf#L54) (at least until they merge my changes). Currently can parse entire Unicode TOML files and reconstruct them into a perfect copy, preserving order and all whitespace. Tested with perfect output on the toml README example, the regular, hard, and unicode hard examples in the [toml test directory](https://github.com/toml-lang/toml/tree/master/tests) as well as all of the valid and invalid tests in [BurntSushi/toml-test](https://github.com/BurntSushi/toml-test/tree/master/tests ) (except for [one invalid test that is actually valid](https://github.com/BurntSushi/toml-test/issues/35)).

### Examples

Here's how you would parse a TOML document and then get and set values (note that due to the way the `parse` method works it takes ownership of the parser, then returns it in a tuple with the `ParseResult`):
```rust
use tomllib::TOMLParser;
use tomllib::types::Value;

let parser = TOMLParser::new();
let toml_doc = r#"
[table] # This is a comment
 "A Key" = "A Value" # This line is indented
  SomeKey = "Some Value" # This line is indented twice
"#;
let (mut parser, result) = parser.parse(toml_doc);
parser.get_value("table.SomeKey"); // gets "Some Value"
parser.set_value("table.\"A Key\"", Value::float(9.876));
parser.set_value("table.SomeKey", Value::bool(false));
```

Tables and inline tables have subkeys while arrays of tables and arrays use array indexing starting at 0 for example:
```toml
an_array = ["A", "B", "C"]
inline_table = {first = 1.1, second = 1.3}
[[array_of_table]]
[[array_of_table]]
foo = "D"
[table]
bar = "F"
[[fruit]]
  [fruit.type]
nested = {third = [{fourth = "okay"}, {fifth = "something", sixth = "baz"}]
```
In the above example the key of "C" is `an_array[2]`, the key of 1.3 is `inline_table.second`, the key of "D" is `array_of_table[1].foo`, and the key "F" is `table.bar`. You can nest inline table and arrays, so for example the of "baz" is `fruit[0].type.nested.third[1].sixth`

Here's a quick example of using the returned `ParseResult` any `ParseError`s:

```
use tomllib::TOMLParser;
use tomllib::types::{Value, ParseResult, ParseError};

let parser = TOMLParser::new();
let toml_doc = r#"
[[array_of_tables]]
 [array_of_tables.has_error]
 mixed_array = [5, true]
"#;
let (mut parser, result) = parser.parse(toml_doc);
// For brevity's sake we're only matching `FullError` `ParseResult`s and `MixedArray` `ParseError`s
match result {
  ParseResult::FullError(rrc_errors) => {
    println!("Parsed the full document, but with errors:");
    for error in rrc_errors.borrow().iter() {
      match error {
        &ParseError::MixedArray(ref key, ref line, ref column) => {
          println!("A mixed array with key {} was encountered on line {}, column {}.", key, line, column);
          assert_eq!("array_of_tables[0].has_error.mixed_array", *key);
          assert_eq!(4, *line);
          assert_eq!(0, *column); // column reporting is unimplemented so it will always be zero        
        },
        _ => assert!(false),
      }
    }
  },
  _ => assert!(false),
}
```

In this first release you can parse any TOML document, lookup any value, get the sub-keys of any key, and modify any value to be any other value of any type. And throughout it all, it will preserve the original formatting and comments, with the exception of changes to the structure of an Array or InlineTable. The caveats are that if you change the number of elements in an array or inline table then formatting is not preserved. If you keep the number elements in an array or inline table the same, but change some or all of the values then all formatting is preserved.

Some things that you can't do yet, but are planned for the next release are:

* Key/Val insertion
* Key/Val deletion
* Table and Array of Table insertion
* Table and Array of Table deletion
* Key modification
* Table and Array of Table modification
* More error reporting

For future releases here some things I plan on adding:
* Method to strip extraneous spaces, newlines and comments
* User defined whitespace schemes
* Element re-ordering
* Conversion to JSON and YAML

### Getting Started

To get the source simply ```git clone https://github.com/joelself/toml_parser.git```.
I took a dependency on `regex_macros` which requires you be on the beta version of Rust. Fortunately [multirust](https://github.com/brson/multirust) makes this dead simple without forcing all of your Rust evironment to be on Beta.

Install multirust (you'll have to [uninstall currently installed versions of Rust](https://doc.rust-lang.org/book/installing-rust.html#uninstalling)) first:

```shell
curl -sf https://raw.githubusercontent.com/brson/multirust/master/blastoff.sh | sh
```
Change into the toml_parser directory and set that directory (and that directory only) to use Beta Rust:

```shell
cd toml_parser
multirust override beta
```

You can always go back to stable or beta with ```multirust override (beta|stable)```.
To make changes fork the repository, then clone it, make changes, and issue a pull request. If you have any problems enter an issue in the issue tracker.

**I would love to hear your feedback. If there's something you would like this project to do then feel free to write up an issue about it.** If you're not comfortable writing an issue out in the open you can email me at <self@jself.io>.
