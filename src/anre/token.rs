// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::location::Location;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // includes `\n` and `\r\n`
    NewLine,

    // `,`
    Comma,

    // `!`
    Exclamation,

    // ..
    Interval,

    // .
    Dot,

    // `||`
    LogicOr,

    // [
    LeftBracket,
    // ]
    RightBracket,

    // (
    LeftParen,
    // )
    RightParen,

    // [a-zA-Z0-9_] and '\u{a0}' - '\u{d7ff}' and '\u{e000}' - '\u{10ffff}'
    Identifier(String),
    PresetCharSet(String),
    Special(String),
    AnchorAssertion(String),
    BoundaryAssertion(String),

    Number(usize),
    Char(char),
    String(String),
    Comment(Comment),

    /*
     * Notations/Symbols
     */
    // ?
    Question,

    // ??
    QuestionLazy,

    // +
    Plus,

    // +?
    PlusLazy,

    // *
    Asterisk,

    // *?
    AsteriskLazy,

    // {
    LeftBrace,

    // }
    RightBrace,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Comment {
    // `//...`
    // note that the trailing '\n' or '\r\n' does not belong to line comment
    Line(String),

    // `/*...*/`
    Block(String),
}

impl Token {
    // for printing
    pub fn get_description(&self) -> String {
        match self {
            Token::NewLine => "new line".to_owned(),
            Token::Comma => "comma \",\"".to_owned(),
            Token::LeftBracket => "left bracket \"[\"".to_owned(),
            Token::RightBracket => "right bracket \"]\"".to_owned(),
            Token::LeftParen => "left parenthese \"(\"".to_owned(),
            Token::RightParen => "right parenthese \")\"".to_owned(),
            Token::Exclamation => "exclamation point \"!\"".to_owned(),
            Token::Interval => "interval \"..\"".to_owned(),
            Token::Dot => "dot \".\"".to_owned(),
            Token::LogicOr => "logic or \"||\"".to_owned(),
            Token::Identifier(s) => format!("identifier \"{}\"", s),
            Token::PresetCharSet(s) => format!("preset charset \"{}\"", s),
            Token::Special(s) => format!("special \"{}\"", s),
            Token::AnchorAssertion(s) => format!("anchor assertion \"{}\"", s),
            Token::BoundaryAssertion(s) => format!("boundary assertion \"{}\"", s),
            Token::Number(i) => format!("number \"{}\"", i),
            Token::Char(c) => format!("char \"{}\"", c),
            Token::String(_) => "string".to_owned(),
            Token::Comment(_) => "comment".to_owned(),
            Token::Question => "question mark \"?\"".to_owned(),
            Token::QuestionLazy => "question and question mark \"??\"".to_owned(),
            Token::Plus => "plus sign \"+\"".to_owned(),
            Token::PlusLazy => "plus and question mark \"+?\"".to_owned(),
            Token::Asterisk => "asterisk \"*\"".to_owned(),
            Token::AsteriskLazy => "asterisk and question mark \"*?\"".to_owned(),
            Token::LeftBrace => "left brace \"{\"".to_owned(),
            Token::RightBrace => "right brace \"}\"".to_owned(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TokenWithRange {
    pub token: Token,
    pub range: Location,
}

impl TokenWithRange {
    pub fn new(token: Token, range: Location) -> Self {
        Self { token, range }
    }

    pub fn from_position_and_length(token: Token, position: &Location, length: usize) -> Self {
        Self {
            token,
            range: Location::from_position_and_length(position, length),
        }
    }
}
