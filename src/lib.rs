// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod anre;
mod ast;
mod charwithposition;
mod compiler;
mod errorprinter;
mod instance;
mod location;
mod peekableiter;
mod printer;
mod rulechecker;
mod tradition;
mod transition;
mod utf8reader;

pub mod process;
pub mod route;

pub use process::Regex;

use std::fmt::{self, Display};

use crate::location::Location;

#[derive(Debug, PartialEq, Clone)]
pub enum AnreError {
    SyntaxIncorrect(String),
    UnexpectedEndOfDocument(String),

    // note that the "index" (and the result of "index+length") may exceed
    // the last index of string, for example, the "char incomplete" error raised by a string `'a`,
    // which index is 2.
    MessageWithLocation(String, Location),
}

impl Display for AnreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AnreError::SyntaxIncorrect(msg) => f.write_str(msg),
            AnreError::UnexpectedEndOfDocument(detail) => {
                writeln!(f, "Unexpected to reach the end of document.")?;
                write!(f, "{}", detail)
            }
            AnreError::MessageWithLocation(detail, location) => {
                writeln!(
                    f,
                    "Error at line: {}, column: {}",
                    location.line + 1,
                    location.column + 1
                )?;
                write!(f, "{}", detail)
            }
        }
    }
}

impl std::error::Error for AnreError {}
