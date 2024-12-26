// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use super::token::{Token, TokenWithRange};

pub fn clean(tokens: Vec<TokenWithRange>) -> Vec<TokenWithRange> {
    // remove all comments.
    let clean_tokens: Vec<TokenWithRange> = tokens
        .into_iter()
        .filter(|e| {
            !matches!(
                e,
                TokenWithRange {
                    token: Token::Comment(_),
                    ..
                }
            )
        })
        .collect();

    clean_tokens
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        anre::{
            lexer::lex_from_str,
            token::{Token, TokenWithRange},
        },
        AnreError,
        location::Location,
    };

    use super::clean;

    fn clean_and_lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, AnreError> {
        let tokens = lex_from_str(s)?;
        let clean_tokens = clean(tokens);
        Ok(clean_tokens)
    }

    fn clean_and_lex_from_str_without_location(s: &str) -> Result<Vec<Token>, AnreError> {
        let tokens = clean_and_lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_clean_comments() {
        assert_eq!(
            clean_and_lex_from_str_without_location(
                r#"'1' // line comment 1
                // line comment 2
                '3' /* block comment 1 */
                /*
                block comment 2
                */
                '7'
                "#
            )
            .unwrap(),
            vec![
                Token::Char('1'),
                Token::NewLine,
                Token::NewLine,
                Token::Char('3'),
                Token::NewLine,
                Token::NewLine,
                Token::Char('7'),
                Token::NewLine,
            ]
        );

        assert_eq!(
            clean_and_lex_from_str(r#"'1' /* foo */ '3'"#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Char('1'),
                    &Location::new_position(/*0,*/ 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Char('3'),
                    &Location::new_position(/*0,*/ 14, 0, 14),
                    3
                ),
            ]
        );
    }
}
