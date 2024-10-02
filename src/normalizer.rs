// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    location::Location,
    peekableiter::PeekableIter,
    token::{Token, TokenWithRange},
};

pub fn normalize(iter1: &mut dyn Iterator<Item = TokenWithRange>) -> Vec<TokenWithRange> {
    // step 1: remove all comments.
    let mut stage1: Vec<TokenWithRange> = vec![];
    while let Some(tr) = iter1.next() {
        match tr {
            TokenWithRange {
                token: Token::Comment(_),
                ..
            } => {
                // consume comments
            }
            _ => {
                stage1.push(tr);
            }
        }
    }

    // step 2: combine multiple continuous newlines into one newline.
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

    let mut prev_iter2 = stage1.into_iter();
    let mut iter2 = PeekableIter::new(&mut prev_iter2, 1);
    let mut stage2: Vec<TokenWithRange> = vec![];

    while let Some(tr) = iter2.next() {
        let TokenWithRange {
            token,
            range: current_range,
        } = &tr;

        let mut start_range = *current_range;
        let mut end_range = start_range;

        let ntr = match token {
            Token::NewLine => {
                // consume continuous newlines
                while let Some(TokenWithRange {
                    token: Token::NewLine,
                    range: current_range,
                }) = iter2.peek(0)
                {
                    end_range = *current_range;
                    iter2.next();
                }

                // found ','
                if let Some(TokenWithRange {
                    token: Token::Comma,
                    range: current_range,
                }) = iter2.peek(0)
                {
                    // consume comma
                    start_range = *current_range;
                    end_range = start_range;
                    iter2.next();

                    // consume trailing continuous newlines
                    while let Some(TokenWithRange {
                        token: Token::NewLine,
                        range: _,
                    }) = iter2.peek(0)
                    {
                        iter2.next();
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
                }) = iter2.peek(0)
                {
                    iter2.next();
                }

                TokenWithRange::new(
                    Token::Comma,
                    Location::from_range_pair(&start_range, &end_range),
                )
            }
            _ => tr,
        };

        stage2.push(ntr);
    }

    // step 3: remove document leading and tailing newlines.
    if let Some(TokenWithRange {
        token: Token::NewLine,
        ..
    }) = stage2.first()
    {
        stage2.remove(0);
    }

    if let Some(TokenWithRange {
        token: Token::NewLine,
        ..
    }) = stage2.last()
    {
        stage2.pop();
    }

    stage2
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        charposition::CharsWithPositionIter,
        error::Error,
        lexer::Lexer,
        location::Location,
        peekableiter::PeekableIter,
        token::{Token, TokenWithRange},
    };

    use super::normalize;

    fn lex_str_to_vec_with_range(s: &str) -> Result<Vec<TokenWithRange>, Error> {
        let mut chars = s.chars();
        let mut char_position_iter = CharsWithPositionIter::new(0, &mut chars);
        let mut peekable_char_position_iter = PeekableIter::new(&mut char_position_iter, 3);
        let mut lexer = Lexer::new(&mut peekable_char_position_iter);
        let tokens = lexer.lex()?;
        let mut token_iter = tokens.into_iter();
        let normalized_tokens = normalize(&mut token_iter);
        Ok(normalized_tokens)
    }

    fn lex_str_to_vec(s: &str) -> Result<Vec<Token>, Error> {
        let tokens = lex_str_to_vec_with_range(s)?
            .iter()
            .map(|e| e.token.to_owned())
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_clear_comments() {
        assert_eq!(
            lex_str_to_vec(
                r#"'1' // line comment 1
                // line comment 2
                '2' /* block comment 1 */
                /*
                block comment 2
                */
                '3'
                "#
            )
            .unwrap(),
            vec![
                Token::Char('1'),
                Token::NewLine,
                Token::Char('2'),
                Token::NewLine,
                Token::Char('3'),
            ]
        );
    }

    #[test]
    fn test_normalize_blanks_commas_and_comments() {
        assert_eq!(
            // test items:
            //
            // unchaged:
            // - comma => comma
            //
            // normalization:
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
            lex_str_to_vec(
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
            lex_str_to_vec_with_range("'1'\n \n  \n'2'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 3, 0, 3),
                    6
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('2'),
                    &Location::new_position(0, 9, 3, 0),
                    3
                ),
            ]
        );

        // comma + blanks -> comma
        assert_eq!(
            lex_str_to_vec_with_range(",\n\n\n'1'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(0, 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(0, 4, 3, 0),
                    3
                ),
            ]
        );

        // blanks + comma -> comma
        assert_eq!(
            lex_str_to_vec_with_range("'1'\n\n\n,").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(0, 6, 3, 0),
                    1
                ),
            ]
        );

        // blanks + comma + blanks -> comma
        assert_eq!(
            lex_str_to_vec_with_range("'1'\n\n,\n\n'2'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(0, 5, 2, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('2'),
                    &Location::new_position(0, 8, 4, 0),
                    3
                ),
            ]
        );

        // comma + comment + comma -> comma + comma
        assert_eq!(
            lex_str_to_vec_with_range(",//abc\n,").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(0, 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(0, 7, 1, 0),
                    1
                ),
            ]
        );

        // blanks + comment + blanks -> blank
        assert_eq!(
            lex_str_to_vec_with_range("'1'\n\n//abc\n\n'2'").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 3, 0, 3),
                    9
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('2'),
                    &Location::new_position(0, 12, 4, 0),
                    3
                ),
            ]
        );
    }

    #[test]
    fn test_trim_blanks() {
        assert_eq!(
            lex_str_to_vec(
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
