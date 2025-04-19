// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// Syntax Summary:
//
// Meta characters and their meanings:
//
// - [ ]      Character set
// - [^ ]     Negated character set
// - {m}      Exact repetition (m times)
// - {m,n}    Repetition range (m to n times)
// - {m,}     At least m repetitions
// - (xyz)    Grouping
// - *        Zero or more repetitions
// - +        One or more repetitions
// - ?        Optional or lazy repetition
// - |        Logical OR
// - ^        Start-of-line assertion
// - $        End-of-line assertion
// - .        Any character except newline (\r and \n)
// - \        Escape character for special symbols
//
// Notes on escaping meta characters:
// Meta characters `( ) { } [ ] + * ? . | ^ $ \` must be escaped when used literally, e.g., `\(`, `\*`, and `\.`.
// In character sets, only `]` and `\` need escaping. The hyphen `-` must be escaped unless it is the first or last character in the set, e.g., `[ab-]`, `[a\-b]`.
//
// Escaped characters:
//
// - \t       Horizontal tab
// - \n       Newline
// - \r       Carriage return
// - \u{hhhh} Unicode character (hexadecimal code point)
//
// Unsupported escape sequences:
// - \f       Form feed
// - \v       Vertical tab
// - \0       Null character
//
// Preset character sets:
//
// - \w       Alphanumeric characters: [a-zA-Z0-9_]
// - \W       Negated \w: [^\w]
// - \d       Digits: [0-9]
// - \D       Negated \d: [^\d]
// - \s       Whitespace characters: [ \t\r\n\v\f]
// - \S       Negated \s: [^\s]
//
// Boundary assertions:
// - \b       Word boundary
// - \B       Not a word boundary
//
// Non-capturing groups:
// - (?:...)  Non-capturing group
//
// Named capture groups:
// - (?<name>...)  Named group with identifier `name`
//
// Backreferences:
// - \number  Backreference by group number, e.g., `\1`, `\2`
// - \k<name> Backreference by group name
//
// Lookaround assertions:
//
// - (?=...)  Positive lookahead
// - (?!...)  Negative lookahead
// - (?<=...) Positive lookbehind
// - (?<!...) Negative lookbehind

use crate::{
    charwithposition::{CharWithPosition, CharsWithPositionIter},
    location::Location,
    peekableiter::PeekableIter,
    AnreError,
};

use super::token::{Repetition, Token, TokenWithRange};

pub const LEXER_PEEK_CHAR_MAX_COUNT: usize = 3;

pub fn lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, AnreError> {
    let mut chars = s.chars();
    let mut char_position_iter = CharsWithPositionIter::new(&mut chars);
    let mut peekable_char_position_iter =
        PeekableIter::new(&mut char_position_iter, LEXER_PEEK_CHAR_MAX_COUNT);
    let mut lexer = Lexer::new(&mut peekable_char_position_iter);
    lexer.lex()
}

struct Lexer<'a> {
    upstream: &'a mut PeekableIter<'a, CharWithPosition>,
    last_position: Location, // last position consumed
    saved_positions: Vec<Location>,
}

impl<'a> Lexer<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, CharWithPosition>) -> Self {
        Self {
            upstream,
            last_position: Location::new_position(/*0,*/ 0, 0, 0),
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

    fn expect_char(
        &mut self,
        expected_char: char,
        char_description: &str,
    ) -> Result<(), AnreError> {
        match self.next_char() {
            Some(ch) => {
                if ch == expected_char {
                    Ok(())
                } else {
                    Err(AnreError::MessageWithLocation(
                        format!("Expect char: {}.", char_description),
                        self.last_position,
                    ))
                }
            }
            None => Err(AnreError::UnexpectedEndOfDocument(format!(
                "Expect char: {}.",
                char_description
            ))),
        }
    }
}

