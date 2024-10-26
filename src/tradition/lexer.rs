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
// whereas, only `]` and `\` need to be escaped in a charset, and also
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
    charposition::{CharWithPosition, CharsWithPositionIter},
    error::Error,
    location::Location,
    peekableiter::PeekableIter,
};

use super::token::{Repetition, Token, TokenWithRange};

pub const LEXER_PEEK_CHAR_MAX_COUNT: usize = 3;

pub fn lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, Error> {
    let mut chars = s.chars();
    let mut char_position_iter = CharsWithPositionIter::new(0, &mut chars);
    let mut peekable_char_position_iter =
        PeekableIter::new(&mut char_position_iter, LEXER_PEEK_CHAR_MAX_COUNT);
    let mut lexer = Lexer::new(&mut peekable_char_position_iter);
    lexer.lex()
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

    fn expect_char(&mut self, expected_char: char, char_description: &str) -> Result<(), Error> {
        match self.next_char() {
            Some(ch) => {
                if ch == expected_char {
                    Ok(())
                } else {
                    Err(Error::MessageWithLocation(
                        format!("Expect char: {}.", char_description),
                        self.last_position,
                    ))
                }
            }
            None => Err(Error::UnexpectedEndOfDocument(format!(
                "Expect char: {}.",
                char_description
            ))),
        }
    }
}

