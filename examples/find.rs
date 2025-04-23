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
    let re = Regex::new(r"#[\da-fA-F]{6}").unwrap();
    let text = "The color is #ffbb33 and the background is #bbdd99.";

    // find one
    if let Some(m) = re.find(text) {
        println!("Found match: {}", m.as_str());
    } else {
        println!("No match found");
    }

    // find all
    let matches: Vec<_> = re.find_iter(text).collect();
    for m in matches {
        println!("Found match: {}", m.as_str());
    }
}

fn anre() {
    // ANRE regex
    let re = Regex::from_anre("'#', char_hex.repeat(6)").unwrap();
    let text = "The color is #ffbb33 and the background is #bbdd99.";

    // find one
    if let Some(m) = re.find(text) {
        println!("Found match: {}", m.as_str());
    } else {
        println!("No match found");
    }

    // find all
    let matches: Vec<_> = re.find_iter(text).collect();
    for m in matches {
        println!("Found match: {}", m.as_str());
    }
}
