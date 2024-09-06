use std::{collections::HashMap, ops::Range, str::FromStr};

use logos::Logos;
use num::{BigInt, BigUint};

use crate::ir::*;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Keyword,
    #[regex(r"%[a-zA-Z0-9_]*")]
    ValueId,
    #[regex(r#""[^"]*""#)]
    String,
    #[regex(r"[+-]?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
    #[regex(
        r";|\}\}|\{\{|,|:|=|->|<-|=>|<=|\+|-|\*|/|==|!=|<|>|<=|>=|\||&|\^|<<|>>|\.\.|\.|\[|\]|\{|\}"
    )]
    Punct,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen
}

#[derive(Default)]
pub struct ValueResolver {
    pub values: ValueMap,
    pub resolver: HashMap<String, ValueId>,
}
fn is_num1(s: &str) -> bool { s.chars().all(|c| c.is_ascii_digit()) }
impl ValueResolver {
    pub fn new() -> Self {
        ValueResolver {
            values: Default::default(),
            resolver: HashMap::new(),
        }
    }
    pub fn resolve(&mut self, name: &str, ty: Type) -> Result<ValueId, String> {
        if name.chars().nth(0) != Some('%') {
            return Err("Value name must start with %".to_string());
        }
        let name = &name[1..];
        if let Some(value_id) = self.resolver.get(name) {
            if self.values[*value_id].ty != ty {
                return Err(format!(
                    "Type mismatch for {}: expected {:?}, found {:?}",
                    name, ty, self.values[*value_id].ty
                ));
            }
            return Ok(*value_id);
        } else {
            let vname = if is_num1(name) {
                None
            } else {
                Some(name.to_string())
            };
            let value_id = self.values.insert(Value { name: vname, ty });
            self.resolver.insert(name.to_string(), value_id);
            Ok(value_id)
        }
    }
}

fn unescape_str(s: &str) -> String {
    let mut res = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => res.push('\n'),
                Some('r') => res.push('\r'),
                Some('t') => res.push('\t'),
                Some('\\') => res.push('\\'),
                Some('"') => res.push('"'),
                Some(c) => res.push(c),
                None => res.push('\\'),
            }
        } else {
            res.push(c);
        }
    }
    res
}

pub struct Parser<'src> {
    pub lexer: logos::Lexer<'src, Token>,
    peeked: Option<(&'src str, Token)>,
    resolver: Option<ValueResolver>,
}
impl<'src> Parser<'src> {
    pub fn new(src: &'src str) -> Self {
        Parser {
            lexer: Token::lexer(src),
            peeked: None,
            resolver: None,
        }
    }
    pub fn next(&mut self) -> Result<(&'src str, Token), String> {
        if let Some(peeked) = self.peeked.take() {
            Ok(peeked)
        } else {
            match self.lexer.next() {
                Some(Ok(token)) => Ok((self.lexer.slice(), token)),
                Some(Err(_)) => Err("Invalid token found".to_string()),
                None => Err("Unexpected end of input".to_string()),
            }
        }
    }
    pub fn peek(&mut self) -> Result<(&'src str, Token), String> {
        if let Some(peeked) = self.peeked {
            Ok(peeked)
        } else {
            let peeked = self.next()?;
            self.peeked = Some(peeked);
            Ok(peeked)
        }
    }
    pub fn peek_any(&mut self, expected: &[Token]) -> bool {
      self.peek().map(|(_, t)| expected.contains(&t)).unwrap_or(false)
    }
    pub fn peek_fn(&mut self, f: impl FnOnce(&str, Token) -> bool) -> bool {
      self.peek().map(|(s, t)| f(s, t)).unwrap_or(false)
    }
    pub fn expect(&mut self, expected: Token) -> Result<&'src str, String> {
        let (slice, token) = self.next()?;
        if token == expected {
            Ok(slice)
        } else {
            Err(format!(
                "Expected {:?}, found {:?} at {}",
                expected, token, slice
            ))
        }
    }
    pub fn expect_any(&mut self, expected: &[Token]) -> Result<&'src str, String> {
        let (slice, token) = self.next()?;
        if expected.contains(&token) {
            Ok(slice)
        } else {
            Err(format!(
                "Expected {:?}, found {:?} at {}",
                expected, token, slice
            ))
        }
    }
    pub fn expect_str(&mut self, token: Token, s: &str) -> Result<&'src str, String> {
        let slice = self.expect(token)?;
        if slice == s {
            Ok(slice)
        } else {
            Err(format!("Expected {}, found {}", s, slice))
        }
    }
    pub fn parse_unescaped_string(&mut self) -> Result<String, String> {
        let str = self.expect(Token::String)?;
        Ok(unescape_str(&str[1..str.len() - 1]))
    }
    pub fn parse<T: Parse>(&mut self) -> Result<T, String> { T::parse(self) }
    pub fn parse_value(&mut self) -> Result<ValueId, String> {
        let vid = self.expect(Token::ValueId)?;
        self.expect_str(Token::Punct, ":")?;
        // let ty = Type::from_str(ty).map_err(|_| format!("Invalid type: {}", ty))?;
        let ty: Type = self.parse()?;
        self.resolve_value(vid, ty)
    }
    pub fn parse_list<T: Parse>(&mut self, sep: &str) -> Result<Vec<T>, String> {
        let mut vec = Vec::new();
        let left = self.expect_any(&[Token::Punct, Token::LParen])?;
        let right = match left {
            "[" => "]",
            "(" => ")",
            "{" => "}",
            _ => return Err("Invalid paren".to_string()),
        };
        loop {
            match self.peek()? {
                (p, Token::Punct) if p == sep => {
                    self.next()?;
                }
                (p, Token::Punct) if p == right => {
                    self.next()?;
                    break;
                }
                (p, Token::RParen) if p == right => {
                    self.next()?;
                    break;
                }
                _ => vec.push(self.parse()?),
            }
        }
        Ok(vec)
    }
    pub fn set_resolver(&mut self, resolver: Option<ValueResolver>) -> Option<ValueResolver> {
        std::mem::replace(&mut self.resolver, resolver)
    }
    pub fn resolve_value(&mut self, name: &str, ty: Type) -> Result<ValueId, String> {
        self.resolver.as_mut().unwrap().resolve(name, ty)
    }
    pub fn parse_or_report<T: Parse>(&mut self) -> T {
        match self.parse() {
            Ok(x) => x,
            Err(msg) => {
                self.report_error("stdin", &msg);
                panic!("Parse failed");
            }
        }
    }
    pub fn report_error(&self, name: &str, msg: &str) {
        use codespan_reporting::diagnostic::{Diagnostic, Label};
        use codespan_reporting::files::SimpleFiles;
        use codespan_reporting::term::{
            self,
            termcolor::{ColorChoice, StandardStream},
        };
        let mut files = SimpleFiles::new();
        let file_id = files.add(name, self.lexer.source());
        let diagnostic = Diagnostic::error()
            .with_message(msg)
            .with_labels(vec![Label::primary(file_id, self.lexer.span())]);
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = term::Config::default();
        term::emit(&mut writer.lock(), &config, &files, &diagnostic).unwrap();
    }
}

