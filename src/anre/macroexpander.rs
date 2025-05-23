// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use crate::{location::Location, peekableiter::PeekableIter, AnreError};

use super::token::{Token, TokenWithRange};

/// Extracts macro definitions from the token stream.
fn extract_definitions(
    mut tokens: Vec<TokenWithRange>,
) -> Result<(Vec<TokenWithRange>, Vec<Definition>), AnreError> {
    let mut definitions: Vec<Definition> = vec![];
    loop {
        let pos = tokens.iter().position(|token_with_range| {
            matches!(token_with_range, TokenWithRange {
                token: Token::Identifier(id),
                ..  } if id == "define" )
        });

        if pos.is_none() {
            break;
        }

        let start = pos.unwrap();
        let mut depth: usize = 0;

        let mut end_option: Option<usize> = None;
        let mut idx = start + 1;

        // find the ending ')'
        while idx < tokens.len() {
            match tokens[idx].token {
                Token::LeftParen => {
                    // found '('
                    depth += 1;
                }
                Token::RightParen => {
                    // found ')'
                    if depth == 1 {
                        end_option = Some(idx);
                        break;
                    } else {
                        depth -= 1;
                    }
                }
                _ => {
                    // pass
                }
            }

            idx += 1;
        }

        // extract one definition
        if let Some(end) = end_option {
            let definition_tokens: Vec<TokenWithRange> = tokens.drain(start..(end + 1)).collect();
            let mut definition_token_iter = definition_tokens.into_iter();
            let mut peekable_iter = PeekableIter::new(&mut definition_token_iter, 1);
            let mut extractor = DefinitionExtractor::new(&mut peekable_iter);
            let definition = extractor.extract()?;
            definitions.push(definition);
        } else {
            return Err(AnreError::UnexpectedEndOfDocument(
                "Incomplete definition statement.".to_owned(),
            ));
        }
    }

    Ok((tokens, definitions))
}

/// Replaces identifiers in the token stream with their corresponding macro definitions.
fn replace_identifiers(
    mut program_tokens: Vec<TokenWithRange>,
    mut definitions: Vec<Definition>,
) -> Vec<TokenWithRange> {
    definitions.reverse();
    while let Some(definition) = definitions.pop() {
        for idx in (0..definitions.len()).rev() {
            find_and_replace_identifiers(
                &mut definitions[idx].tokens,
                &definition.name,
                &definition.tokens,
            );
        }

        find_and_replace_identifiers(&mut program_tokens, &definition.name, &definition.tokens);
    }

    program_tokens
}

fn find_and_replace_identifiers(
    source_tokens: &mut Vec<TokenWithRange>,
    find_id: &str,
    replace_with: &[TokenWithRange],
) {
    for idx in (0..source_tokens.len()).rev() {
        if let Token::Identifier(id) = &source_tokens[idx].token {
            if id == find_id {
                // remove the identifier token, and insert the target tokens
                source_tokens.splice(idx..(idx + 1), replace_with.iter().cloned());
            }
        }
    }
}

/// Expands macros in the token stream by replacing defined identifiers with their corresponding tokens.
/// The input tokens must be free of comments and normalized.
pub fn expand(tokens: Vec<TokenWithRange>) -> Result<Vec<TokenWithRange>, AnreError> {
    let (program_tokens, definitions) = extract_definitions(tokens)?;
    let expand_tokens = replace_identifiers(program_tokens, definitions);

    Ok(expand_tokens)
}

#[derive(Debug, PartialEq)]
struct Definition {
    name: String,
    tokens: Vec<TokenWithRange>,
}

pub struct DefinitionExtractor<'a> {
    upstream: &'a mut PeekableIter<'a, TokenWithRange>,
    last_range: Location,
}

