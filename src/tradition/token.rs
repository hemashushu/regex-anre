// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::location::Location;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    CharSetStart,         // [
    CharSetStartNegative, // [^
    CharSetEnd,           // ]

    ZeroOrMore,     // *
    ZeroOrMoreLazy, // *?
    OneOrMore,      // +
    OneOrMoreLazy,  // +?
    Optional,       // ?
    OptionalLazy,   // ??
    LogicOr,        // `|`
    StartAssertion, // ^
    EndAssertion,   // $
    Dot,            // .

    Char(char),
    CharRange(char, char), // e.g. a-zA-Z0-9
    PresetCharSet(char),
    BoundaryAssertion(char),
    Repetition(Repetition), // {N}, {M,}, {M,N}

    GroupStart,           // (
    NonCapturing,         // (?...)
    NamedCapture(String), // (?<name>...)
    LookAhead,            // (?=...)
    LookAheadNegative,    // (?!...)
    LookBehind,           // (?<=...)
    LookBehindNegative,   // (?<!...)
    GroupEnd,             // )

    BackReferenceNumber(usize),      // \number
    BackReferenceIdentifier(String), // \k<name>
}

#[derive(Debug, PartialEq, Clone)]
pub enum Repetition {
    Specified(usize),
    AtLeast(usize),
    Range(usize, usize),
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
