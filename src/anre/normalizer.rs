// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{location::Location, peekableiter::PeekableIter};

use super::token::{Token, TokenWithRange};

pub fn normalize(tokens: Vec<TokenWithRange>) -> Vec<TokenWithRange> {
    // combine multiple continuous newlines into one newline.
    // rules:
    //   + blanks => blank
    //   + comma + blank(s) => comma
    //   + blank(s) + comma => comma
    //   + blank(s) + comma + blank(s) => comma
    //
    // because the comments have been removed, the following conclusions
    // can be inferred:
    //   + comma + comment(s) + comma => comma + comma
    //   + blank(s) + comment(s) + blank(s) => blank

    let mut token_iter = tokens.into_iter();
    let mut peekable_token_iter = PeekableIter::new(&mut token_iter, 1);
    let mut normalized_tokens: Vec<TokenWithRange> = vec![];

    while let Some(token_with_range) = peekable_token_iter.next() {
        let TokenWithRange {
            token,
            range: current_range,
        } = &token_with_range;

        let mut start_range = *current_range;
        let mut end_range = start_range;

        let compact_token_with_range = match token {
            Token::NewLine => {
                // consume continuous newlines
                while let Some(TokenWithRange {
                    token: Token::NewLine,
                    range: current_range,
                }) = peekable_token_iter.peek(0)
                {
                    end_range = *current_range;
                    peekable_token_iter.next();
                }

                // found ','
                if let Some(TokenWithRange {
                    token: Token::Comma,
                    range: current_range,
                }) = peekable_token_iter.peek(0)
                {
                    // consume comma
                    start_range = *current_range;
                    end_range = start_range;
                    peekable_token_iter.next();

                    // consume trailing continuous newlines
                    while let Some(TokenWithRange {
                        token: Token::NewLine,
                        range: _,
                    }) = peekable_token_iter.peek(0)
                    {
                        peekable_token_iter.next();
                    }

                    TokenWithRange::new(
                        Token::Comma,
                        Location::from_range_pair(&start_range, &end_range),
                    )
                } else {
                    TokenWithRange::new(
                        Token::NewLine,
                        Location::from_range_pair(&start_range, &end_range),
                    )
                }
            }
            Token::Comma => {
                // consume trailing continuous newlines
                while let Some(TokenWithRange {
                    token: Token::NewLine,
                    range: _,
                }) = peekable_token_iter.peek(0)
                {
                    peekable_token_iter.next();
                }

                TokenWithRange::new(
                    Token::Comma,
                    Location::from_range_pair(&start_range, &end_range),
                )
            }
            _ => token_with_range,
        };

        normalized_tokens.push(compact_token_with_range);
    }

    // remove document leading and tailing newlines.
    if let Some(TokenWithRange {
        token: Token::NewLine,
        ..
    }) = normalized_tokens.first()
    {
        normalized_tokens.remove(0);
    }

    if let Some(TokenWithRange {
        token: Token::NewLine,
        ..
    }) = normalized_tokens.last()
    {
        normalized_tokens.pop();
    }

    normalized_tokens
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        anre::{
            commentremover::clean,
            lexer::lex_from_str,
            token::{Token, TokenWithRange},
        },
        AnreError,
        location::Location,
    };

    use super::normalize;

    fn clean_and_normalize_lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, AnreError> {
        let tokens = lex_from_str(s)?;
        let clean_tokens = clean(tokens);
        let normalized_tokens = normalize(clean_tokens);
        Ok(normalized_tokens)
    }

    fn clean_and_normalize_lex_from_str_without_location(s: &str) -> Result<Vec<Token>, AnreError> {
        let tokens = clean_and_normalize_lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_normalize_blanks_and_commas() {
        assert_eq!(
            // test items:
            //
            // unchaged:
            // - comma => comma
            //
            // normalized:
            // - comma + blank(s) => comma
            // - blank(s) + comma => comma
            // - blank(s) + comma + blank(s) => comma
            //
            // inferred:
            // - comma + comment(s) + comma => comma + comma
            // - blank(s) + comment(s) + blank(s) => blank
            //
            // normalization:
            // - blanks => blank
            clean_and_normalize_lex_from_str_without_location(
                r#"
                    ('1','2',

                    '3'

                    ,'4'

                    ,

                    '5'
                    ,
                    // comment between commas
                    ,
                    '6'

                    // comment between blank lines

                    '7'
                    '8'
                    )

                    "#
            )
            .unwrap(),
            vec![
                Token::LeftParen,
                Token::Char('1'),
                Token::Comma,
                Token::Char('2'),
                Token::Comma,
                Token::Char('3'),
                Token::Comma,
                Token::Char('4'),
                Token::Comma,
                Token::Char('5'),
                Token::Comma,
                Token::Comma,
                Token::Char('6'),
                Token::NewLine,
                Token::Char('7'),
                Token::NewLine,
                Token::Char('8'),
                Token::NewLine,
                Token::RightParen,
            ]
        );

        // location

        // blanks -> blank
        assert_eq!(
            clean_and_normalize_lex_from_str("'1'\n \n  \n'2'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    6
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('2'),
                    &Location::new_position(/*0,*/ 9, 3, 0),
                    3
                ),
            ]
        );

        // comma + blanks -> comma
        assert_eq!(
            clean_and_normalize_lex_from_str(",\n\n\n'1'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(/*0,*/ 4, 3, 0),
                    3
                ),
            ]
        );

        // blanks + comma -> comma
        assert_eq!(
            clean_and_normalize_lex_from_str("'1'\n\n\n,").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(/*0,*/ 6, 3, 0),
                    1
                ),
            ]
        );

        // blanks + comma + blanks -> comma
        assert_eq!(
            clean_and_normalize_lex_from_str("'1'\n\n,\n\n'2'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(/*0,*/ 5, 2, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('2'),
                    &Location::new_position(/*0,*/ 8, 4, 0),
                    3
                ),
            ]
        );

        // comma + comment + comma -> comma + comma
        assert_eq!(
            clean_and_normalize_lex_from_str(",//abc\n,").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(/*0,*/ 7, 1, 0),
                    1
                ),
            ]
        );

        // blanks + comment + blanks -> blank
        assert_eq!(
            clean_and_normalize_lex_from_str("'1'\n\n//abc\n\n'2'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(/*0,*/ 3, 0, 3),
                    9
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('2'),
                    &Location::new_position(/*0,*/ 12, 4, 0),
                    3
                ),
            ]
        );
    }

    #[test]
    fn test_normalize_trim_blanks() {
        assert_eq!(
            clean_and_normalize_lex_from_str_without_location(
                r#"

                '1'

                '2'

                "#
            )
            .unwrap(),
            vec![Token::Char('1'), Token::NewLine, Token::Char('2'),]
        );
    }
}
