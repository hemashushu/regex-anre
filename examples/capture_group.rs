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
    let re =
        Regex::new(r"#(?<red>[\da-fA-F]{2})(?<green>[\da-fA-F]{2})(?<blue>[\da-fA-F]{2})").unwrap();
    let text = "The color is #ffbb33 and the background is #bbdd99.";

    // capture groups once
    if let Some(m) = re.captures(text) {
        println!("Found match: {}", m.get(0).unwrap().as_str());
        println!("Red: {}", m.name("red").unwrap().as_str());
        println!("Green: {}", m.name("green").unwrap().as_str());
        println!("Blue: {}", m.name("blue").unwrap().as_str());
    } else {
        println!("No match found");
    }

    // capture groups all
    let matches: Vec<_> = re.captures_iter(text).collect();
    for m in matches {
        println!("Found match: {}", m.get(0).unwrap().as_str());
        println!("Red: {}", m.name("red").unwrap().as_str());
        println!("Green: {}", m.name("green").unwrap().as_str());
        println!("Blue: {}", m.name("blue").unwrap().as_str());
    }
}

fn anre() {
    // ANRE regex
    let re = Regex::from_anre(
        r#"
    '#'
    char_hex.repeat(2).name("red")
    char_hex.repeat(2).name("green")
    char_hex.repeat(2).name("blue")
    "#,
    )
    .unwrap();
    let text = "The color is #ffbb33 and the background is #bbdd99.";

    // capture groups once
    if let Some(m) = re.captures(text) {
        println!("Found match: {}", m.get(0).unwrap().as_str());
        println!("Red: {}", m.name("red").unwrap().as_str());
        println!("Green: {}", m.name("green").unwrap().as_str());
        println!("Blue: {}", m.name("blue").unwrap().as_str());
    } else {
        println!("No match found");
    }

    // capture groups all
    let matches: Vec<_> = re.captures_iter(text).collect();
    for m in matches {
        println!("Found match: {}", m.get(0).unwrap().as_str());
        println!("Red: {}", m.name("red").unwrap().as_str());
        println!("Green: {}", m.name("green").unwrap().as_str());
        println!("Blue: {}", m.name("blue").unwrap().as_str());
    }
}
