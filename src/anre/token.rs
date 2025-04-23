// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use crate::location::Location;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Represents a newline (`\n` or `\r\n`).
    NewLine,

    // Represents a comma (`,`).
    Comma,

    // Represents an exclamation mark (`!`).
    Exclamation,

    // Represents a range operator (`..`).
    Range,

    // Represents a dot (`.`).
    Dot,

    // Represents a logical OR operator (`||`).
    LogicOr,

    // Represents a left square bracket (`[`).
    LeftBracket,
    // Represents a right square bracket (`]`).
    RightBracket,
    // Represents a left parenthesis (`(`).
    LeftParen,
    // Represents a right parenthesis (`)`).
    RightParen,

    // Represents an identifier, which includes alphanumeric characters and underscores.
    // [a-zA-Z0-9_] and '\u{a0}' - '\u{d7ff}' and '\u{e000}' - '\u{10ffff}'
    Identifier(String),
    // Represents a predefined character set (e.g., `char_word`).
    PresetCharSet(String),
    // Represents a special character (e.g., `char_any`).
    Special(String),
    // Represents an anchor assertion (e.g., `start`, `end`).
    AnchorAssertion(String),
    // Represents a boundary assertion (e.g., `is_bound`, `is_not_bound`).
    BoundaryAssertion(String),

    // Represents a numeric value.
    Number(usize),
    // Represents a single character.
    Char(char),
    // Represents a string literal.
    String(String),
    // Represents a comment (line or block).
    Comment(Comment),

    //
    // Quantifiers and symbols
    //

    // Represents a question mark (`?`).
    Question,

    // Represents a lazy question mark (`??`).
    QuestionLazy,

    // Represents a plus sign (`+`).
    Plus,

    // Represents a lazy plus sign (`+?`).
    PlusLazy,

    // Represents an asterisk (`*`).
    Asterisk,

    // Represents a lazy asterisk (`*?`).
    AsteriskLazy,

    // Represents a left curly brace (`{`).
    LeftBrace,

    // Represents a right curly brace (`}`).
    RightBrace,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Comment {
    // Represents a line comment (`//...`).
    // Note: The trailing newline (`\n` or `\r\n`) is not part of the comment.
    Line(String),

    // Represents a block comment (`/*...*/`).
    Block(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct TokenWithRange {
    // The token itself.
    pub token: Token,
    // The range of the token in the source code.
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
