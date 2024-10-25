// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod commentcleaner;
mod lexer;
mod macroexpander;
mod normalizer;
mod parser;
mod token;

pub use parser::parse_from_str;