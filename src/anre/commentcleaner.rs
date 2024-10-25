// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::peekableiter::PeekableIter;

use super::token::{Token, TokenWithRange};

pub fn clean(tokens: Vec<TokenWithRange>) -> Vec<TokenWithRange> {
    // remove all comments.
    let mut token_iter = tokens.into_iter();
    let peekable_token_iter = PeekableIter::new(&mut token_iter, 1);
    let mut clean_tokens: Vec<TokenWithRange> = vec![];

    for tr in peekable_token_iter {
        match tr {
            TokenWithRange {
                token: Token::Comment(_),
                ..
            } => {
                // consume comments
            }
            _ => {
                clean_tokens.push(tr);
            }
        }
    }

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
        error::Error,
    };

    use super::clean;

    fn clean_lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, Error> {
        let tokens = lex_from_str(s)?;
        let clean_tokens = clean(tokens);
        Ok(clean_tokens)
    }

    fn clean_lex_from_str_without_location(s: &str) -> Result<Vec<Token>, Error> {
        let tokens = clean_lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_clear_comments() {
        assert_eq!(
            clean_lex_from_str_without_location(
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
                Token::NewLine,
                Token::Char('2'),
                Token::NewLine,
                Token::NewLine,
                Token::Char('3'),
                Token::NewLine,
            ]
        );
    }
}
