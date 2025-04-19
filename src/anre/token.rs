// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

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
    Range,

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

    //
    // Notations/Symbols
    //

    // '?'
    Question,

    // '??'
    QuestionLazy,

    // '+'
    Plus,

    // '+?'
    PlusLazy,

    // '*'
    Asterisk,

    // '*?'
    AsteriskLazy,

    // '{'
    LeftBrace,

    // '}'
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
