use internals::ast::structs::{Toml, NLExpression, Expression, WSSep};
use internals::parser::Parser;

impl<'a> Parser<'a> {
  method!(pub toml<Parser<'a>, &'a str, Toml>, mut self,
    chain!(
      expr: call_m!(self.expression)    ~
  nl_exprs: call_m!(self.nl_expressions),
      ||{
        let mut tmp = vec![NLExpression::new_str("", expr)];
        tmp.extend(nl_exprs); Toml{ exprs: tmp}
      }
    )
  );

  method!(nl_expressions<Parser<'a>, &'a str, Vec<NLExpression> >, mut self, many0!(call_m!(self.nl_expression)));

  method!(nl_expression<Parser<'a>, &'a str, NLExpression>, mut self,
    chain!(
       nl: call_m!(self.newline)    ~
     expr: call_m!(self.expression) ,
      ||{
        NLExpression::new_str(nl, expr)
      }
    )
  );

  method!(expression<Parser<'a>, &'a str,  Expression>, mut self,
    alt!(
      complete!(call_m!(self.table_comment))  |
      complete!(call_m!(self.keyval_comment)) |
      complete!(call_m!(self.ws_comment))     |
      complete!(call_m!(self.ws_expr))
    )
  );

  method!(ws_expr<Parser<'a>, &'a str, Expression>, mut self,
    chain!(
      ws: call_m!(self.ws),
      ||{
        Expression::new(WSSep::new_str(ws, ""), None, None, None)
      }
    ));

  method!(table_comment<Parser<'a>, &'a str, Expression>, mut self,
    chain!(
      ws1: call_m!(self.ws)                 ~
    table: call_m!(self.table)              ~
      ws2: call_m!(self.ws)                 ~
  comment: complete!(call_m!(self.comment))?,
      ||{
        Expression::new(WSSep::new_str(ws1, ws2), None, Some(table), comment)
      }
    )
  );

  method!(keyval_comment<Parser<'a>, &'a str, Expression>, mut self,
    chain!(
      ws1: call_m!(self.ws)       ~
   keyval: call_m!(self.keyval)   ~
      ws2: call_m!(self.ws)       ~
  comment: complete!(call_m!(self.comment)) ? ,
      ||{
        Expression::new(WSSep::new_str(ws1, ws2,), Some(keyval), None, comment)
      }
    )
  );

  method!(ws_comment<Parser<'a>, &'a str, Expression>, mut self,
    chain!(
       ws: call_m!(self.ws)     ~
  comment: call_m!(self.comment),
      ||{
        Expression::new(WSSep::new_str(ws, "",), None, None, Some(comment))
      }
    )
  );
}

#[cfg(test)]
mod test {
  use std::rc::Rc;
  use std::cell::RefCell;
  use nom::IResult::Done;
  use internals::parser::Parser;
  use types::{TimeOffsetAmount, DateTime, Date, Time, TimeOffset, StrType};
  use internals::ast::structs::{Expression, Comment, WSSep, KeyVal, Table, WSKeySep,
                     TableType, TOMLValue, NLExpression, ArrayValue, Toml,
                     Array, CommentOrNewLines};
  

