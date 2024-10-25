// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::location::Location;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    LeftBracket,         // [
    LeftBracketNegative, // [^
    RightBracket,        // ]
    LeftBrace,           // {
    RightBrace,          // }
    LeftParen,           // (
    RightParen,          // )
    Asterisk,            // *
    AsteriskLazy,        // *?
    Plus,                // +
    PlusLazy,            // +?
    Question,            // ?
    QuestionLazy,        // ??
    Pipe,                // `|`
    Caret,               // ^
    Dollar,              // $
    Dot,                 // .

    Char(char),
    PresetCharSet(char),
    BoundaryAssertion(char),
    // [a-zA-Z0-9_] and '\u{a0}' - '\u{d7ff}' and '\u{e000}' - '\u{10ffff}'
    Identifier(String),
    Number(usize),

    Comma,  // , (within repetition)
    Hyphen, // - (within charset)

    NonCapturing,        // (?...)
    NamedCapture(String), // (?<name>...)
    LookAhead,            // (?=...)
    LookAheadNegative,    // (?!...)
    LookBehind,           // (?<=...)
    LookBehindNegative,   // (?<!...)

    BackReferenceNumber(usize),      // \number
    BackReferenceIdentifier(String), // \k<name>
}

impl Token {
    // for printing
    pub fn get_description(&self) -> String {
        match self {
            Token::LeftBracket => "left bracket \"[\"".to_owned(),
            Token::LeftBracketNegative => "left bracket with caret \"[^\"".to_owned(),
            Token::RightBracket => "right bracket \"]\"".to_owned(),
            Token::LeftBrace => "left brace \"{\"".to_owned(),
            Token::RightBrace => "right brace \"}\"".to_owned(),
            Token::LeftParen => "left parenthese \"(\"".to_owned(),
            Token::RightParen => "right parenthese \")\"".to_owned(),
            Token::Asterisk => "asterisk \"*\"".to_owned(),
            Token::AsteriskLazy => "asterisk and question mark \"*?\"".to_owned(),
            Token::Plus => "plus sign \"+\"".to_owned(),
            Token::PlusLazy => "plus and question mark \"+?\"".to_owned(),
            Token::Question => "question mark \"?\"".to_owned(),
            Token::QuestionLazy => "question and question mark \"??\"".to_owned(),
            Token::Pipe => "pipe \"|\"".to_owned(),
            Token::Caret => "caret \"^\"".to_owned(),
            Token::Dollar => "dollar sign \"$\"".to_owned(),
            Token::Dot => "dot \".\"".to_owned(),
            //
            Token::Char(c) => format!("char \"{}\"", c),
            Token::PresetCharSet(c) => format!("preset charset \"{}\"", c),
            Token::BoundaryAssertion(c) => format!("boundary assertion \"{}\"", c),
            Token::Identifier(s) => format!("identifier \"{}\"", s),
            Token::Number(i) => format!("number \"{}\"", i),
            //
            Token::Comma => "comma \",\"".to_owned(),
            Token::Hyphen => "hyphen \"-\"".to_owned(),
            //
            Token::NonCapturing => "non-capturing \"(?\"".to_owned(),
            Token::NamedCapture(_) => "named capture \"(?<name>\"".to_owned(),
            Token::LookAhead => "look ahead \"(?=\"".to_owned(),
            Token::LookAheadNegative => "negative look ahead \"(?!\"".to_owned(),
            Token::LookBehind => "look behind \"(?<=\"".to_owned(),
            Token::LookBehindNegative => "negative look behind \"(?<!\"".to_owned(),
            //
            Token::BackReferenceNumber(_) => "numeric back reference \"\\number\"".to_owned(),
            Token::BackReferenceIdentifier(_) => "identifier back reference \"\\<name>\"".to_owned(),
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
