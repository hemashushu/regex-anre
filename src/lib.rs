// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

mod anre;
mod ast;
mod charwithposition;
mod compiler;
mod context;
mod errorprinter;
mod location;
mod peekableiter;
mod printer;
mod rulechecker;
mod traditional;
mod transition;
mod utf8reader;

pub mod object_file;
pub mod process;
pub mod regex;

pub use regex::Regex;

use std::fmt::{self, Display};

use crate::location::Location;

#[derive(Debug, PartialEq, Clone)]
pub enum AnreError {
    SyntaxIncorrect(String),
    UnexpectedEndOfDocument(String),

    // The "index" (and the result of "index + length") may exceed
    // the last index of the string. For example, the "char incomplete" error
    // raised by a string `'a` has an index of 2.
    MessageWithLocation(String, Location),
}

impl Display for AnreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AnreError::SyntaxIncorrect(msg) => f.write_str(msg),
            AnreError::UnexpectedEndOfDocument(detail) => {
                writeln!(f, "Unexpected end of document.")?;
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