  #[test]
  fn test_toml() {
    let p = Parser::new();
    assert_eq!(p.toml(
r#"# Tλïƨ ïƨ á TÓM£ δôçú₥èñƭ.

title = "TÓM£ Éжá₥ƥℓè"

[owner]
name = "Tô₥ Þřèƨƭôñ-Wèřñèř"
dob = 1979-05-27T07:32:00-08:00 # Fïřƨƭ çℓáƨƨ δáƭèƨ

[database]
server = "192.168.1.1"
ports = [ 8001, 8001, 8002 ]
connection_max = 5000
enabled = true"#).1, Done("",
      Toml::new(vec![
        NLExpression::new_str(
          "", Expression::new(
            WSSep::new_str("", ""), None, None, Some(Comment::new_str(
              " Tλïƨ ïƨ á TÓM£ δôçú₥èñƭ."
            ))
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), None, None, None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), Some(KeyVal::new_str(
              "title", WSSep::new_str(" ", " "),
              Rc::new(RefCell::new(TOMLValue::String("TÓM£ Éжá₥ƥℓè".into(), StrType::Basic)))
            )),
            None, None
          )
        ),
        NLExpression::new_str("\n", Expression::new(
            WSSep::new_str("", ""), None, None, None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), None, Some(Rc::new(TableType::Standard(Table::new_str(
              WSSep::new_str("", ""), "owner", vec![]
            )))), None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), Some(KeyVal::new_str(
              "name", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::String("Tô₥ Þřèƨƭôñ-Wèřñèř".into(), StrType::Basic)))
            )), None, None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", " "), Some(KeyVal::new_str(
              "dob", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::DateTime(
                DateTime::new(
                  Date::new_str("1979", "05", "27"), Some(Time::new_str("07", "32", "00", None, Some(TimeOffset::Time(
                    TimeOffsetAmount::new_str("-", "08", "00")
                  ))))
                )
              )))
            )),
            None, Some(Comment::new_str(" Fïřƨƭ çℓáƨƨ δáƭèƨ"))
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), None, None, None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), None, Some(Rc::new(TableType::Standard(Table::new_str(
              WSSep::new_str("", ""), "database", vec![]
            )))), None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), Some(KeyVal::new_str(
              "server", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::String("192.168.1.1".into(), StrType::Basic)))
            )),
            None, None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), Some(KeyVal::new_str(
              "ports", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::Array(Rc::new(RefCell::new(Array::new(
                vec![
                  ArrayValue::new(
                    Rc::new(RefCell::new(TOMLValue::Integer("8001".into()))), Some(WSSep::new_str("", " " )), vec![CommentOrNewLines::NewLines("".into())]
                  ),
                  ArrayValue::new(
                    Rc::new(RefCell::new(TOMLValue::Integer("8001".into()))), Some(WSSep::new_str("", " ")), vec![CommentOrNewLines::NewLines("".into())]
                  ),
                  ArrayValue::new(
                    Rc::new(RefCell::new(TOMLValue::Integer("8002".into()))), None, vec![CommentOrNewLines::NewLines(" ".into())]
                  )
                ],
                vec![CommentOrNewLines::NewLines(" ".into())], vec![CommentOrNewLines::NewLines("".into())]
              ))))))
            )),
            None, None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), Some(KeyVal::new_str(
              "connection_max", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::Integer("5000".into())))
            )),
            None, None
          )
        ),
        NLExpression::new_str(
          "\n", Expression::new(
            WSSep::new_str("", ""), Some(KeyVal::new_str(
              "enabled", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::Boolean(true)))
            )),
            None, None
          )
        )
      ]) 
    ));
  }

  #[test]
  fn test_nl_expressions() {
    let mut p = Parser::new();
    // allow for zero expressions
    assert_eq!(p.nl_expressions("aoeunth £ôřè₥ ïƥƨú₥ doℓôř ƨïƭ amet, çônƨèçƭeƭuř áδïƥïscïñϱ èℓïƭ").1,
      Done("aoeunth £ôřè₥ ïƥƨú₥ doℓôř ƨïƭ amet, çônƨèçƭeƭuř áδïƥïscïñϱ èℓïƭ", vec![])
    );
    p = Parser::new();
    assert_eq!(p.nl_expressions("\n[\"δřá\"]#Mèƨsaϱè\r\nkey=\"value\"#wλïƭeƨƥáçè\n").1,
      Done(
        "", vec![
          NLExpression::new_str(
            "\n", Expression::new(
              WSSep::new_str("", ""), None, Some(Rc::new(TableType::Standard(Table::new_str(
                WSSep::new_str("", ""), "\"δřá\"", vec![] )))),
              Some(Comment::new_str("Mèƨsaϱè"))
            )
          ),
          NLExpression::new_str(
            "\r\n", Expression::new(
              WSSep::new_str("", ""), Some(KeyVal::new_str(
                "key", WSSep::new_str("", ""), Rc::new(RefCell::new(TOMLValue::String("value".into(), StrType::Basic)))
              )),
              None, Some(Comment::new_str("wλïƭeƨƥáçè"))
            )
          ),
          // A whitespace expression only requires a newline, and a newline is required to terminate the comment
          // of the previous expression so expressions ending in comments always end up with an extra whitespace
          // expression at the end of the list
          // The exceptions are for characters that end comments, but are not "newlines". It's something that
          // needs to be fixed in the ABNF
          NLExpression::new_str( 
            "\n", Expression::new( 
              WSSep::new_str("", ""), None, None, None
            )
          )
        ]
      )
    );
    p = Parser::new();
    assert_eq!(p.nl_expressions("\n[[NODOTNET.\"NÓJÂVÂ\"]]").1,
      Done(
        "", vec![
          NLExpression::new_str(
            "\n", Expression::new(
              WSSep::new_str("", ""), None, Some(Rc::new(TableType::Array(Table::new_str(
                WSSep::new_str("", ""), "NODOTNET", vec![
                  WSKeySep::new_str(WSSep::new_str("", ""), "\"NÓJÂVÂ\"")],
              )))), None
            )
          )
        ]
      )
    );
  }
