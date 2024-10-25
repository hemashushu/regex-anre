// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// syntax summary
//
// meta characters and their meanings:
//
// - [ ]      charset
// - [^ ]     negative charset
// - {m}      repeat
// - {m,n}    repeat range
// - {m,}     at least
// - (xyz)    group
// - *        0 or more
// - +        1 or more
// - ?        optional or lazy repetition
// - |        logic or
// - ^        beginning assertion
// - $        end assertion
// - .        any character except new line (\r and \n)
// - \        to form escape character
//
// meta characters `( ) { } [ ] + * ? . | ^ $ \` need to be escaped when
// used as literal characters, e.g. `\(`, `\*` and `\.`
// whereas, only `[`, `]` and `\` need to be escaped in a charset, and also
// if the hyphen `-` is not in the first or last position in the charset, e.g.:
// `[ab-]`, `[a\-b]`
//
// escaped characters:
//
// - \t       horizontal tab
// - \n       new line
// - \r       carriage return
// - \u{hhhh} unicode

// - \f       form feed  (x)
// - \v       vertical tab  (x)
// - \0       (x)

//
// preset charsets:
//
// - \w       alphanumeric characters: [a-zA-Z0-9_]
// - \W       [^\w]
// - \d       digits: [0-9]
// - \D       [^\d]
// - \s       whitespaces [ \t\r\n\v\f]
// - \S       [^\s]
//
// boundary assertions:
// - \b       word boundary
// - \B       not word boundary
//
// non-capturing groups:
// - (?:...)
//
// named capture group
// - (?<name>...)
//
// back references:
// - \number  by number, e.g. `\1`, `\2` and `\3`
// - \k<name> by name
//
// lookaround assertions:
//
// - (?=...)  lookahead
// - (?!...)  negative lookahead
// - (?<=...) lookbehind
// - (?<!...) negative lookbehind

use crate::{
    charposition::CharWithPosition, error::Error, location::Location, peekableiter::PeekableIter,
};

use super::token::{Token, TokenWithRange};

pub const LEXER_PEEK_CHAR_MAX_COUNT: usize = 4;

pub fn lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, Error> {
    todo!()
}

struct Lexer<'a> {
    upstream: &'a mut PeekableIter<'a, CharWithPosition>,
    last_position: Location,
    saved_positions: Vec<Location>,
}

impl<'a> Lexer<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, CharWithPosition>) -> Self {
        Self {
            upstream,
            last_position: Location::new_position(0, 0, 0, 0),
            saved_positions: vec![],
        }
    }

    fn next_char(&mut self) -> Option<char> {
        match self.upstream.next() {
            Some(CharWithPosition {
                character,
                position,
            }) => {
                self.last_position = position;
                Some(character)
            }
            None => None,
        }
    }

    fn peek_char(&self, offset: usize) -> Option<&char> {
        match self.upstream.peek(offset) {
            Some(CharWithPosition { character, .. }) => Some(character),
            None => None,
        }
    }

    fn peek_char_and_equals(&self, offset: usize, expected_char: char) -> bool {
        matches!(
            self.upstream.peek(offset),
            Some(CharWithPosition { character, .. }) if character == &expected_char)
    }

    fn peek_position(&self, offset: usize) -> Option<&Location> {
        match self.upstream.peek(offset) {
            Some(CharWithPosition { position, .. }) => Some(position),
            None => None,
        }
    }

    fn push_peek_position(&mut self) {
        self.saved_positions.push(*self.peek_position(0).unwrap());
    }

    fn pop_saved_position(&mut self) -> Location {
        self.saved_positions.pop().unwrap()
    }
}