impl<'a> Lexer<'a> {
    fn lex(&mut self) -> Result<Vec<TokenWithRange>, Error> {
        let mut token_with_ranges = vec![];

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '[' => {
                    // charset start
                    let mut twrs = self.lex_charset()?;
                    token_with_ranges.append(&mut twrs);
                }
                ']' => {
                    // charset end
                    self.next_char(); // consume ']'
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::CharSetEnd,
                        &self.last_position,
                        1,
                    ));
                }
                '{' => {
                    // repetition
                    self.push_peek_position();

                    let repetition = self.lex_repetition()?;

                    token_with_ranges.push(TokenWithRange {
                        token: Token::Repetition(repetition),
                        range: Location::from_position_pair(
                            &self.pop_saved_position(),
                            &self.last_position,
                        ),
                    });
                }
                '(' if self.peek_char_and_equals(1, '?')
                    && matches!(self.peek_char(2), Some('<' | '=' | '!')) =>
                {
                    self.push_peek_position();

                    self.next_char(); // consume '('
                    self.next_char(); // consume '?'

                    match self.peek_char(0).unwrap() {
                        '<' => {
                            match self.peek_char(1) {
                                Some('=') => {
                                    // look behind group
                                    self.next_char(); // consume '<'
                                    self.next_char(); // consume '='
                                    token_with_ranges.push(TokenWithRange {
                                        token: Token::LookBehind,
                                        range: Location::from_position_and_length(
                                            &self.pop_saved_position(),
                                            4,
                                        ),
                                    });
                                }
                                Some('!') => {
                                    // negative look behind group
                                    self.next_char(); // consume '<'
                                    self.next_char(); // consume '='
                                    token_with_ranges.push(TokenWithRange {
                                        token: Token::LookBehindNegative,
                                        range: Location::from_position_and_length(
                                            &self.pop_saved_position(),
                                            4,
                                        ),
                                    });
                                }
                                _ => {
                                    // named capture group
                                    let name = self.lex_identifier()?;
                                    token_with_ranges.push(TokenWithRange {
                                        token: Token::NamedCapture(name),
                                        range: Location::from_position_pair(
                                            &self.pop_saved_position(),
                                            &self.last_position,
                                        ),
                                    });
                                }
                            }
                        }
                        '=' => {
                            // look ahead group
                            self.next_char(); // consule '='
                            token_with_ranges.push(TokenWithRange {
                                token: Token::LookAhead,
                                range: Location::from_position_and_length(
                                    &self.pop_saved_position(),
                                    3,
                                ),
                            });
                        }
                        '!' => {
                            // negative look ahead group
                            self.next_char(); // consule '!'
                            token_with_ranges.push(TokenWithRange {
                                token: Token::LookAheadNegative,
                                range: Location::from_position_and_length(
                                    &self.pop_saved_position(),
                                    3,
                                ),
                            });
                        }
                        _ => unreachable!(),
                    }
                }
                '(' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position();

                    self.next_char(); // consume '?'
                    self.next_char(); // consume '?'
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::NonCapturing,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '(' => {
                    self.next_char(); // consume '('
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::GroupStart,
                        &self.last_position,
                        1,
                    ));
                }
                ')' => {
                    self.next_char(); // consume ')'
                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::GroupEnd,
                        &self.last_position,
                        1,
                    ));
                }
                '?' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position();

                    self.next_char(); // consume '?'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::OptionalLazy,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '?' => {
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Optional,
                        &self.last_position,
                        1,
                    ));
                }
                '+' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position();

                    self.next_char(); // consume '+'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::OneOrMoreLazy,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '+' => {
                    self.next_char(); // consume '+'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::OneOrMore,
                        &self.last_position,
                        1,
                    ));
                }
                '*' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position();

                    self.next_char(); // consume '*'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::ZeroOrMoreLazy,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '*' => {
                    self.next_char(); // consume '*'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::ZeroOrMore,
                        &self.last_position,
                        1,
                    ));
                }
                '^' => {
                    self.next_char(); // consume '^'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::StartAssertion,
                        &self.last_position,
                        1,
                    ));
                }
                '$' => {
                    self.next_char(); // consume '$'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::EndAssertion,
                        &self.last_position,
                        1,
                    ));
                }
                '.' => {
                    self.next_char(); // consume '.'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Dot,
                        &self.last_position,
                        1,
                    ));
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
                    ));
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
                    // general escaped chars
                    '\\' => Token::Char('\\'),
                    't' => Token::Char('\t'), // horizontal tabulation
                    'r' => Token::Char('\r'), // carriage return (CR, ascii 13)
                    'n' => Token::Char('\n'), // new line character (line feed, LF, ascii 10)
                    'u' => {
                        // unicode code point, e.g. '\u{2d}', '\u{6587}'
                        if self.peek_char_and_equals(0, '{') {
                            let c = self.unescape_unicode()?;
                            Token::Char(c)
                        } else {
                            return Err(Error::MessageWithLocation(
                                "Missing the brace for unicode escape sequence.".to_owned(),
                                self.last_position.move_position_forward(),
                            ));
                        }
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
                        if self.peek_char_and_equals(0, '<') {
                            let s = self.lex_identifier()?;
                            Token::BackReferenceIdentifier(s)
                        } else {
                            return Err(Error::MessageWithLocation(
                                "Missing the angle bracket for group name.".to_owned(),
                                self.last_position.move_position_forward(),
                            ));
                        }
                    }
                    _ => {
                        return Err(Error::MessageWithLocation(
                            format!("Unsupported escape char '{}'.", previous_char),
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

    fn lex_charset_escaping(&mut self) -> Result<TokenWithRange, Error> {
        // [\xxxx...]  //
        //  ^    ^_____// to here
        //  |__________// current char, validated

        self.push_peek_position();

        self.next_char(); // consume '\'

        let token = match self.next_char() {
            Some(previous_char) => {
                match previous_char {
                    // general escaped chars
                    '\\' => Token::Char('\\'),
                    't' => Token::Char('\t'), // horizontal tabulation
                    'r' => Token::Char('\r'), // carriage return (CR, ascii 13)
                    'n' => Token::Char('\n'), // new line character (line feed, LF, ascii 10)
                    'u' => {
                        // unicode code point, e.g. '\u{2d}', '\u{6587}'
                        if self.peek_char_and_equals(0, '{') {
                            let c = self.unescape_unicode()?;
                            Token::Char(c)
                        } else {
                            return Err(Error::MessageWithLocation(
                                "Missing the brace for unicode escape sequence.".to_owned(),
                                self.last_position.move_position_forward(),
                            ));
                        }
                    }
                    // meta chars
                    //
                    // note:
                    // in the charset, only the meta char ']' is required to be
                    // escaped, but the escapes of other meta chars are also supported
                    // for consistency.
                    '(' | ')' | '{' | '}' | '[' | ']' | '+' | '*' | '?' | '.' | '|' | '^' | '$' => {
                        Token::Char(previous_char)
                    }
                    // preset charsets
                    //
                    // note:
                    // only positive preset charsets are supported
                    'w' | 'd' | 's' => Token::PresetCharSet(previous_char),
                    'W' | 'D' | 'S' => {
                        return Err(Error::MessageWithLocation(
                            format!(
                                "Negative char class '{}' is not supported in charset.",
                                previous_char
                            ),
                            self.last_position,
                        ));
                    }
                    'b' | 'B' => {
                        return Err(Error::MessageWithLocation(
                            "Boundary assertions are not supported in charset.".to_owned(),
                            self.last_position,
                        ));
                    }
                    _ => {
                        return Err(Error::MessageWithLocation(
                            format!("Unsupported escape char '{}' in charset.", previous_char),
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

        if num_string.is_empty() {
            return Err(Error::MessageWithLocation(
                "Expect a number.".to_owned(),
                self.last_position,
            ));
        }

        let num = num_string.parse::<usize>().map_err(|_| {
            let num_range = Location::from_position_pair_with_end_included(
                &self.pop_saved_position(),
                &self.last_position,
            );

            Error::MessageWithLocation(
                format!("Can not convert \"{}\" to integer number.", num_string),
                num_range,
            )
        })?;

        Ok(num)
    }

    fn lex_identifier(&mut self) -> Result<String, Error> {
        // <name>?  //
        // ^     ^__// to here
        // |________// current char, validated

        self.next_char(); // consume '<'

        let mut name_string = String::new();

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

        if name_string.is_empty() {
            return Err(Error::MessageWithLocation(
                "Expect an identifier".to_owned(),
                self.last_position,
            ));
        }

        self.expect_char('>', "angle bracket \">\"")?;

        Ok(name_string)
    }

    fn lex_charset(&mut self) -> Result<Vec<TokenWithRange>, Error> {
        // [.....]?  //
        // ^      ^__// to here
        // |_________// current char, validated
        //
        // also `[^.....]`

        self.next_char(); // consume '['

        let negative = if self.peek_char_and_equals(0, '^') {
            self.next_char(); // consume '^'
            true
        } else {
            false
        };

        let mut token_with_ranges = vec![];

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '\\' => {
                    let twr = self.lex_charset_escaping()?;
                    token_with_ranges.push(twr);
                }
                ']' => {
                    break;
                }
                _ => {
                    let c = *current_char;
                    self.next_char(); // consume current char

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Char(c),
                        &self.last_position,
                        1,
                    ));
                }
            }
        }

        self.next_char(); // consume ']'

        // handle the char range, e.g. `(a-z)`
        let normalized = if token_with_ranges.len() > 2 {
            let mut twrs = vec![];
            let mut idx = 0;
            while idx < token_with_ranges.len() - 2 {
                if matches!(
                    token_with_ranges[idx + 1],
                    TokenWithRange {
                        token: Token::Char('-'),
                        ..
                    }
                ) {
                    let position_start = &token_with_ranges[idx].range;
                    let position_end = &token_with_ranges[idx + 2].range;

                    let char_start = if let Token::Char(c) = &token_with_ranges[idx].token {
                        *c
                    } else {
                        return Err(Error::MessageWithLocation(
                            "Expect a char.".to_owned(),
                            *position_start,
                        ));
                    };

                    let char_end = if let Token::Char(c) = &token_with_ranges[idx + 2].token {
                        *c
                    } else {
                        return Err(Error::MessageWithLocation(
                            "Expect a char.".to_owned(),
                            *position_end,
                        ));
                    };

                    let token = Token::CharRange(char_start, char_end);
                    let range = Location::from_position_pair(position_start, position_end);
                    twrs.push(TokenWithRange::new(token, range));

                    idx += 3;
                } else {
                    twrs.push(token_with_ranges[idx].clone());
                }
            }
            twrs
        } else {
            token_with_ranges
        };

        Ok(normalized)
    }

    fn lex_repetition(&mut self) -> Result<Repetition, Error> {
        // {...}?  //
        // ^    ^__// to here
        // |_______// from here, validated

        self.next_char(); // consume '{'

        let from = self.lex_number()?;

        let repetition = if self.peek_char_and_equals(0, ',') {
            self.next_char(); // consume ','
            if self.peek_char_and_equals(0, '}') {
                self.next_char(); // consume '}'
                Repetition::AtLeast(from)
            } else {
                let to = self.lex_number()?;
                self.expect_char('}', "right brace \"}\"")?;
                Repetition::Range(from, to)
            }
        } else {
            self.expect_char('}', "right brace \"}\"")?;
            Repetition::Specified(from)
        };

        Ok(repetition)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        error::Error,
        location::Location,
        tradition::token::{Token, TokenWithRange},
    };

    use super::lex_from_str;

    fn lex_from_str_without_location(s: &str) -> Result<Vec<Token>, Error> {
        let tokens = lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_lex_char() {
        assert_eq!(lex_from_str_without_location("").unwrap(), vec![]);

        assert_eq!(
            lex_from_str_without_location("a").unwrap(),
            vec![Token::Char('a')]
        );

        assert_eq!(
            lex_from_str_without_location("a文😊").unwrap(),
            vec![Token::Char('a'), Token::Char('文'), Token::Char('😊'),]
        );

        // escaped chars
        assert_eq!(
            lex_from_str_without_location(r#"\t\r\n\\\u{6587}"#).unwrap(),
            vec![
                Token::Char('\t'),
                Token::Char('\r'),
                Token::Char('\n'),
                Token::Char('\\'),
                Token::Char('文'),
            ]
        );

        // escaped meta chars
        assert_eq!(
            lex_from_str_without_location(r#"\(\)\{\}\[\]\+\*\?\.\|\^\$"#).unwrap(),
            vec![
                Token::Char('('),
                Token::Char(')'),
                Token::Char('{'),
                Token::Char('}'),
                Token::Char('['),
                Token::Char(']'),
                Token::Char('+'),
                Token::Char('*'),
                Token::Char('?'),
                Token::Char('.'),
                Token::Char('|'),
                Token::Char('^'),
                Token::Char('$'),
            ]
        );

        // location
        assert_eq!(
            lex_from_str(r#"a文😊\t\u{6587}"#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('a'),
                    &Location::new_position(0, 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('文'),
                    &Location::new_position(0, 1, 0, 1),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('😊'),
                    &Location::new_position(0, 2, 0, 2),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('\t'),
                    &Location::new_position(0, 3, 0, 3),
                    2
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('文'),
                    &Location::new_position(0, 5, 0, 5),
                    8
                ),
            ]
        );

        // err: unsupported escape char \v
        assert!(matches!(
            lex_from_str_without_location("'\\v'"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 2,
                    line: 0,
                    column: 2,
                    length: 0,
                }
            ))
        ));

        // err: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str_without_location("'\\x33'"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 2,
                    line: 0,
                    column: 2,
                    length: 0
                }
            ))
        ));

        // err: empty unicode escape string
        // "'\\u{}'"
        //  01 2345     // index
        assert!(matches!(
            lex_from_str_without_location("'\\u{}'"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 2
                }
            ))
        ));

        // err: invalid unicode code point, digits too much
        // "'\\u{1000111}'"
        //  01 234567890    // index
        assert!(matches!(
            lex_from_str_without_location("'\\u{1000111}'"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 8
                }
            ))
        ));

        // err: invalid unicode code point, code point out of range
        // "'\\u{123456}'"
        //  01 2345678901
        assert!(matches!(
            lex_from_str_without_location("'\\u{123456}'"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 8
                }
            ))
        ));

        // err: invalid char in the unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location("'\\u{12mn}''"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 0
                }
            ))
        ));

        // err: missing the closed brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location("'\\u{1234'"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 8,
                    line: 0,
                    column: 8,
                    length: 0
                }
            ))
        ));

        // err: incomplete unicode escape sequence, encounter EOF
        assert!(matches!(
            lex_from_str_without_location("'\\u{1234"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location("'\\u1234}'"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));
    }
}
