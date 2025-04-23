// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use regex_anre::Regex;

pub fn main() {
    traditional();
    anre();
}

fn traditional() {
    // Traditional regex
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    println!("{}", re.is_match("2025-04-22")); // should be true
    println!("{}", re.is_match("04-22")); // should be false
}
fn anre() {
    // ANRE regex
    let re = Regex::from_anre(
        "start, char_digit.repeat(4), '-', char_digit.repeat(2), '-', char_digit.repeat(2), end",
    )
    .unwrap();

    println!("{}", re.is_match("2025-04-22")); // should be true
    println!("{}", re.is_match("04-22")); // should be false
}
