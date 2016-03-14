![tomllib logo](https://dislocal.com/wp-content/uploads/2016/01/tomllib_logo1.svg)          ![tomlkit logo](https://dislocal.com/wp-content/uploads/2016/01/tomlkit_logo1.svg)
#### I wrote a blog post about my adventures in creating method macros in __nom__. [Give it a read](https://wp.me/p7ikGY-3g)!
## `tomllib` is a parser, modifier, and generator for TOML files ***that doesn't judge you***! 

######It is written in Rust using [nom](https://github.com/Geal/nom). `tomlkit` is the command line tool that has the same functionality as the library  (but doesn't actually exist yet, but then again neither does the library).

[![Build Status](https://travis-ci.org/joelself/tomllib.svg?branch=master)](https://travis-ci.org/joelself/toml_parser) [![Coverage Status](https://coveralls.io/repos/joelself/tomllib/badge.svg?branch=master&service=github)](https://coveralls.io/github/joelself/tomllib?branch=master) [![ghit.me](https://ghit.me/badge.svg?repo=joelself/tomllib)](https://ghit.me/repo/joelself/tomllib)

####What does it mean that it doesn't judge me?###

`tomlib` respects your crazy indentation and whitespace scheme. It respects the order you place things. It doesn't try to reformat the file based on *somebody else's* views on file format purity. It only makes changes that you tell it to make and leaves the rest alone. Want 20 tabs between every key and `=` in a key value pair? Have at it! Want to put a comment and 5 newlines after every array value? We won't try to change your mind! Randomly placed tables and nested tables? It's your file! Do whatever you want and as long as it is within spec; we won't try to change it.

###`tomllib`###

Based on [my version](https://github.com/joelself/toml/blob/abnf/toml.abnf) of the official [TOML ABNF](https://github.com/toml-lang/toml/blob/abnf/toml.abnf#L54) (at least until they merge my changes). Currently can parse entire Unicode TOML files and reconstruct them into a perfect copy, preserving order and all whitespace. Tested with perfect output on the toml README example, the regular, hard, and unicode hard examples in the [toml test directory](https://github.com/toml-lang/toml/tree/master/tests).

Next steps for the first release are:
- [x] Back off to use beta Rust or, less likely, stable Rust
- [x] Unit tests for all parsers that don't have them yet
- [x] ***Add macros to [nom](https://github.com/joelself/nom/tree/methods) that produce and consume methods***
- [x] An integration test method that will iterate through each toml file in the assets directory (which includes the toml-test valid tests and the toml examples mentioned in the README), parse the file, then reconstruct it and compare it to the original.
- [x] TOMLValue look-up
  - [x] Sub-key list ~~for arrays and inline tables only~~ *for __any__ key or partial key*
- [x] TOMLValue modification
  - [x] TOMLValue add/delete for arrays only
  - [x] Key/TOMLValue add/delete for inline tables only
- [x] Key modification for inline tables (general key modification moved to 0.2)
- [x] Content validation, and non-failure error reporting (currently the parser doesn't fail on heterogeneous arrays, duplicate keys, or invalid tables because it wants to give you a chance to correct them rather than  force you to fix the original TOML by hand.)
  - [x] DateTime validation
  - [x] TOMLValue parsing on set_value
- [x] Convenience functions
  - [x] For creating Values, especially DateTime
  - [x] Combining a key and a subkey to a new key
- [x] Logging
- [x] Unit tests for key look-up
  - [x] Unit tests for sub-key look-up
- [x] Unit tests for key modification
- [x] An integration test that fails or returns errors for each invalid toml-test
- [ ] Add documentation on public structs, enums, functions, methods, and macros
- [ ] Add example code somewhere, possibly part of the documentation, probably some in the README too.

The TOMLParser's first release is done. You can parse any TOML document, lookup any value, get the sub-keys of any key, and modify any value to be any other value of any type. And throughout it all, it will preserve the original formatting and comments, with the exception of changes to the structure of an Array or InlineTable. All that remains is to add unit tests for key look-up, sub-key look-up, key modification, failure integration tests, and documentation.

Some other things I will probably add for future realeses:
* Element addition and deletion
* Table manipulation
* More error reporting
* ~~Type conversion~~ (moved up to v0.1.0, because it is built-in to the way things work)
* Strip extraneous spaces, newlines and comments
* User defined whitespace schemes
* Element re-ordering
* ~~For Cargo.toml files, updating dependency versions to the latest version on crates.io~~ (Application specific features are for different projects)
* ~~For Cargo.toml files, change ranges on dependency versions~~ (Application specific features are for different projects)
* Conversion to JSON and YAML

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