// named!(nl_expression<&'a str, NLExpression>,
  #[test]
  fn test_nl_expression() {
    let p = Parser::new();
    assert_eq!(p.nl_expression("\r\n   SimpleKey = 1_2_3_4_5     #  áñ áƭƭè₥ƥƭ ƭô δèƒïñè TÓM£\r\n").1,
      Done("\r\n", NLExpression::new_str(
        "\r\n", Expression::new(
         WSSep::new_str("   ", "     "), Some(KeyVal::new_str(
            "SimpleKey", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::Integer("1_2_3_4_5".into())))
          )),
         None, Some(Comment::new_str("  áñ áƭƭè₥ƥƭ ƭô δèƒïñè TÓM£"))
        )
      ))
    );
  }

  #[test]
  fn test_expression() {
    let mut p = Parser::new();
    assert_eq!(p.expression(" \t[\"δřáƒƭ\".THISKEY  . \tkeythethird] \t#Mèƨƨáϱè Rèƥℓïèδ\n").1,
      Done("\n",
        Expression::new(
          WSSep::new_str(" \t", " \t"), None, Some(Rc::new(TableType::Standard(Table::new_str(
            WSSep::new_str("", ""), "\"δřáƒƭ\"", vec![
              WSKeySep::new_str(WSSep::new_str("", ""), "THISKEY"),
              WSKeySep::new_str(WSSep::new_str("  ", " \t"), "keythethird")
            ]
          )))),
          Some(Comment::new_str("Mèƨƨáϱè Rèƥℓïèδ"))
        )
    ));
    p = Parser::new();
    assert_eq!(p.expression("\t\t\t\"řúññïñϱôúƭôƒωôřδƨ\" = 0.1  #Â₥èřïçáñ Éжƥřèƨƨ\n").1,
      Done("\n",
        Expression::new(
          WSSep::new_str("\t\t\t", "  "), Some(KeyVal::new_str(
            "\"řúññïñϱôúƭôƒωôřδƨ\"", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::Float("0.1".into())))
          )),
          None, Some(Comment::new_str("Â₥èřïçáñ Éжƥřèƨƨ"))
        )
      ));
    p = Parser::new();
    assert_eq!(p.expression("\t \t #Þℓèáƨè Ʋèřïƒ¥ Your áççôúñƭ\n").1, Done("\n",
      Expression::new(
        WSSep::new_str("\t \t ", ""), None, None, Some(Comment::new_str("Þℓèáƨè Ʋèřïƒ¥ Your áççôúñƭ"))
      )
    ));
    p = Parser::new();
    assert_eq!(p.expression("\t  \t  \t\n").1, Done("\n",
      Expression::new(
        WSSep::new_str("\t  \t  \t", ""), None, None, None,
      )));
  }

  #[test]
  fn test_ws_expr() {
    let p = Parser::new();
    assert_eq!(p.ws_expr("  \t \t \n").1, Done("\n", 
      Expression::new(WSSep::new_str("  \t \t ", ""), None, None, None)
    ));
  }

  #[test]
  fn test_table_comment() {
    let p = Parser::new();
    assert_eq!(p.table_comment(" [table.\"ƭáβℓè\"] #úñïçôřñřôβôƭ\n").1,
      Done("\n",
        Expression::new(WSSep::new_str(" ", " "), None, Some(Rc::new(TableType::Standard(Table::new_str(
            WSSep::new_str("", ""), "table", vec![
              WSKeySep::new_str(WSSep::new_str("", ""), "\"ƭáβℓè\"")
            ]
          )))),
          Some(Comment::new_str("úñïçôřñřôβôƭ"))
        )
    ));
  }

  #[test]
  fn test_keyval_comment() {
    let p = Parser::new();
    assert_eq!(p.keyval_comment(" \"Tôƭáℓℓ¥\" = true\t#λèřè ïƨ ₥¥ çô₥₥èñƭ\n").1,
      Done("\n",
        Expression::new(WSSep::new_str(" ", "\t"), Some(KeyVal::new_str(
            "\"Tôƭáℓℓ¥\"", WSSep::new_str(" ", " "), Rc::new(RefCell::new(TOMLValue::Boolean(true)))
          )),
          None, Some(Comment::new_str("λèřè ïƨ ₥¥ çô₥₥èñƭ"))
        )
    ));
  }

  #[test]
  fn test_ws_comment() {
    let p = Parser::new();
    assert_eq!(p.ws_comment(" \t #This is RÂNÐÓM §TRÌNG\n").1, Done("\n",
      Expression::new(WSSep::new_str(" \t ", ""), None, None, Some(Comment::new_str("This is RÂNÐÓM §TRÌNG"))
      )
    ));
  }
}