impl<'a> DefinitionExtractor<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, TokenWithRange>) -> Self {
        Self {
            upstream,
            last_range: Location::new_range(/*0,*/ 0, 0, 0, 0),
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        match self.upstream.next() {
            Some(TokenWithRange { token, range }) => {
                self.last_range = range;
                Some(token)
            }
            None => None,
        }
    }

    fn peek_token(&self, offset: usize) -> Option<&Token> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { token, .. }) => Some(token),
            None => None,
        }
    }

    fn peek_range(&self, offset: usize) -> Option<&Location> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { range, .. }) => Some(range),
            None => None,
        }
    }

    // consume '\n' if it exists.
    fn consume_new_line_if_exist(&mut self) -> bool {
        match self.peek_token(0) {
            Some(Token::NewLine) => {
                self.next_token();
                true
            }
            _ => false,
        }
    }

    fn consume_new_line_or_comma(&mut self) -> Result<(), AnreError> {
        match self.peek_token(0) {
            Some(Token::NewLine | Token::Comma) => {
                self.next_token();
                Ok(())
            }
            Some(_) => Err(AnreError::MessageWithLocation(
                "Expect a comma or new-line.".to_owned(),
                self.peek_range(0).unwrap().get_position_by_range_start(),
            )),
            None => Err(AnreError::UnexpectedEndOfDocument(
                "Expect a comma or new-line.".to_owned(),
            )),
        }
    }

    fn consume_identifier(&mut self) -> Result<String, AnreError> {
        match self.peek_token(0) {
            Some(Token::Identifier(s)) => {
                let id = s.to_owned();
                self.next_token();
                Ok(id)
            }
            Some(_) => Err(AnreError::MessageWithLocation(
                "Expect an identifier.".to_owned(),
                self.peek_range(0).unwrap().get_position_by_range_start(),
            )),
            None => Err(AnreError::UnexpectedEndOfDocument(
                "Expect an identifier.".to_owned(),
            )),
        }
    }

    fn extract(&mut self) -> Result<Definition, AnreError> {
        // "define" "(" ... ")" ?
        // -------- ---     --- -
        // ^        ^       ^__ validated
        // |        |__ validated
        // | current validated

        self.next_token(); // consume "define"
        self.consume_new_line_if_exist(); // consume trailing new-line

        self.next_token(); // consume '('
        self.consume_new_line_if_exist(); // consume trailing new-line

        let name = self.consume_identifier()?;
        self.consume_new_line_or_comma()?;

        let mut token_with_ranges = vec![];

        while let Some(token_with_range) = self.upstream.next() {
            // exclude the last one
            if self.peek_token(0).is_some() {
                token_with_ranges.push(token_with_range);
            }
        }

        let definition = Definition {
            name,
            tokens: token_with_ranges,
        };

        Ok(definition)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        anre::{
            commentremover::clean,
            lexer::lex_from_str,
            normalizer::normalize,
            token::{Token, TokenWithRange},
        },
        AnreError,
    };

    use super::expand;

    fn expand_and_lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, AnreError> {
        let tokens = lex_from_str(s)?;
        let clean_tokens = clean(tokens);
        let normalized_tokens = normalize(clean_tokens);
        let expanded_tokens = expand(normalized_tokens)?;
        let expanded_and_normalized_tokens = normalize(expanded_tokens);
        Ok(expanded_and_normalized_tokens)
    }

    fn expand_and_lex_from_str_without_location(s: &str) -> Result<Vec<Token>, AnreError> {
        let tokens = expand_and_lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_macro_expand() {
        assert_eq!(
            expand_and_lex_from_str_without_location(
                r#"
            define(a, 'a')
            start, a, end
            "#,
            )
            .unwrap(),
            vec![
                Token::new_anchor_assertion("start"),
                Token::Comma,
                Token::Char('a'),
                Token::Comma,
                Token::new_anchor_assertion("end"),
            ]
        );

        assert_eq!(
            expand_and_lex_from_str_without_location(
                r#"
            define(a, 'a')
            define(b, a+)
            start, a, b, end
            "#,
            )
            .unwrap(),
            vec![
                Token::new_anchor_assertion("start"),
                Token::Comma,
                Token::Char('a'),
                Token::Comma,
                Token::Char('a'),
                Token::Plus,
                Token::Comma,
                Token::new_anchor_assertion("end"),
            ]
        );

        assert_eq!(
            expand_and_lex_from_str_without_location(
                r#"
            define(a, 'a')
            define(b, (a, 'b'))
            define(c, ([a, 'c'], optional(b), b.one_or_more()))
            define(d, (a || b || 'd'))
            start
            a
            b
            c
            d
            end
            "#,
            )
            .unwrap(),
            vec![
                // start
                Token::new_anchor_assertion("start"),
                Token::NewLine,
                // a
                Token::Char('a'),
                Token::NewLine,
                // b
                Token::LeftParen,
                Token::Char('a'),
                Token::Comma,
                Token::Char('b'),
                Token::RightParen,
                Token::NewLine,
                // c
                Token::LeftParen,
                // c - [a, 'c']
                Token::LeftBracket,
                Token::Char('a'),
                Token::Comma,
                Token::Char('c'),
                Token::RightBracket,
                Token::Comma,
                // c - optional(b)
                Token::new_identifier("optional"),
                Token::LeftParen,
                Token::LeftParen,
                Token::Char('a'),
                Token::Comma,
                Token::Char('b'),
                Token::RightParen,
                Token::RightParen,
                Token::Comma,
                // c - b.one_or_more()
                Token::LeftParen,
                Token::Char('a'),
                Token::Comma,
                Token::Char('b'),
                Token::RightParen,
                Token::Dot,
                Token::new_identifier("one_or_more"),
                Token::LeftParen,
                Token::RightParen,
                // c
                Token::RightParen,
                Token::NewLine,
                // d
                Token::LeftParen,
                Token::Char('a'),
                Token::LogicOr,
                Token::LeftParen,
                Token::Char('a'),
                Token::Comma,
                Token::Char('b'),
                Token::RightParen,
                Token::LogicOr,
                Token::Char('d'),
                Token::RightParen,
                Token::NewLine,
                // end
                Token::new_anchor_assertion("end"),
            ]
        );
    }
}