pub trait Parse: Sized {
    fn parse(parser: &mut Parser) -> Result<Self, String>;
}

impl Parse for String {
    fn parse(parser: &mut Parser) -> Result<Self, String> { parser.parse_unescaped_string() }
}

macro_rules! impl_parse_for_number {
    ($($t:ty),*) => {
        $(
            impl Parse for $t {
                fn parse(
                    parser: &mut Parser,
                ) -> Result<Self, String> {
                    parser
                        .expect(Token::Number)?
                        .parse::<Self>()
                        .map_err(|x| x.to_string())
                }
            }
        )*
    };
}
impl_parse_for_number!(bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, isize, usize);
impl_parse_for_number!(BigUint, BigInt);

impl Parse for ValueId {
    fn parse(parser: &mut Parser) -> Result<Self, String> { parser.parse_value() }
}

impl Parse for Type {
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    let s = parser.expect(Token::Keyword)?;
    Type::from_str(s).map_err(|_| format!("Invalid type: {}", s))
  }
}

impl<T: Parse> Parse for Vec<T> {
    fn parse(parser: &mut Parser) -> Result<Self, String> { parser.parse_list(",") }
}
impl<T: Parse, const N: usize> Parse for [T; N] {
    fn parse(parser: &mut Parser) -> Result<Self, String> {
        let res = parser.parse_list(",")?;
        res.try_into().map_err(|_| format!("Expected {} values", N))
    }
}
impl<T: Parse> Parse for Option<T> {
    fn parse(parser: &mut Parser) -> Result<Self, String> {
        let res = parser.parse_list(",")?;
        match res.len() {
            0 => Ok(None),
            1 => Ok(Some(res.into_iter().next().unwrap())),
            _ => Err("Expected at most one value".to_string()),
        }
    }
}
impl<T: Parse> Parse for Box<T> {
    fn parse(parser: &mut Parser) -> Result<Self, String> { Ok(Box::new(parser.parse()?)) }
}
impl<T: Parse> Parse for Range<T> {
    fn parse(parser: &mut Parser) -> Result<Self, String> {
        let start = parser.parse()?;
        parser.expect_str(Token::Punct, "..")?;
        let end = parser.parse()?;
        Ok(start..end)
    }
}
