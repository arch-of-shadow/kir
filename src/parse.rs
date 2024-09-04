use std::{any::Any, collections::HashMap, ops::Range};

use logos::Logos;
use num::{BigInt, BigUint};
use slotmap::{Key, SlotMap};

use crate::{HasResolver, HasStaticName};

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
  RParen,
}

pub fn is_num1(s: &str) -> bool {
  s.chars().all(|c| c.is_ascii_digit())
}

pub fn unescape_str(s: &str) -> String {
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

pub trait Resolver: Any {
  fn name(&self) -> &'static str;
  fn resolve(&mut self, strs: &[String]) -> Result<Box<dyn Any>, String>;
  fn update_by_name(
    &mut self,
    name: String,
    value: Box<dyn Any>,
  ) -> Result<(), String>;
}


pub struct Parser<'src> {
  pub lexer: logos::Lexer<'src, Token>,
  peeked: Option<(&'src str, Token)>,
  resolvers: HashMap<String, Box<dyn Any + 'static>>,
}
impl<'src> Parser<'src> {
  pub fn new(src: &'src str) -> Self {
    Parser {
      lexer: Token::lexer(src),
      peeked: None,
      resolvers: HashMap::new(),
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
  pub fn expect_any(
    &mut self,
    expected: &[Token],
  ) -> Result<&'src str, String> {
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
  pub fn expect_str(
    &mut self,
    token: Token,
    s: &str,
  ) -> Result<&'src str, String> {
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
  pub fn parse<T: Parse>(&mut self) -> Result<T, String> {
    T::parse(self)
  }
  pub fn parse_by_resolver<T: 'static, R: Resolver>(
    &mut self,
    name: &str,
    strs: &[String],
  ) -> Result<T, String> {
    let resolver = self
      .resolvers
      .get_mut(name)
      .ok_or_else(|| format!("No resolver for {}", name))?
      .downcast_mut::<R>()
      .ok_or_else(|| format!("Resolver type mismatch for {}", name))?;
    let res = resolver.resolve(&strs)?;
    Ok(*res.downcast::<T>().unwrap())
  }
  pub fn update_resolver<
    R: 'static + Resolver,
    T: Parse + HasResolver + 'static,
  >(
    &mut self,
    resolver_name: &str,
    value: T,
  ) -> Result<(), String> {
    let resolver = self.get_resolver_mut::<R>(resolver_name);
    let value_name = value.name();
    resolver.update_by_name(value_name, Box::new(value) as Box<dyn Any>)?;
    Ok(())
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

  pub fn parse_list_foreach<T: Parse>(
    &mut self,
    sep: &str,
    mut f: impl FnMut(&mut Self, T) -> Result<(), String>,
  ) -> Result<(), String> {
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
        _ => {
          let t = self.parse()?;
          f(self, t)?;
        }
      }
    }
    Ok(())
  }

  pub fn add_resolver(&mut self, name: &str, resolver: Box<dyn Resolver>) {
    self.resolvers.insert(name.to_string(), resolver);
  }

  pub fn pop_resolver<T: Resolver>(&mut self, name: &str) -> T {
    *(self.resolvers.remove(name).unwrap() as Box<dyn Any>)
      .downcast::<T>()
      .unwrap()
  }
  pub fn get_resolver_mut<T: Resolver>(&mut self, name: &str) -> &mut T {
    self
      .resolvers
      .get_mut(name)
      .unwrap()
      .downcast_mut::<T>()
      .expect("Resolver type mismatch")
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
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    parser.parse_unescaped_string()
  }
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
impl_parse_for_number!(
  bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, isize, usize
);
impl_parse_for_number!(BigUint, BigInt);

impl<K: Parse + Key + HasStaticName, V: Parse + HasResolver + 'static> Parse
  for SlotMap<K, V>
{
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    // treat the slotmap as Vec
    // the real data is stored in resolver
    let resolver_name = K::name();
    parser.parse_list_foreach::<V>(",", |parser, x| {
      // insert each value into the resolver
      parser.update_resolver::<V::Resolver, V>(resolver_name, x)?;
      Ok(())
    })?;

    Ok(SlotMap::with_key())
  }
}

impl<T: Parse> Parse for Vec<T> {
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    parser.parse_list(",")
  }
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
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    Ok(Box::new(parser.parse()?))
  }
}
impl<T: Parse> Parse for Range<T> {
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    let start = parser.parse()?;
    parser.expect_str(Token::Punct, "..")?;
    let end = parser.parse()?;
    Ok(start..end)
  }
}