impl Lexer<'_> {
    fn lex(&mut self) -> Result<Vec<TokenWithRange>, AnreError> {
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
                    let twr = self.lex_repetition()?;
                    token_with_ranges.push(twr);
                }
                '(' if self.peek_char_and_equals(1, '?') => {
                    if matches!(self.peek_char(2), Some(':' | '<' | '=' | '!')) {
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
                                        // named capture group or incomplete lookbehind assertion
                                        let name = self.lex_identifier()?;
                                        token_with_ranges.push(TokenWithRange {
                                            token: Token::NamedCapture(name),
                                            range: Location::from_position_pair_with_end_included(
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
                            ':' => {
                                // non-capturing
                                self.next_char(); // consule ':'
                                token_with_ranges.push(TokenWithRange {
                                    token: Token::NonCapturing,
                                    range: Location::from_position_and_length(
                                        &self.pop_saved_position(),
                                        3,
                                    ),
                                });
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        return Err(AnreError::MessageWithLocation(
                            "Incomplete group.".to_owned(),
                            Location::from_position_and_length(&self.last_position, 2),
                        ));
                    }
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
                '|' => {
                    self.next_char(); // consume '|'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::LogicOr,
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

    fn lex_charset(&mut self) -> Result<Vec<TokenWithRange>, AnreError> {
        // [.....]?  //
        // ^      ^__// to here
        // |_________// current char, validated
        //
        // also `[^.....]`

        let mut token_with_ranges = vec![];

        self.push_peek_position();

        self.next_char(); // consume '['

        let charset_start = if self.peek_char_and_equals(0, '^') {
            self.next_char(); // consume '^'
            TokenWithRange::from_position_and_length(
                Token::CharSetStartNegative,
                &self.pop_saved_position(),
                2,
            )
        } else {
            TokenWithRange::from_position_and_length(
                Token::CharSetStart,
                &self.pop_saved_position(),
                1,
            )
        };

        token_with_ranges.push(charset_start);

        // while let Some(current_char) = self.peek_char(0) {
        loop {
            match self.peek_char(0) {
                Some(current_char) => {
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
                None => {
                    return Err(AnreError::UnexpectedEndOfDocument(
                        "Incomplete charset.".to_owned(),
                    ));
                }
            }
        }

        self.next_char(); // consume ']'

        let charset_end =
            TokenWithRange::from_position_and_length(Token::CharSetEnd, &self.last_position, 1);

        token_with_ranges.push(charset_end);

        // handle the char range, e.g.
        // [a-z]
        //  ^ ^__ // to here
        //  |____ // merge from here

        if token_with_ranges.len() > 4 {
            let mut idx = token_with_ranges.len() - 3;
            while idx > 1 {
                if matches!(
                    token_with_ranges[idx],
                    TokenWithRange {
                        token: Token::Char('-'),
                        ..
                    }
                ) {
                    let position_start = &token_with_ranges[idx - 1].range;
                    let position_end = &token_with_ranges[idx + 1].range;

                    let char_start = if let Token::Char(c) = &token_with_ranges[idx - 1].token {
                        *c
                    } else {
                        return Err(AnreError::MessageWithLocation(
                            "Expect a char for char range, e.g. \"A-Z\".".to_owned(),
                            *position_start,
                        ));
                    };

                    let char_end = if let Token::Char(c) = &token_with_ranges[idx + 1].token {
                        *c
                    } else {
                        return Err(AnreError::MessageWithLocation(
                            "Expect a char for char range, e.g. \"a-z\".".to_owned(),
                            *position_end,
                        ));
                    };

                    let token = Token::CharRange(char_start, char_end);
                    let range = Location::from_position_pair_with_end_included(
                        position_start,
                        position_end,
                    );
                    let twr = TokenWithRange::new(token, range);

                    let pos = idx - 1;
                    token_with_ranges.drain(pos..(pos + 3));
                    token_with_ranges.insert(pos, twr);

                    idx -= 2;
                } else {
                    idx -= 1;
                }
            }
        }

        Ok(token_with_ranges)
    }

    fn lex_main_escaping(&mut self) -> Result<TokenWithRange, AnreError> {
        // \xxxx?  //
        // ^    ^__// to here
        // |_______// current char, validated

        self.push_peek_position();

        self.next_char(); // consume '\'

        let token = match self.peek_char(0) {
            Some(previous_char) => {
                match previous_char {
                    // general escaped chars
                    '\\' => {
                        self.next_char();
                        Token::Char('\\')
                    }
                    't' => {
                        // horizontal tabulation
                        self.next_char();
                        Token::Char('\t')
                    }
                    'r' => {
                        // carriage return (CR, ascii 13)
                        self.next_char();
                        Token::Char('\r')
                    }
                    'n' => {
                        // new line character (line feed, LF, ascii 10)
                        self.next_char();
                        Token::Char('\n')
                    }
                    'u' => {
                        // unicode code point, e.g. '\u{2d}', '\u{6587}'
                        self.next_char(); // consume 'u'

                        if self.peek_char_and_equals(0, '{') {
                            let c = self.unescape_unicode()?;
                            Token::Char(c)
                        } else {
                            return Err(AnreError::MessageWithLocation(
                                "Missing the brace \"{\" for unicode escape sequence.".to_owned(),
                                self.last_position.move_position_forward(),
                            ));
                        }
                    }
                    // meta chars
                    '(' | ')' | '{' | '}' | '[' | ']' | '+' | '*' | '?' | '.' | '|' | '^' | '$' => {
                        let c = *previous_char;
                        self.next_char();
                        Token::Char(c)
                    }
                    // preset charsets
                    'w' | 'W' | 'd' | 'D' | 's' | 'S' => {
                        let c = *previous_char;
                        self.next_char();
                        Token::PresetCharSet(c)
                    }
                    // boundary assertions
                    'b' | 'B' => {
                        let c = *previous_char;
                        self.next_char();
                        Token::BoundaryAssertion(c)
                    }
                    // back reference by index
                    '1'..='9' => {
                        let num = self.lex_number()?;
                        Token::BackReferenceNumber(num)
                    }
                    '0' => {
                        return Err(AnreError::MessageWithLocation(
                            "Cannot back-reference group 0.".to_owned(),
                            self.last_position.move_position_forward(),
                        ));
                    }
                    // back reference by name
                    'k' => {
                        self.next_char(); // consume 'k'

                        if self.peek_char_and_equals(0, '<') {
                            let s = self.lex_identifier()?;
                            Token::BackReferenceIdentifier(s)
                        } else {
                            return Err(AnreError::MessageWithLocation(
                                "Missing the angle bracket \"<\" for group name.".to_owned(),
                                self.last_position.move_position_forward(),
                            ));
                        }
                    }
                    _ => {
                        return Err(AnreError::MessageWithLocation(
                            format!("Unsupported escape char '{}'.", previous_char),
                            Location::from_position_and_length(&self.pop_saved_position(), 2),
                        ));
                    }
                }
            }
            None => {
                // `\` | EOF
                return Err(AnreError::UnexpectedEndOfDocument(
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

    fn lex_charset_escaping(&mut self) -> Result<TokenWithRange, AnreError> {
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
                            return Err(AnreError::MessageWithLocation(
                                "Missing the brace \"{\" for unicode escape sequence.".to_owned(),
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
                        return Err(AnreError::MessageWithLocation(
                            format!(
                                "Negative char class '{}' is not supported in charset.",
                                previous_char
                            ),
                            Location::from_position_and_length(&self.pop_saved_position(), 2),
                        ));
                    }
                    'b' | 'B' => {
                        return Err(AnreError::MessageWithLocation(
                            "Boundary assertions are not supported in charset.".to_owned(),
                            Location::from_position_and_length(&self.pop_saved_position(), 2),
                        ));
                    }
                    '0'..='9' | 'k' => {
                        return Err(AnreError::MessageWithLocation(
                            "Back references are not supported in charset.".to_owned(),
                            Location::from_position_and_length(&self.pop_saved_position(), 2),
                        ));
                    }
                    _ => {
                        return Err(AnreError::MessageWithLocation(
                            format!("Unsupported escape char '{}' in charset.", previous_char),
                            Location::from_position_and_length(&self.pop_saved_position(), 2),
                        ));
                    }
                }
            }
            None => {
                // `\` | EOF
                return Err(AnreError::UnexpectedEndOfDocument(
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

    fn unescape_unicode(&mut self) -> Result<char, AnreError> {
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
                        return Err(AnreError::MessageWithLocation(
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
                    return Err(AnreError::UnexpectedEndOfDocument(
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
            return Err(AnreError::MessageWithLocation(
                "Unicode point code exceeds six digits.".to_owned(),
                codepoint_range,
            ));
        }

        if codepoint_string.is_empty() {
            return Err(AnreError::MessageWithLocation(
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
            Err(AnreError::MessageWithLocation(
                "Invalid unicode code point.".to_owned(),
                codepoint_range,
            ))
        }
    }

    fn lex_number(&mut self) -> Result<usize, AnreError> {
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
            return Err(AnreError::MessageWithLocation(
                "Expect a number.".to_owned(),
                self.last_position.move_position_forward(),
            ));
        }

        let num_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        let num = num_string.parse::<usize>().map_err(|_| {
            AnreError::MessageWithLocation(
                format!("Can not convert \"{}\" to integer number.", num_string),
                num_range,
            )
        })?;

        Ok(num)
    }

    fn lex_identifier(&mut self) -> Result<String, AnreError> {
        // <name>?  //
        // ^     ^__// to here
        // |________// current char, validated

        self.next_char(); // consume '<'

        let mut name_string = String::new();

        // while let Some(current_char) = self.peek_char(0) {
        loop {
            match self.peek_char(0) {
                Some(current_char) => match current_char {
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
                        return Err(AnreError::MessageWithLocation(
                            format!("Invalid char '{}' for identifier.", current_char),
                            *self.peek_position(0).unwrap(),
                        ));
                    }
                },
                None => {
                    return Err(AnreError::UnexpectedEndOfDocument(
                        "Incomplete identifier.".to_owned(),
                    ));
                }
            }
        }

        if name_string.is_empty() {
            return Err(AnreError::MessageWithLocation(
                "Expect an identifier".to_owned(),
                self.last_position.move_position_forward(),
            ));
        }

        self.expect_char('>', "angle bracket \">\"")?;

        Ok(name_string)
    }

    fn lex_repetition(&mut self) -> Result<TokenWithRange, AnreError> {
        // {...}?  //
        // ^    ^__// to here
        // |_______// from here, validated

        self.push_peek_position();

        self.next_char(); // consume '{'

        let from = self.lex_number()?;

        let repetition = if self.peek_char_and_equals(0, ',') {
            self.next_char(); // consume ','
            if self.peek_char_and_equals(0, '}') {
                self.next_char(); // consume '}'
                Repetition::AtLeast(from)
            } else {
                let to = self.lex_number()?;
                // consume '}'
                self.expect_char('}', "right brace \"}\"")?;
                Repetition::Range(from, to)
            }
        } else {
            // consume '}'
            self.expect_char('}', "right brace \"}\"")?;
            Repetition::Specified(from)
        };

        let lazy = if self.peek_char_and_equals(0, '?') {
            self.next_char(); // consume '?'
            true
        } else {
            false
        };

        let token = Token::Repetition(repetition, lazy);
        let range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange { token, range })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        location::Location,
        traditional::token::{Repetition, Token, TokenWithRange},
        AnreError,
    };

    use super::lex_from_str;

    fn lex_from_str_without_location(s: &str) -> Result<Vec<Token>, AnreError> {
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

        // general escaped chars
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
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('文'),
                    &Location::new_position(/*0,*/ 1, 0, 1),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('😊'),
                    &Location::new_position(/*0,*/ 2, 0, 2),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('\t'),
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    2
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('文'),
                    &Location::new_position(/*0,*/ 5, 0, 5),
                    8
                ),
            ]
        );

        // err: unsupported escape char \v
        assert!(matches!(
            lex_from_str_without_location(r#"\v"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 2,
                }
            ))
        ));

        // err: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str_without_location(r#"\x33"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 2
                }
            ))
        ));

        // err: empty unicode escape string
        // "'\\u{}'"
        //  01 2345     // index
        assert!(matches!(
            lex_from_str_without_location("'\\u{}'"),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
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
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
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
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
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
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
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
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
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
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // err: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location("'\\u1234}'"),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));
    }

    #[test]
    fn test_lex_preset_charset() {
        assert_eq!(
            lex_from_str_without_location(r#"\d\D\w\W\s\S"#).unwrap(),
            vec![
                Token::PresetCharSet('d'),
                Token::PresetCharSet('D'),
                Token::PresetCharSet('w'),
                Token::PresetCharSet('W'),
                Token::PresetCharSet('s'),
                Token::PresetCharSet('S'),
            ]
        );
    }

    #[test]
    fn test_lex_charset() {
        assert_eq!(
            lex_from_str_without_location(r#"[a文😊]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::Char('a'),
                Token::Char('文'),
                Token::Char('😊'),
                Token::CharSetEnd
            ]
        );

        assert_eq!(
            lex_from_str_without_location(r#"[^a]"#).unwrap(),
            vec![
                Token::CharSetStartNegative,
                Token::Char('a'),
                Token::CharSetEnd
            ]
        );

        // general escaped char
        assert_eq!(
            lex_from_str_without_location(r#"[\t\r\n\\\]\u{6587}]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::Char('\t'),
                Token::Char('\r'),
                Token::Char('\n'),
                Token::Char('\\'),
                Token::Char(']'),
                Token::Char('文'),
                Token::CharSetEnd
            ]
        );

        // escaped meta chars
        // note: only ']' is necessary.
        assert_eq!(
            lex_from_str_without_location(r#"[\(\)\{\}\[\]\+\*\?\.\|\^\$]"#).unwrap(),
            vec![
                Token::CharSetStart,
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
                Token::CharSetEnd
            ]
        );

        // meta chars in charset
        // note: only ']' is escaped
        assert_eq!(
            lex_from_str_without_location(r#"[(){}[\]+*?.|^$]"#).unwrap(),
            vec![
                Token::CharSetStart,
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
                Token::CharSetEnd
            ]
        );

        // range
        assert_eq!(
            lex_from_str_without_location(r#"[-a-zA-Z0-9_-]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::Char('-'),
                Token::CharRange('a', 'z'),
                Token::CharRange('A', 'Z'),
                Token::CharRange('0', '9'),
                Token::Char('_'),
                Token::Char('-'),
                Token::CharSetEnd
            ]
        );

        // preset charset
        assert_eq!(
            lex_from_str_without_location(r#"[\w\d\s]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::PresetCharSet('w'),
                Token::PresetCharSet('d'),
                Token::PresetCharSet('s'),
                Token::CharSetEnd
            ]
        );

        // location

        assert_eq!(
            lex_from_str(r#"[a文😊\t\u{5b57}0-9-]"#).unwrap(),
            //              012 3 456789012345678
            vec![
                TokenWithRange::from_position_and_length(
                    Token::CharSetStart,
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('a'),
                    &Location::new_position(/*0,*/ 1, 0, 1),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('文'),
                    &Location::new_position(/*0,*/ 2, 0, 2),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('😊'),
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('\t'),
                    &Location::new_position(/*0,*/ 4, 0, 4),
                    2
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('字'),
                    &Location::new_position(/*0,*/ 6, 0, 6),
                    8
                ),
                TokenWithRange::from_position_and_length(
                    Token::CharRange('0', '9'),
                    &Location::new_position(/*0,*/ 14, 0, 14),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('-'),
                    &Location::new_position(/*0,*/ 17, 0, 17),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::CharSetEnd,
                    &Location::new_position(/*0,*/ 18, 0, 18),
                    1
                ),
            ]
        );

        // err: missing ']'
        assert!(matches!(
            lex_from_str_without_location(r#"[abc"#),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // err: negative preset charset
        assert!(matches!(
            lex_from_str_without_location(r#"[ab\Wcd]"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 2
                }
            ))
        ));

        // err: does not suppoert boundary assertions
        assert!(matches!(
            lex_from_str_without_location(r#"[\b]"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 2
                }
            ))
        ));

        // err: unsupported escape char
        assert!(matches!(
            lex_from_str_without_location(r#"[\v]"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 2
                }
            ))
        ));

        // err: unsupported back reference - number
        assert!(matches!(
            lex_from_str_without_location(r#"[\1]"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 2
                }
            ))
        ));

        // err: unsupported back reference - name
        assert!(matches!(
            lex_from_str_without_location(r#"[\k<name>]"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 2
                }
            ))
        ));
    }

    #[test]
    fn test_lex_notations() {
        assert_eq!(
            lex_from_str_without_location(r#"a?b??c+d+?e*f*?"#).unwrap(),
            vec![
                Token::Char('a'),
                Token::Optional,
                Token::Char('b'),
                Token::OptionalLazy,
                Token::Char('c'),
                Token::OneOrMore,
                Token::Char('d'),
                Token::OneOrMoreLazy,
                Token::Char('e'),
                Token::ZeroOrMore,
                Token::Char('f'),
                Token::ZeroOrMoreLazy,
            ]
        );

        // location
        assert_eq!(
            lex_from_str(r#"a+b+?"#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('a'),
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::OneOrMore,
                    &Location::new_position(/*0,*/ 1, 0, 1),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('b'),
                    &Location::new_position(/*0,*/ 2, 0, 2),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::OneOrMoreLazy,
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    2
                ),
            ]
        );
    }

    #[test]
    fn test_lex_anchor_and_boundary_assertions() {
        assert_eq!(
            lex_from_str(r#"^\b\B$"#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::StartAssertion,
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::BoundaryAssertion('b'),
                    &Location::new_position(/*0,*/ 1, 0, 1),
                    2
                ),
                TokenWithRange::from_position_and_length(
                    Token::BoundaryAssertion('B'),
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    2
                ),
                TokenWithRange::from_position_and_length(
                    Token::EndAssertion,
                    &Location::new_position(/*0,*/ 5, 0, 5),
                    1
                ),
            ]
        );
    }

    #[test]
    fn test_lex_repetition() {
        assert_eq!(
            lex_from_str(r#"{3}{5,}{7,13}"#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Repetition(Repetition::Specified(3), false),
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Repetition(Repetition::AtLeast(5), false),
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    4
                ),
                TokenWithRange::from_position_and_length(
                    Token::Repetition(Repetition::Range(7, 13), false),
                    &Location::new_position(/*0,*/ 7, 0, 7),
                    6
                ),
            ]
        );

        assert_eq!(
            lex_from_str(r#"{3}?{5,}?{7,13}?"#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Repetition(Repetition::Specified(3), true),
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    4
                ),
                TokenWithRange::from_position_and_length(
                    Token::Repetition(Repetition::AtLeast(5), true),
                    &Location::new_position(/*0,*/ 4, 0, 4),
                    5
                ),
                TokenWithRange::from_position_and_length(
                    Token::Repetition(Repetition::Range(7, 13), true),
                    &Location::new_position(/*0,*/ 9, 0, 9),
                    7
                ),
            ]
        );

        // err: missing number
        assert!(matches!(
            lex_from_str(r#"{}"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 0
                }
            ))
        ));

        // err: expect a number
        assert!(matches!(
            lex_from_str(r#"{a}"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 0
                }
            ))
        ));

        // err: incorrect syntax
        assert!(matches!(
            lex_from_str(r#"{a}"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 0
                }
            ))
        ));

        // err: expect a number
        assert!(matches!(
            lex_from_str(r#"{1,a}"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));

        // err: incomplete
        assert!(matches!(
            lex_from_str(r#"{1,3"#),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_group_and_backreference() {
        assert_eq!(
            lex_from_str(r#"(a)(?:b)(?<c>d)\1\k<e>"#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::GroupStart,
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('a'),
                    &Location::new_position(/*0,*/ 1, 0, 1),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::GroupEnd,
                    &Location::new_position(/*0,*/ 2, 0, 2),
                    1
                ),
                // non-capturing group
                TokenWithRange::from_position_and_length(
                    Token::NonCapturing,
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('b'),
                    &Location::new_position(/*0,*/ 6, 0, 6),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::GroupEnd,
                    &Location::new_position(/*0,*/ 7, 0, 7),
                    1
                ),
                // named group
                TokenWithRange::from_position_and_length(
                    Token::NamedCapture("c".to_owned()),
                    &Location::new_position(/*0,*/ 8, 0, 8),
                    5
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('d'),
                    &Location::new_position(/*0,*/ 13, 0, 13),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::GroupEnd,
                    &Location::new_position(/*0,*/ 14, 0, 14),
                    1
                ),
                // back reference - number
                TokenWithRange::from_position_and_length(
                    Token::BackReferenceNumber(1),
                    &Location::new_position(/*0,*/ 15, 0, 15),
                    2
                ),
                // back reference - name
                TokenWithRange::from_position_and_length(
                    Token::BackReferenceIdentifier("e".to_owned()),
                    &Location::new_position(/*0,*/ 17, 0, 17),
                    5
                ),
            ]
        );

        // err: missing identifier for named group
        assert!(matches!(
            lex_from_str(r#"(?<>abc)"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));

        // err: back reference to group 0
        assert!(matches!(
            lex_from_str(r#"(a)b\0"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 5,
                    line: 0,
                    column: 5,
                    length: 0
                }
            ))
        ));

        // err: missing identifier for named back reference
        assert!(matches!(
            lex_from_str(r#"\k<>)"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));

        // err: missing '<' for named back reference
        assert!(matches!(
            lex_from_str(r#"\kabc)"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 2,
                    line: 0,
                    column: 2,
                    length: 0
                }
            ))
        ));

        println!("{:?}", lex_from_str(r#"(?abc)"#));

        // err: incomplete group structure
        assert!(matches!(
            lex_from_str(r#"(?abc)"#),
            Err(AnreError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 2
                }
            ))
        ));
    }

    #[test]
    fn test_look_around_assertions() {
        assert_eq!(
            lex_from_str(r#"(?=a)(?!b)(?<=c)(?<!d)"#).unwrap(),
            vec![
                // look ahead
                TokenWithRange::from_position_and_length(
                    Token::LookAhead,
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('a'),
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::GroupEnd,
                    &Location::new_position(/*0,*/ 4, 0, 4),
                    1
                ),
                // look ahead - negative
                TokenWithRange::from_position_and_length(
                    Token::LookAheadNegative,
                    &Location::new_position(/*0,*/ 5, 0, 5),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('b'),
                    &Location::new_position(/*0,*/ 8, 0, 8),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::GroupEnd,
                    &Location::new_position(/*0,*/ 9, 0, 9),
                    1
                ),
                // look behind
                TokenWithRange::from_position_and_length(
                    Token::LookBehind,
                    &Location::new_position(/*0,*/ 10, 0, 10),
                    4
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('c'),
                    &Location::new_position(/*0,*/ 14, 0, 14),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::GroupEnd,
                    &Location::new_position(/*0,*/ 15, 0, 15),
                    1
                ),
                // look behind - negative
                TokenWithRange::from_position_and_length(
                    Token::LookBehindNegative,
                    &Location::new_position(/*0,*/ 16, 0, 16),
                    4
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('d'),
                    &Location::new_position(/*0,*/ 20, 0, 20),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::GroupEnd,
                    &Location::new_position(/*0,*/ 21, 0, 21),
                    1
                ),
            ]
        );
    }
}
