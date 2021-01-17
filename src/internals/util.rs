use crate::internals::ast::structs::Comment;
use crate::internals::parser::Parser;

fn not_eol(chr: char) -> bool {
  chr as u32 == 0x09 || (chr as u32 >= 0x20 && chr as u32 <= 0x10FFF)
}

impl<'a> Parser<'a> {
  // Newline
  method!(pub newline<Parser<'a>, &'a str,  &'a str>, self,
    alt!(
      complete!(tag_s!("\r\n")) => {|s| {self.line_count.set(self.line_count.get() + 1); s}}  |
      complete!(tag_s!("\n")) => {|s| {self.line_count.set(self.line_count.get() + 1); s}}
    )
  );

  // Whitespace
  method!(pub ws<Parser<'a>, &'a str,  &'a str>, self, re_find!("^( |\t)*"));

  // Comment
  method!(pub comment<Parser<'a>, &'a str,  Comment>, self,
    chain!(
               tag_s!("#")            ~
  comment_txt: take_while_s!(not_eol) ,
      ||{
        Comment::new_str(comment_txt)
      }
    )
  );
}

#[cfg(test)]
mod test {
  use nom::IResult::Done;
  use crate::internals::parser::Parser;
  use crate::internals::ast::structs::Comment;

  #[test]
  fn test_newline() {
    let mut p = Parser::new();
    assert_eq!(p.newline("\r\n").1, Done("", "\r\n"));
    p = Parser::new();
    assert_eq!(p.newline("\n").1, Done("", "\n"));
  }

  #[test]
  fn test_ws() {
    let p = Parser::new();
    assert_eq!(p.ws(" \t  ").1, Done("", " \t  "));
  }

  #[test]
  fn test_comment() {
    let p = Parser::new();
    assert_eq!(p.comment("# Hèřè'ƨ ₥¥ çô₥₥èñƭ. -?#word").1,
      Done("", Comment::new_str(" Hèřè'ƨ ₥¥ çô₥₥èñƭ. -?#word")));
  }
}