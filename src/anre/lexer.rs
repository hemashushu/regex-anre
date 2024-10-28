// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub const LEXER_PEEK_CHAR_MAX_COUNT: usize = 2;

use crate::{
    charposition::{CharWithPosition, CharsWithPositionIter},
    error::Error,
    location::Location,
    peekableiter::PeekableIter,
};

use super::token::{Comment, Token, TokenWithRange};

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
}

impl<'a> Lexer<'a> {
    fn lex(&mut self) -> Result<Vec<TokenWithRange>, Error> {
        let mut token_with_ranges = vec![];

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                ' ' | '\t' => {
                    self.next_char(); // consume whitespace
                }
                '\r' if self.peek_char_and_equals(1, '\n') => {
                    self.push_peek_position();

                    self.next_char(); // consume '\r'
                    self.next_char(); // consume '\n'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::NewLine,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '\n' => {
                    self.next_char(); // consume '\n'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::NewLine,
                        &self.last_position,
                        1,
                    ));
                }
                ',' => {
                    self.next_char(); // consume ','

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Comma,
                        &self.last_position,
                        1,
                    ));
                }
                '|' if self.peek_char_and_equals(1, '|') => {
                    self.push_peek_position();

                    self.next_char(); // consume '|'
                    self.next_char(); // consume '|'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::LogicOr,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '!' => {
                    self.next_char(); // consume '!'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Exclamation,
                        &self.last_position,
                        1,
                    ));
                }
                '.' if self.peek_char_and_equals(1, '.') => {
                    self.push_peek_position();

                    self.next_char(); // consume '.'
                    self.next_char(); // consume '.'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Interval,
                        &self.pop_saved_position(),
                        2,
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
                '[' => {
                    self.next_char(); // consume '['

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::LeftBracket,
                        &self.last_position,
                        1,
                    ));
                }
                ']' => {
                    self.next_char(); // consume ']'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::RightBracket,
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
                    ))
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
                    ))
                }
                '0'..='9' => {
                    // number
                    token_with_ranges.push(self.lex_number()?);
                }
                '"' => {
                    // string
                    token_with_ranges.push(self.lex_string()?);
                }
                '\'' => {
                    // char
                    token_with_ranges.push(self.lex_char()?);
                }
                '/' if self.peek_char_and_equals(1, '/') => {
                    // line comment
                    token_with_ranges.push(self.lex_line_comment()?);
                }
                '/' if self.peek_char_and_equals(1, '*') => {
                    // block comment
                    token_with_ranges.push(self.lex_block_comment()?);
                }
                'a'..='z' | 'A'..='Z' | '_' | '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' => {
                    // identifier (the key name of struct/object) or keyword
                    token_with_ranges.push(self.lex_identifier()?);
                }
                current_char => {
                    return Err(Error::MessageWithLocation(
                        format!("Unexpected char '{}'.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        Ok(token_with_ranges)
    }

    fn lex_identifier(&mut self) -> Result<TokenWithRange, Error> {
        // key_nameT  //
        // ^       ^__// to here
        // |__________// current char, validated
        //
        // current char = the character of `iter.upstream.peek(0)``
        // T = terminator chars || EOF

        let mut name_string = String::new();

        self.push_peek_position();

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
                ' ' | '\t' | '\r' | '\n' | ',' | '|' | '!' | '[' | ']' | '(' | ')' | '/' | '\''
                | '"' | '.' | '?' | '+' | '*' | '{' | '}' => {
                    // terminator chars
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

        let name_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        let token = match name_string.as_str() {
            "start" | "end" => Token::AnchorAssertion(name_string),
            "is_bound" | "is_not_bound" => Token::BoundaryAssertion(name_string),
            "char_space" | "char_not_space" | "char_word" | "char_not_word" | "char_digit"
            | "char_not_digit" => Token::PresetCharSet(name_string),
            "char_any" => Token::Special(name_string),
            _ => Token::Identifier(name_string),
        };

        Ok(TokenWithRange::new(token, name_range))
    }

    fn lex_number(&mut self) -> Result<TokenWithRange, Error> {
        // 123456T  //
        // ^     ^__// to here
        // |________// current char, validated
        //
        // T = terminator chars || EOF

        let mut num_string = String::new();

        self.push_peek_position();

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' => {
                    // valid digits for decimal number
                    num_string.push(*current_char);

                    self.next_char(); // consume digit
                }
                '_' => {
                    self.next_char(); // consume '_'
                }
                ' ' | '\t' | '\r' | '\n' | ',' | '|' | '!' | '[' | ']' | '(' | ')' | '/' | '\''
                | '"' | '.' | '?' | '+' | '*' | '{' | '}' => {
                    // terminator chars
                    break;
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        format!("Invalid char '{}' for decimal number.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
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

        let num_token = Token::Number(num);

        Ok(TokenWithRange::new(num_token, num_range))
    }

    fn lex_char(&mut self) -> Result<TokenWithRange, Error> {
        // 'a'?  //
        // ^  ^__// to here
        // |_____// current char, validated

        self.push_peek_position();

        self.next_char(); // consume "'"

        let character = match self.next_char() {
            Some(previous_previous_char) => {
                match previous_previous_char {
                    '\\' => {
                        // escape chars
                        match self.next_char() {
                            Some(previous_char) => {
                                match previous_char {
                                    '\\' => '\\',
                                    '\'' => '\'',
                                    '"' => {
                                        // double quote does not necessary to be escaped for char
                                        // however, it is still supported for consistency between chars and strings.
                                        '"'
                                    }
                                    't' => {
                                        // horizontal tabulation
                                        '\t'
                                    }
                                    'r' => {
                                        // carriage return (CR, ascii 13)
                                        '\r'
                                    }
                                    'n' => {
                                        // new line character (line feed, LF, ascii 10)
                                        '\n'
                                    }
                                    // '0' => {
                                    //     // null char
                                    //     '\0'
                                    // }
                                    'u' => {
                                        if self.peek_char_and_equals(0, '{') {
                                            // unicode code point, e.g. '\u{2d}', '\u{6587}'
                                            self.unescape_unicode()?
                                        } else {
                                            return Err(Error::MessageWithLocation(
                                                "Missing the brace for unicode escape sequence."
                                                    .to_owned(),
                                                self.last_position.move_position_forward(),
                                            ));
                                        }
                                    }
                                    _ => {
                                        return Err(Error::MessageWithLocation(
                                            format!("Unexpected escape char '{}'.", previous_char),
                                            // self.last_position,
                                            Location::from_position_and_length(
                                                &self.last_position.move_position_backward(),
                                                2,
                                            ),
                                        ));
                                    }
                                }
                            }
                            None => {
                                // `\` + EOF
                                return Err(Error::UnexpectedEndOfDocument(
                                    "Incomplete escape character sequence.".to_owned(),
                                ));
                            }
                        }
                    }
                    '\'' => {
                        // `''`
                        return Err(Error::MessageWithLocation(
                            "Empty char.".to_owned(),
                            Location::from_position_pair_with_end_included(
                                &self.pop_saved_position(),
                                &self.last_position,
                            ),
                        ));
                    }
                    _ => {
                        // ordinary char
                        previous_previous_char
                    }
                }
            }
            None => {
                // `'EOF`
                return Err(Error::UnexpectedEndOfDocument(
                    "Incomplete character.".to_owned(),
                ));
            }
        };

        // consume the right single quote
        match self.next_char() {
            Some('\'') => {
                // Ok
            }
            Some(_) => {
                // `'a?`
                return Err(Error::MessageWithLocation(
                    "Expected a closing single quote for char".to_owned(),
                    self.last_position,
                ));
            }
            None => {
                // `'aEOF`
                return Err(Error::UnexpectedEndOfDocument(
                    "Incomplete character.".to_owned(),
                ));
            }
        }

        let character_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );
        Ok(TokenWithRange::new(Token::Char(character), character_range))
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

    fn lex_string(&mut self) -> Result<TokenWithRange, Error> {
        // "abc"?  //
        // ^    ^__// to here
        // |_______// current char, validated

        self.push_peek_position();

        self.next_char(); // consume '"'

        let mut final_string = String::new();

        loop {
            match self.next_char() {
                Some(previous_previous_char) => {
                    match previous_previous_char {
                        '\\' => {
                            // escape chars
                            match self.next_char() {
                                Some(previous_char) => {
                                    match previous_char {
                                        '\\' => {
                                            final_string.push('\\');
                                        }
                                        '\'' => {
                                            // single quote does not necessary to be escaped for string
                                            // however, it is still supported for consistency between chars and strings.
                                            final_string.push('\'');
                                        }
                                        '"' => {
                                            final_string.push('"');
                                        }
                                        't' => {
                                            // horizontal tabulation
                                            final_string.push('\t');
                                        }
                                        'r' => {
                                            // carriage return (CR, ascii 13)
                                            final_string.push('\r');
                                        }
                                        'n' => {
                                            // new line character (line feed, LF, ascii 10)
                                            final_string.push('\n');
                                        }
                                        '0' => {
                                            // null char
                                            final_string.push('\0');
                                        }
                                        'u' => {
                                            if self.peek_char_and_equals(0, '{') {
                                                // unicode code point, e.g. '\u{2d}', '\u{6587}'
                                                let ch = self.unescape_unicode()?;
                                                final_string.push(ch);
                                            } else {
                                                return Err(Error::MessageWithLocation(
                                                    "Missing the brace for unicode escape sequence.".to_owned(),
                                                    self.last_position.move_position_forward()
                                                ));
                                            }
                                        }
                                        _ => {
                                            return Err(Error::MessageWithLocation(
                                                format!(
                                                    "Unsupported escape char '{}'.",
                                                    previous_char
                                                ),
                                                Location::from_position_and_length(
                                                    &self.last_position.move_position_backward(),
                                                    2,
                                                ),
                                            ));
                                        }
                                    }
                                }
                                None => {
                                    // `\` + EOF
                                    return Err(Error::UnexpectedEndOfDocument(
                                        "Incomplete character escape sequence.".to_owned(),
                                    ));
                                }
                            }
                        }
                        '"' => {
                            // end of the string
                            break;
                        }
                        _ => {
                            // ordinary char
                            final_string.push(previous_previous_char);
                        }
                    }
                }
                None => {
                    // `"...EOF`
                    return Err(Error::UnexpectedEndOfDocument(
                        "Incomplete string.".to_owned(),
                    ));
                }
            }
        }

        let final_string_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(
            Token::String(final_string),
            final_string_range,
        ))
    }

    fn lex_line_comment(&mut self) -> Result<TokenWithRange, Error> {
        // xx...[\r]\n?  //
        // ^^         ^__// to here ('?' = any char or EOF)
        // ||____________// validated
        // |_____________// current char, validated
        //
        // x = '/'

        self.push_peek_position();

        self.next_char(); // consume the 1st '/'
        self.next_char(); // consume the 2nd '/'

        let mut comment_string = String::new();

        while let Some(current_char) = self.peek_char(0) {
            // ignore all chars except '\n' or '\r\n'
            // note that the "line comment token" does not include the trailing new line chars (\n or \r\n),

            match current_char {
                '\n' => {
                    break;
                }
                '\r' if self.peek_char_and_equals(0, '\n') => {
                    break;
                }
                _ => {
                    comment_string.push(*current_char);

                    self.next_char(); // consume char
                }
            }
        }

        let comment_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(
            Token::Comment(Comment::Line(comment_string)),
            comment_range,
        ))
    }

    fn lex_block_comment(&mut self) -> Result<TokenWithRange, Error> {
        // /*...*/?  //
        // ^^     ^__// to here
        // ||________// validated
        // |_________// current char, validated

        self.push_peek_position();

        self.next_char(); // consume '/'
        self.next_char(); // consume '*'

        let mut comment_string = String::new();
        let mut depth = 1; // nested depth

        loop {
            match self.next_char() {
                Some(previous_char) => {
                    match previous_char {
                        '/' if self.peek_char_and_equals(0, '*') => {
                            // nested block comment
                            comment_string.push_str("/*");

                            self.next_char(); // consume '*'

                            // increase depth
                            depth += 1;
                        }
                        '*' if self.peek_char_and_equals(0, '/') => {
                            self.next_char(); // consume '/'

                            // decrease depth
                            depth -= 1;

                            // check pairs
                            if depth == 0 {
                                break;
                            } else {
                                comment_string.push_str("*/");
                            }
                        }
                        _ => {
                            // ignore all chars except "/*" and "*/"
                            // note that line comments within block comments are ignored also.
                            comment_string.push(previous_char);
                        }
                    }
                }
                None => {
                    let msg = if depth > 1 {
                        "Incomplete nested block comment.".to_owned()
                    } else {
                        "Incomplete block comment.".to_owned()
                    };

                    return Err(Error::UnexpectedEndOfDocument(msg));
                }
            }
        }

        let comment_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(
            Token::Comment(Comment::Block(comment_string)),
            comment_range,
        ))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        anre::token::{Comment, Token, TokenWithRange},
        error::Error,
        location::Location,
    };

    use super::lex_from_str;

    impl Token {
        pub fn new_identifier(s: &str) -> Self {
            Token::Identifier(s.to_owned())
        }

        pub fn new_anchor_assertion(s: &str) -> Self {
            Token::AnchorAssertion(s.to_owned())
        }

        pub fn new_boundary_assertion(s: &str) -> Self {
            Token::BoundaryAssertion(s.to_owned())
        }

        pub fn new_special(s: &str) -> Self {
            Token::Special(s.to_owned())
        }

        pub fn new_preset_charset(s: &str) -> Self {
            Token::PresetCharSet(s.to_owned())
        }

        pub fn new_string(s: &str) -> Self {
            Token::String(s.to_owned())
        }
    }

    fn lex_from_str_without_location(s: &str) -> Result<Vec<Token>, Error> {
        let tokens = lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_lex_whitespaces() {
        assert_eq!(lex_from_str_without_location("  ").unwrap(), vec![]);

        assert_eq!(
            lex_from_str_without_location("()").unwrap(),
            vec![Token::LeftParen, Token::RightParen]
        );

        assert_eq!(
            lex_from_str_without_location("(  )").unwrap(),
            vec![Token::LeftParen, Token::RightParen]
        );

        assert_eq!(
            lex_from_str_without_location("(\t\r\n\n\n)").unwrap(),
            vec![
                Token::LeftParen,
                Token::NewLine,
                Token::NewLine,
                Token::NewLine,
                Token::RightParen,
            ]
        );

        // location

        assert_eq!(lex_from_str("  ").unwrap(), vec![]);

        assert_eq!(
            lex_from_str("()").unwrap(),
            vec![
                TokenWithRange::new(Token::LeftParen, Location::new_range(0, 0, 0, 0, 1)),
                TokenWithRange::new(Token::RightParen, Location::new_range(0, 1, 0, 1, 1)),
            ]
        );

        assert_eq!(
            lex_from_str("(  )").unwrap(),
            vec![
                TokenWithRange::new(Token::LeftParen, Location::new_range(0, 0, 0, 0, 1)),
                TokenWithRange::new(Token::RightParen, Location::new_range(0, 3, 0, 3, 1)),
            ]
        );

        // "(\t\r\n\n\n)"
        //  _--____--__-
        //  0  2   4 5 6    // index
        //  0  0   1 2 3    // line
        //  0  2   0 0 1    // column
        //  1  2   1 1 1    // length

        assert_eq!(
            lex_from_str("(\t\r\n\n\n)").unwrap(),
            vec![
                TokenWithRange::new(Token::LeftParen, Location::new_range(0, 0, 0, 0, 1)),
                TokenWithRange::new(Token::NewLine, Location::new_range(0, 2, 0, 2, 2,)),
                TokenWithRange::new(Token::NewLine, Location::new_range(0, 4, 1, 0, 1,)),
                TokenWithRange::new(Token::NewLine, Location::new_range(0, 5, 2, 0, 1,)),
                TokenWithRange::new(Token::RightParen, Location::new_range(0, 6, 3, 0, 1)),
            ]
        );
    }

    #[test]
    fn test_lex_punctuations() {
        assert_eq!(
            lex_from_str_without_location(",!...||[]()???++?**?{}").unwrap(),
            vec![
                Token::Comma,
                Token::Exclamation,
                Token::Interval,
                Token::Dot,
                Token::LogicOr,
                Token::LeftBracket,
                Token::RightBracket,
                Token::LeftParen,
                Token::RightParen,
                Token::QuestionLazy,
                Token::Question,
                Token::Plus,
                Token::PlusLazy,
                Token::Asterisk,
                Token::AsteriskLazy,
                Token::LeftBrace,
                Token::RightBrace
            ]
        );

        // location

        assert_eq!(
            lex_from_str("???++?**?").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::QuestionLazy,
                    &Location::new_position(0, 0, 0, 0),
                    2
                ),
                TokenWithRange::from_position_and_length(
                    Token::Question,
                    &Location::new_position(0, 2, 0, 2),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Plus,
                    &Location::new_position(0, 3, 0, 3),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::PlusLazy,
                    &Location::new_position(0, 4, 0, 4),
                    2
                ),
                TokenWithRange::from_position_and_length(
                    Token::Asterisk,
                    &Location::new_position(0, 6, 0, 6),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::AsteriskLazy,
                    &Location::new_position(0, 7, 0, 7),
                    2
                ),
            ]
        );
    }

    #[test]
    fn test_lex_identifier() {
        assert_eq!(
            lex_from_str_without_location("name").unwrap(),
            vec![Token::new_identifier("name")]
        );

        assert_eq!(
            lex_from_str_without_location("(name)").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_identifier("name"),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str_without_location("( a )").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_identifier("a"),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str_without_location("a__b__c").unwrap(),
            vec![Token::new_identifier("a__b__c")]
        );

        assert_eq!(
            lex_from_str_without_location("foo bar").unwrap(),
            vec![Token::new_identifier("foo"), Token::new_identifier("bar")]
        );

        assert_eq!(
            lex_from_str_without_location("foo.bar").unwrap(),
            vec![
                Token::new_identifier("foo"),
                Token::Dot,
                Token::new_identifier("bar")
            ]
        );

        assert_eq!(
            lex_from_str_without_location("αβγ 文字 🍞🥛").unwrap(),
            vec![
                Token::new_identifier("αβγ"),
                Token::new_identifier("文字"),
                Token::new_identifier("🍞🥛"),
            ]
        );

        // location

        assert_eq!(
            lex_from_str("hello ASON").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_identifier("hello"),
                    &Location::new_position(0, 0, 0, 0),
                    5
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_identifier("ASON"),
                    &Location::new_position(0, 6, 0, 6),
                    4
                )
            ]
        );

        // err: invalid char
        assert!(matches!(
            lex_from_str_without_location("abc&xyz"),
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

    #[test]
    fn test_lex_preset_charset() {
        assert_eq!(
            lex_from_str_without_location(
                "char_space char_not_space char_word char_not_word char_digit char_not_digit"
            )
            .unwrap(),
            vec![
                Token::new_preset_charset("char_space"),
                Token::new_preset_charset("char_not_space"),
                Token::new_preset_charset("char_word"),
                Token::new_preset_charset("char_not_word"),
                Token::new_preset_charset("char_digit"),
                Token::new_preset_charset("char_not_digit"),
            ]
        );

        // location

        assert_eq!(
            lex_from_str("char_space char_not_digit").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_preset_charset("char_space"),
                    &Location::new_position(0, 0, 0, 0),
                    10
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_preset_charset("char_not_digit"),
                    &Location::new_position(0, 11, 0, 11),
                    14
                )
            ]
        );
    }

    #[test]
    fn test_lex_other_identifier() {
        assert_eq!(
            lex_from_str_without_location("char_any start end is_bound is_not_bound").unwrap(),
            vec![
                Token::new_special("char_any"),
                Token::new_anchor_assertion("start"),
                Token::new_anchor_assertion("end"),
                Token::new_boundary_assertion("is_bound"),
                Token::new_boundary_assertion("is_not_bound"),
            ]
        );

        // location

        assert_eq!(
            lex_from_str("start end").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_anchor_assertion("start"),
                    &Location::new_position(0, 0, 0, 0),
                    5
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_anchor_assertion("end"),
                    &Location::new_position(0, 6, 0, 6),
                    3
                )
            ]
        );
    }

    #[test]
    fn test_lex_number() {
        assert_eq!(
            lex_from_str_without_location("223").unwrap(),
            vec![Token::Number(223),]
        );

        assert_eq!(
            lex_from_str_without_location("211").unwrap(),
            vec![Token::Number(211)]
        );

        assert_eq!(
            lex_from_str_without_location("223_211").unwrap(),
            vec![Token::Number(223_211)]
        );

        assert_eq!(
            lex_from_str_without_location("223 211").unwrap(),
            vec![Token::Number(223), Token::Number(211),]
        );

        // location

        assert_eq!(
            lex_from_str("223 211").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Number(223),
                    &Location::new_position(0, 0, 0, 0,),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Number(211),
                    &Location::new_position(0, 4, 0, 4,),
                    3
                ),
            ]
        );

        // err: invalid char for decimal number
        assert!(matches!(
            lex_from_str_without_location("12x34"),
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
    }

    #[test]
    fn test_lex_char() {
        assert_eq!(
            lex_from_str_without_location("'a'").unwrap(),
            vec![Token::Char('a')]
        );

        assert_eq!(
            lex_from_str_without_location("('a')").unwrap(),
            vec![Token::LeftParen, Token::Char('a'), Token::RightParen]
        );

        assert_eq!(
            lex_from_str_without_location("'a' 'z'").unwrap(),
            vec![Token::Char('a'), Token::Char('z')]
        );

        // CJK
        assert_eq!(
            lex_from_str_without_location("'文'").unwrap(),
            vec![Token::Char('文')]
        );

        // emoji
        assert_eq!(
            lex_from_str_without_location("'😊'").unwrap(),
            vec![Token::Char('😊')]
        );

        // escape char `\\`
        assert_eq!(
            lex_from_str_without_location("'\\\\'").unwrap(),
            vec![Token::Char('\\')]
        );

        // escape char `\'`
        assert_eq!(
            lex_from_str_without_location("'\\\''").unwrap(),
            vec![Token::Char('\'')]
        );

        // escape char `"`
        assert_eq!(
            lex_from_str_without_location("'\\\"'").unwrap(),
            vec![Token::Char('"')]
        );

        // escape char `\t`
        assert_eq!(
            lex_from_str_without_location("'\\t'").unwrap(),
            vec![Token::Char('\t')]
        );

        // escape char `\r`
        assert_eq!(
            lex_from_str_without_location("'\\r'").unwrap(),
            vec![Token::Char('\r')]
        );

        // escape char `\n`
        assert_eq!(
            lex_from_str_without_location("'\\n'").unwrap(),
            vec![Token::Char('\n')]
        );

        // // escape char `\0`
        // assert_eq!(
        //     lex_from_str_without_location("'\\0'").unwrap(),
        //     vec![Token::Char('\0')]
        // );

        // escape char, unicode
        assert_eq!(
            lex_from_str_without_location("'\\u{2d}'").unwrap(),
            vec![Token::Char('-')]
        );

        // escape char, unicode
        assert_eq!(
            lex_from_str_without_location("'\\u{6587}'").unwrap(),
            vec![Token::Char('文')]
        );

        // location

        assert_eq!(
            lex_from_str("'a' '文'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('a'),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('文'),
                    &Location::new_position(0, 4, 0, 4),
                    3
                )
            ]
        );

        assert_eq!(
            lex_from_str("'\\t'").unwrap(),
            vec![TokenWithRange::from_position_and_length(
                Token::Char('\t'),
                &Location::new_position(0, 0, 0, 0),
                4
            )]
        );

        assert_eq!(
            lex_from_str("'\\u{6587}'").unwrap(),
            vec![TokenWithRange::from_position_and_length(
                Token::Char('文'),
                &Location::new_position(0, 0, 0, 0),
                10
            )]
        );

        // err: empty char
        assert!(matches!(
            lex_from_str_without_location("''"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 2
                }
            ))
        ));

        // err: empty char, missing the char
        assert!(matches!(
            lex_from_str_without_location("'"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete char, missing the right quote, encounter EOF
        assert!(matches!(
            lex_from_str_without_location("'a"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: invalid char, expect the right quote, encounter another char
        assert!(matches!(
            lex_from_str_without_location("'ab"),
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

        // err: invalid char, expect the right quote, encounter another char
        assert!(matches!(
            lex_from_str_without_location("'ab'"),
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

        // err: unsupported escape char \v
        assert!(matches!(
            lex_from_str_without_location(r#"'\v'"#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 2,
                }
            ))
        ));

        // err: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str_without_location(r#"'\x33'"#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 2
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

    #[test]
    fn test_lex_string() {
        assert_eq!(
            lex_from_str_without_location(r#""abc""#).unwrap(),
            vec![Token::new_string("abc")]
        );

        assert_eq!(
            lex_from_str_without_location(r#"("abc")"#).unwrap(),
            vec![
                Token::LeftParen,
                Token::new_string("abc"),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str_without_location(r#""abc" "xyz""#).unwrap(),
            vec![Token::new_string("abc"), Token::new_string("xyz")]
        );

        assert_eq!(
            lex_from_str_without_location("\"abc\"\n\n\"xyz\"").unwrap(),
            vec![
                Token::new_string("abc"),
                Token::NewLine,
                Token::NewLine,
                Token::new_string("xyz"),
            ]
        );

        // unicode
        assert_eq!(
            lex_from_str_without_location(
                r#"
                "abc文字😊"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("abc文字😊"),
                Token::NewLine,
            ]
        );

        // empty string
        assert_eq!(
            lex_from_str_without_location("\"\"").unwrap(),
            vec![Token::new_string("")]
        );

        // escape chars
        assert_eq!(
            lex_from_str_without_location(
                r#"
                "\\\'\"\t\r\n\0\u{2d}\u{6587}"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("\\\'\"\t\r\n\0-文"),
                Token::NewLine,
            ]
        );

        // location
        // "abc" "文字😊"
        // 01234567 8 9 0

        assert_eq!(
            lex_from_str(r#""abc" "文字😊""#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_string("abc"),
                    &Location::new_position(0, 0, 0, 0),
                    5
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_string("文字😊"),
                    &Location::new_position(0, 6, 0, 6),
                    5
                ),
            ]
        );

        // err: incomplete string, missing the closed quote
        assert!(matches!(
            lex_from_str_without_location("\"abc"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the closed quote, ends with \n
        assert!(matches!(
            lex_from_str_without_location("\"abc\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the closed quote, ends with whitespaces/other chars
        assert!(matches!(
            lex_from_str_without_location("\"abc\n   "),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: unsupported escape char \v
        assert!(matches!(
            lex_from_str_without_location(r#""abc\vxyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 2
                }
            ))
        ));

        // err: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str_without_location(r#""abc\x33xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 2
                }
            ))
        ));

        // err: empty unicode escape string
        // "abc\u{}"
        // 012345678    // index
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 2
                }
            ))
        ));

        // err: invalid unicode code point, too much digits
        // "abc\u{1000111}xyz"
        // 0123456789023456789    // index
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{1000111}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 8
                }
            ))
        ));

        // err: invalid unicode code point, code point out of range
        // "abc\u{123456}xyz"
        // 012345678901234567
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{123456}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 8
                }
            ))
        ));

        // err: invalid char in the unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{12mn}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 9,
                    line: 0,
                    column: 9,
                    length: 0
                }
            ))
        ));

        // err: missing the right brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{1234""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 11,
                    line: 0,
                    column: 11,
                    length: 0
                }
            ))
        ));

        // err: incomplete unicode escape sequence, encounter EOF
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{1234"#),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u1234}xyz""#),
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
    }

    #[test]
    fn test_lex_line_comment() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 //11
                13 17// 19 23
                //  29
                31//    37
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(7),
                Token::Comment(Comment::Line("11".to_owned())),
                Token::NewLine,
                Token::Number(13),
                Token::Number(17),
                Token::Comment(Comment::Line(" 19 23".to_owned())),
                Token::NewLine,
                Token::Comment(Comment::Line("  29".to_owned())),
                Token::NewLine,
                Token::Number(31),
                Token::Comment(Comment::Line("    37".to_owned())),
                Token::NewLine,
            ]
        );

        // location

        assert_eq!(
            lex_from_str("foo // bar").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Identifier("foo".to_owned()),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Line(" bar".to_owned())),
                    &Location::new_position(0, 4, 0, 4),
                    6
                ),
            ]
        );

        assert_eq!(
            lex_from_str("abc // def\n// xyz\n").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Identifier("abc".to_owned()),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Line(" def".to_owned())),
                    &Location::new_position(0, 4, 0, 4),
                    6
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 10, 0, 10),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Line(" xyz".to_owned())),
                    &Location::new_position(0, 11, 1, 0),
                    6
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 17, 1, 6),
                    1
                ),
            ]
        );
    }

    #[test]
    fn test_lex_block_comment() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 13 */ 17
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(7),
                Token::Comment(Comment::Block(" 11 13 ".to_owned())),
                Token::Number(17),
                Token::NewLine,
            ]
        );

        // nested block comment
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 /* 13 */ 17 */ 19
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(7),
                Token::Comment(Comment::Block(" 11 /* 13 */ 17 ".to_owned())),
                Token::Number(19),
                Token::NewLine,
            ]
        );

        // line comment chars "//" within the block comment
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 // 13 17 */ 19
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(7),
                Token::Comment(Comment::Block(" 11 // 13 17 ".to_owned())),
                Token::Number(19),
                Token::NewLine,
            ]
        );

        // location

        assert_eq!(
            lex_from_str("foo /* hello */ bar").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Identifier("foo".to_owned()),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Block(" hello ".to_owned())),
                    &Location::new_position(0, 4, 0, 4),
                    11
                ),
                TokenWithRange::from_position_and_length(
                    Token::Identifier("bar".to_owned()),
                    &Location::new_position(0, 16, 0, 16),
                    3
                ),
            ]
        );

        assert_eq!(
            lex_from_str("/* abc\nxyz */ /* hello */").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Block(" abc\nxyz ".to_owned())),
                    &Location::new_position(0, 0, 0, 0),
                    13
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Block(" hello ".to_owned())),
                    &Location::new_position(0, 14, 1, 7),
                    11
                ),
            ]
        );

        // err: incomplete, missing "*/"
        assert!(matches!(
            lex_from_str_without_location("7 /* 11"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete, missing "*/", ends with \n
        assert!(matches!(
            lex_from_str_without_location("7 /* 11\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete, unpaired, missing "*/"
        assert!(matches!(
            lex_from_str_without_location("a /* b /* c */"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete, unpaired, missing "*/", ends with \n
        assert!(matches!(
            lex_from_str_without_location("a /* b /* c */\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_lex_multiple_tokens() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                ('a', "def", "xyz".repeat(3)).one_or_more()
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::Char('a'),
                Token::Comma,
                Token::new_string("def"),
                Token::Comma,
                Token::new_string("xyz"),
                Token::Dot,
                Token::new_identifier("repeat"),
                Token::LeftParen,
                Token::Number(3),
                Token::RightParen,
                Token::RightParen,
                Token::Dot,
                Token::new_identifier("one_or_more"),
                Token::LeftParen,
                Token::RightParen,
                Token::NewLine
            ]
        );

        assert_eq!(
            lex_from_str_without_location(
                r#"
                'a'?
                'b'+
                'c'*
                'd'{1,2}
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Char('a'),
                Token::Question,
                Token::NewLine,
                Token::Char('b'),
                Token::Plus,
                Token::NewLine,
                Token::Char('c'),
                Token::Asterisk,
                Token::NewLine,
                Token::Char('d'),
                Token::LeftBrace,
                Token::Number(1),
                Token::Comma,
                Token::Number(2),
                Token::RightBrace,
                Token::NewLine
            ]
        );

        assert_eq!(
            lex_from_str_without_location(
                r#"
                one_or_more([
                    'a'..'f'    // comment 1
                    '0'..'9'    // comment 2
                    '_'
                ])
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_identifier("one_or_more"),
                Token::LeftParen,
                Token::LeftBracket,
                Token::NewLine,
                Token::Char('a'),
                Token::Interval,
                Token::Char('f'),
                Token::Comment(Comment::Line(" comment 1".to_owned())),
                Token::NewLine,
                Token::Char('0'),
                Token::Interval,
                Token::Char('9'),
                Token::Comment(Comment::Line(" comment 2".to_owned())),
                Token::NewLine,
                Token::Char('_'),
                Token::NewLine,
                Token::RightBracket,
                Token::RightParen,
                Token::NewLine
            ]
        );
    }
}