impl<'a> Lexer<'a> {
    fn lex(&mut self) -> Result<Vec<TokenWithRange>, Error> {
        let mut token_with_ranges = vec![];

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '[' => {
                    let mut twrs = self.lex_charset()?;
                    token_with_ranges.append(&mut twrs);
                }
                ']' => {
                    self.next_char(); // consume ']'
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::RightBracket,
                        &self.last_position,
                        1,
                    ));
                }
                '{' => {
                    self.next_char(); // consume '{'
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::LeftBrace,
                        &self.last_position,
                        1,
                    ));
                }
                '}' => {
                    self.next_char(); // consume '}'
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::RightBrace,
                        &self.last_position,
                        1,
                    ));
                }
                '(' => {
                    self.next_char(); // consume '('
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::LeftParen,
                        &self.last_position,
                        1,
                    ));
                }
                ')' => {
                    self.next_char(); // consume ')'
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::RightParen,
                        &self.last_position,
                        1,
                    ));
                }
                '?' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position();

                    self.next_char(); // consume '?'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::QuestionLazy,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '?' => {
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Question,
                        &self.last_position,
                        1,
                    ))
                }
                '+' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position();

                    self.next_char(); // consume '+'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::PlusLazy,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '+' => {
                    self.next_char(); // consume '+'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Plus,
                        &self.last_position,
                        1,
                    ))
                }
                '*' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position();

                    self.next_char(); // consume '*'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::AsteriskLazy,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '*' => {
                    self.next_char(); // consume '*'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Asterisk,
                        &self.last_position,
                        1,
                    ))
                }
                '^' => {
                    self.next_char(); // consume '^'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Caret,
                        &self.last_position,
                        1,
                    ))
                }
                '$' => {
                    self.next_char(); // consume '$'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Dollar,
                        &self.last_position,
                        1,
                    ))
                }
                '.' => {
                    self.next_char(); // consume '.'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Dot,
                        &self.last_position,
                        1,
                    ))
                }
                '\\' => {
                    let twr = self.lex_main_escaping()?;
                    token_with_ranges.push(twr);
                }
                _ => {
                    let c = *current_char;
                    self.next_char(); // consume current char

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Char(c),
                        &self.last_position,
                        1,
                    ))
                }
            }
        }

        Ok(token_with_ranges)
    }

    fn lex_main_escaping(&mut self) -> Result<TokenWithRange, Error> {
        // \xxxx?  //
        // ^    ^__// to here
        // |_______// current char, validated

        self.push_peek_position();

        self.next_char(); // consume '\'

        let token = match self.next_char() {
            Some(previous_char) => {
                match previous_char {
                    // escaped literal char
                    '\\' => Token::Char('\\'),
                    't' => Token::Char('\t'), // horizontal tabulation
                    'r' => Token::Char('\r'), // carriage return (CR, ascii 13)
                    'n' => Token::Char('\n'), // new line character (line feed, LF, ascii 10)
                    'u' => {
                        let c = if self.peek_char_and_equals(0, '{') {
                            // unicode code point, e.g. '\u{2d}', '\u{6587}'
                            self.unescape_unicode()?
                        } else {
                            return Err(Error::MessageWithLocation(
                                "Missing the brace for unicode escape sequence.".to_owned(),
                                self.last_position.move_position_forward(),
                            ));
                        };
                        Token::Char(c)
                    }
                    // meta chars
                    '(' | ')' | '{' | '}' | '[' | ']' | '+' | '*' | '?' | '.' | '|' | '^' | '$' => {
                        Token::Char(previous_char)
                    }
                    // preset charsets
                    'w' | 'W' | 'd' | 'D' | 's' | 'S' => Token::PresetCharSet(previous_char),
                    // boundary assertions
                    'b' | 'B' => Token::BoundaryAssertion(previous_char),
                    // back reference by index
                    '1'..'9' => {
                        let num = self.lex_number()?;
                        Token::BackReferenceNumber(num)
                    }
                    // back reference by name
                    'k' => {
                        let s = if self.peek_char_and_equals(0, '<') {
                            self.lex_identifier()?
                        } else {
                            return Err(Error::MessageWithLocation(
                                "Missing the angle bracket for group name.".to_owned(),
                                self.last_position.move_position_forward(),
                            ));
                        };
                        Token::BackReferenceIdentifier(s)
                    }
                    _ => {
                        return Err(Error::MessageWithLocation(
                            format!("Unexpected escape char '{}'.", previous_char),
                            self.last_position,
                        ));
                    }
                }
            }
            None => {
                // `\` | EOF
                return Err(Error::UnexpectedEndOfDocument(
                    "Incomplete escape character sequence.".to_owned(),
                ));
            }
        };

        let token_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(token, token_range))
    }

    fn unescape_unicode(&mut self) -> Result<char, Error> {
        // \u{6587}?  //
        //   ^     ^__// to here
        //   |________// current char, validated

        self.push_peek_position();

        self.next_char(); // comsume char '{'

        let mut codepoint_string = String::new();

        loop {
            match self.next_char() {
                Some(previous_char) => match previous_char {
                    '}' => break,
                    '0'..='9' | 'a'..='f' | 'A'..='F' => codepoint_string.push(previous_char),
                    _ => {
                        return Err(Error::MessageWithLocation(
                            format!(
                                "Invalid character '{}' for unicode escape sequence.",
                                previous_char
                            ),
                            self.last_position,
                        ));
                    }
                },
                None => {
                    // EOF
                    return Err(Error::UnexpectedEndOfDocument(
                        "Incomplete unicode escape sequence.".to_owned(),
                    ));
                }
            }

            if codepoint_string.len() > 6 {
                break;
            }
        }

        let codepoint_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        if codepoint_string.len() > 6 {
            return Err(Error::MessageWithLocation(
                "Unicode point code exceeds six digits.".to_owned(),
                codepoint_range,
            ));
        }

        if codepoint_string.is_empty() {
            return Err(Error::MessageWithLocation(
                "Empty unicode code point.".to_owned(),
                codepoint_range,
            ));
        }

        let codepoint = u32::from_str_radix(&codepoint_string, 16).unwrap();

        if let Some(c) = char::from_u32(codepoint) {
            // valid code point:
            // 0 to 0x10FFFF, inclusive
            //
            // ref:
            // https://doc.rust-lang.org/std/primitive.char.html
            Ok(c)
        } else {
            Err(Error::MessageWithLocation(
                "Invalid unicode code point.".to_owned(),
                codepoint_range,
            ))
        }
    }

    fn lex_number(&mut self) -> Result<usize, Error> {
        // 123456N  //
        // ^     ^__// to here
        // |________// current char, validated
        //
        // T = not a number || EOF

        let mut num_string = String::new();

        self.push_peek_position();

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' => {
                    // valid digits for decimal number
                    num_string.push(*current_char);

                    self.next_char(); // consume digit
                }
                _ => {
                    break;
                }
            }
        }

        let num_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        let num = num_string.parse::<usize>().map_err(|_| {
            Error::MessageWithLocation(
                format!("Can not convert \"{}\" to integer number.", num_string),
                num_range,
            )
        })?;

        Ok(num)
    }

    fn lex_charset(&mut self) -> Result<Vec<TokenWithRange>, Error> {
        //         '[' if self.peek_char_and_equals(1, '^') => {
        //             self.push_peek_position();
        //
        //             self.next_char(); // consume '['
        //             self.next_char(); // consume '^'
        //
        //             token_ranges.push(TokenWithRange::from_position_and_length(
        //                 Token::LeftBracketNegative,
        //                 &self.pop_saved_position(),
        //                 2,
        //             ));
        //
        //             let mut trs = self.lex_tokens_within_charset()?;
        //             token_ranges.append(&mut trs);
        //         }
        //         '[' => {
        //             self.next_char(); // consume '['
        //             token_ranges.push(TokenWithRange::from_position_and_length(
        //                 Token::RightBracket,
        //                 &self.last_position,
        //                 1,
        //             ));
        //
        //             let mut trs = self.lex_tokens_within_charset()?;
        //             token_ranges.append(&mut trs);
        //         }

        todo!()
    }

    fn lex_identifier(&mut self) -> Result<String, Error> {
        // <name>?  //
        // ^     ^__// to here
        // |________// current char, validated

        self.next_char(); // consume '<'

        let mut name_string = String::new();

        // self.push_peek_position();

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {
                    name_string.push(*current_char);
                    self.next_char(); // consume char
                }
                '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' => {
                    // A char is a ‘Unicode scalar value’, which is any ‘Unicode code point’ other than a surrogate code point.
                    // This has a fixed numerical definition: code points are in the range 0 to 0x10FFFF,
                    // inclusive. Surrogate code points, used by UTF-16, are in the range 0xD800 to 0xDFFF.
                    //
                    // check out:
                    // https://doc.rust-lang.org/std/primitive.char.html
                    //
                    // CJK chars: '\u{4e00}'..='\u{9fff}'
                    // for complete CJK chars, check out Unicode standard
                    // Ch. 18.1 Han CJK Unified Ideographs
                    //
                    // summary:
                    // Block Location Comment
                    // CJK Unified Ideographs 4E00–9FFF Common
                    // CJK Unified Ideographs Extension A 3400–4DBF Rare
                    // CJK Unified Ideographs Extension B 20000–2A6DF Rare, historic
                    // CJK Unified Ideographs Extension C 2A700–2B73F Rare, historic
                    // CJK Unified Ideographs Extension D 2B740–2B81F Uncommon, some in current use
                    // CJK Unified Ideographs Extension E 2B820–2CEAF Rare, historic
                    // CJK Unified Ideographs Extension F 2CEB0–2EBEF Rare, historic
                    // CJK Unified Ideographs Extension G 30000–3134F Rare, historic
                    // CJK Unified Ideographs Extension H 31350–323AF Rare, historic
                    // CJK Compatibility Ideographs F900–FAFF Duplicates, unifiable variants, corporate characters
                    // CJK Compatibility Ideographs Supplement 2F800–2FA1F Unifiable variants
                    //
                    // https://www.unicode.org/versions/Unicode15.0.0/ch18.pdf
                    // https://en.wikipedia.org/wiki/CJK_Unified_Ideographs
                    // https://www.unicode.org/versions/Unicode15.0.0/
                    //
                    // see also
                    // https://www.unicode.org/reports/tr31/tr31-37.html

                    name_string.push(*current_char);
                    self.next_char(); // consume char
                }
                '>' => {
                    // terminator char
                    break;
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        format!("Invalid char '{}' for identifier.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        // let name_range = Location::from_position_pair_with_end_included(
        //     &self.pop_saved_position(),
        //     &self.last_position,
        // );

        self.next_char(); // consume '>'

        Ok(name_string)
    }
}
