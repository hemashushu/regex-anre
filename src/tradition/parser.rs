// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub const PARSER_PEEK_TOKEN_MAX_COUNT: usize = 1;

use crate::{
    ast::{
        AnchorAssertionName, BoundaryAssertionName, CharRange, CharSet, CharSetElement, Expression,
        FunctionCall, FunctionCallArg, FunctionName, Literal, PresetCharSetName, Program,
        SpecialCharName,
    },
    error::Error,
    location::Location,
    peekableiter::PeekableIter,
};

use super::{
    lexer::lex_from_str,
    token::{Repetition, Token, TokenWithRange},
};

pub struct Parser<'a> {
    upstream: &'a mut PeekableIter<'a, TokenWithRange>,
    last_range: Location,
}

impl<'a> Parser<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, TokenWithRange>) -> Self {
        Self {
            upstream,
            last_range: Location::new_range(0, 0, 0, 0, 0),
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

    fn peek_token_and_equals(&self, offset: usize, expected_token: &Token) -> bool {
        matches!(
            self.upstream.peek(offset),
            Some(TokenWithRange { token, .. }) if token == expected_token)
    }

    fn expect_token(
        &mut self,
        expected_token: &Token,
        token_description: &str,
    ) -> Result<(), Error> {
        match self.next_token() {
            Some(token) => {
                if &token == expected_token {
                    Ok(())
                } else {
                    Err(Error::MessageWithLocation(
                        format!("Expect token: {}.", token_description),
                        self.last_range.get_position_by_range_start(),
                    ))
                }
            }
            None => Err(Error::UnexpectedEndOfDocument(format!(
                "Expect token: {}.",
                token_description
            ))),
        }
    }

    fn expect_char(&mut self) -> Result<char, Error> {
        match self.peek_token(0) {
            Some(Token::Char(c)) => {
                let ch = *c;
                self.next_token();
                Ok(ch)
            }
            Some(_) => Err(Error::MessageWithLocation(
                "Expect a char.".to_owned(),
                self.last_range.get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument("Expect a char.".to_owned())),
        }
    }
}

impl<'a> Parser<'a> {
    pub fn parse_program(&mut self) -> Result<Program, Error> {
        let mut expressions = vec![];

        // there is only one expression in the tradition regular expression
        if self.peek_token(0).is_some() {
            let expression = self.parse_expression()?;
            expressions.push(expression);
        }

        let program = if expressions.len() == 1
            && matches!(expressions.first().unwrap(), Expression::Group(_))
        {
            // extra expressions from the group 0
            let first = expressions.remove(0);
            if let Expression::Group(exps) = first {
                Program { expressions: exps }
            } else {
                unreachable!()
            }
        } else {
            Program { expressions }
        };

        Ok(program)
    }

    fn parse_expression(&mut self) -> Result<Expression, Error> {
        // token ...
        // -----
        // ^
        // | current, None or Some(...)

        // the expression parsing order:
        //
        //    > precedence low <
        // 1. binary expressions (logic or, and, equality, comparision, additive, multiplicative etc.)
        // 2. unary expressions (negative etc.)
        // 3. simple expressions (dot function call, slice, index etc.)
        // 4. primary expressions (group, list, map, identifier, literal etc.)
        //    > precedence high <

        self.parse_logic_or()
    }

    fn parse_logic_or(&mut self) -> Result<Expression, Error> {
        // token ... [ "|" expression ]
        // -----
        // ^
        // | current, None or Some(...)

        // in the traditional regular expressions, "groups" are implied
        // on both sides of the "logic or" ("|") operator.

        let mut left = self.parse_consecutive_expression()?;

        // """
        // The | operator has the lowest precedence in a regular expression.
        // If you want to use a disjunction as a part of a bigger pattern,
        // you must group it.
        // """
        //
        // e.g.
        // "ab|cd" == "(ab)|(cd)"
        // "ab|cd" != "a(b|c)d)"
        //
        // ref:
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Disjunction
        while let Some(Token::LogicOr) = self.peek_token(0) {
            self.next_token(); // consume "|"

            // Operator associativity
            // - https://en.wikipedia.org/wiki/Operator_associativity
            // - https://en.wikipedia.org/wiki/Operators_in_C_and_C%2B%2B#Operator_precedence
            //
            // left-associative, left-to-right associative
            // a || b || c -> (a || b) || c
            //
            // right-associative, righ-to-left associative
            // a || b || c -> a || (b || c)
            //
            // note:
            // using `parse_expression` for right-to-left associative, e.g.
            // `let right = self.parse_expression()?;`
            // or
            // using `parse_consecutive_expression` for left-to-right associative, e.g.
            // `let right = self.parse_consecutive_expression()?;`
            //
            // for the current interpreter, it is more efficient by using right-associative.

            let right = self.parse_expression()?;
            let expression = Expression::Or(Box::new(left), Box::new(right));
            left = expression;
        }

        Ok(left)
    }

    fn parse_consecutive_expression(&mut self) -> Result<Expression, Error> {
        // token ...
        // -----
        // ^
        // | current, None or Some(...)

        let mut expressions = vec![];
        while let Some(token) = self.peek_token(0) {
            match token {
                // terminator
                Token::GroupEnd | Token::LogicOr => {
                    break;
                }
                _ => {
                    let expression = self.parse_notations()?;
                    expressions.push(expression);
                }
            }
        }

        if expressions.is_empty() {
            return Err(Error::MessageWithLocation(
                "Encountered a blank expression.".to_owned(),
                self.last_range,
            ));
        }

        // merge continous chars to string
        let mut index = expressions.len() - 1;
        while index > 0 {
            // found char
            if matches!(expressions[index], Expression::Literal(Literal::Char(_))) {
                // check prevous expressions
                let mut pre_index = index;
                while pre_index > 0 {
                    if !matches!(
                        expressions[pre_index - 1],
                        Expression::Literal(Literal::Char(_))
                    ) {
                        break;
                    }
                    pre_index -= 1;
                }

                // found continous chars
                if index - pre_index > 0 {
                    let cc: String = expressions
                        .drain(pre_index..=index)
                        .map(|item| {
                            if let Expression::Literal(Literal::Char(c)) = item {
                                c
                            } else {
                                unreachable!()
                            }
                        })
                        .collect();
                    expressions.insert(pre_index, Expression::Literal(Literal::String(cc)));

                    if pre_index == 0 {
                        break;
                    } else {
                        // update index and go on.
                        //
                        // `index` can be `pre_index - 2`, because we are confirmed
                        // `pre_index - 1` is not a char.
                        index = pre_index - 1;
                        continue;
                    }
                }
            }
            index -= 1;
        }

        if expressions.len() == 1 {
            // escape the group if it contains only one element
            let first = expressions.remove(0);
            Ok(first)
        } else {
            // an implied group
            Ok(Expression::Group(expressions))
        }
    }

    fn parse_notations(&mut self) -> Result<Expression, Error> {
        // token ... notations
        // -----
        // ^
        // | current, Some(...)

        let mut expression = self.parse_primary_expression()?;

        while let Some(token) = self.peek_token(0) {
            match token {
                Token::Optional
                | Token::OptionalLazy
                | Token::OneOrMore
                | Token::OneOrMoreLazy
                | Token::ZeroOrMore
                | Token::ZeroOrMoreLazy => {
                    let name = match token {
                        // Greedy quantifier
                        Token::Optional => FunctionName::Optional,
                        Token::OneOrMore => FunctionName::OneOrMore,
                        Token::ZeroOrMore => FunctionName::ZeroOrMore,

                        // Lazy quantifier
                        Token::OptionalLazy => FunctionName::OptionalLazy,
                        Token::OneOrMoreLazy => FunctionName::OneOrMoreLazy,
                        Token::ZeroOrMoreLazy => FunctionName::ZeroOrMoreLazy,

                        _ => unreachable!(),
                    };

                    let function_call = FunctionCall {
                        name,
                        expression: Box::new(expression),
                        args: vec![],
                    };
                    expression = Expression::FunctionCall(Box::new(function_call));

                    self.next_token(); // consume notation
                }
                Token::Repetition(repetition, lazy) => {
                    let mut args = vec![];
                    let name = match repetition {
                        Repetition::Specified(n) => {
                            if *lazy {
                                return Err(Error::MessageWithLocation(
                                    "Specified number of repetitions does not support lazy mode, i.e. '{m}?' is not allowed.".to_owned(), self.last_range));
                            }

                            args.push(FunctionCallArg::Number(*n));
                            FunctionName::Repeat
                        }
                        Repetition::AtLeast(n) => {
                            args.push(FunctionCallArg::Number(*n));

                            if *lazy {
                                FunctionName::AtLeastLazy
                            } else {
                                FunctionName::AtLeast
                            }
                        }
                        Repetition::Range(m, n) => {
                            if *lazy && m == n {
                                return Err(Error::MessageWithLocation(
                                    "Specified number of repetitions does not support lazy mode, i.e. '{m,m}?' is not allowed.".to_owned(), self.last_range));
                            }

                            args.push(FunctionCallArg::Number(*m));
                            args.push(FunctionCallArg::Number(*n));

                            if *lazy {
                                FunctionName::RepeatRangeLazy
                            } else {
                                FunctionName::RepeatRange
                            }
                        }
                    };

                    let function_call = FunctionCall {
                        name,
                        expression: Box::new(expression),
                        args,
                    };
                    expression = Expression::FunctionCall(Box::new(function_call));
                }
                _ => {
                    break;
                }
            }
        }

        Ok(expression)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, Error> {
        // token ...
        // ---------
        // ^
        // | current, Some(...)

        // primary expressions:
        // - literal
        // - anchor assertion
        // - boundary assertion
        // - look around assertion
        // - group
        // - back reference
        let expression = match self.peek_token(0).unwrap() {
            Token::StartAssertion => Expression::AnchorAssertion(AnchorAssertionName::Start),
            Token::EndAssertion => Expression::AnchorAssertion(AnchorAssertionName::End),
            Token::BoundaryAssertion(c) => match c {
                'b' => Expression::BoundaryAssertion(BoundaryAssertionName::IsBound),
                'B' => Expression::BoundaryAssertion(BoundaryAssertionName::IsNotBound),
                _ => unreachable!(),
            },
            token @ (Token::LookAhead
            | Token::LookAheadNegative
            | Token::LookBehind
            | Token::LookBehindNegative) => {
                let name = match token {
                    Token::LookAhead => FunctionName::IsBefore,
                    Token::LookAheadNegative => FunctionName::IsNotBefore,
                    Token::LookBehind => FunctionName::IsAfter,
                    Token::LookBehindNegative => FunctionName::IsNotAfter,
                    _ => unreachable!(),
                };

                self.next_token(); // consume "(?="
                let expression = self.parse_expression()?;
                self.next_token(); // consume ")"

                let function_call = FunctionCall {
                    name,
                    expression: Box::new(expression),
                    args: vec![],
                };
                Expression::FunctionCall(Box::new(function_call))
            }
            Token::GroupStart | Token::NonCapturing | Token::NamedCapture(_) => {
                self.parse_group()?
            }
            _ => {
                let literal = self.parse_literal()?;
                Expression::Literal(literal)
            }
        };

        Ok(expression)
    }

    fn parse_group(&mut self) -> Result<Expression, Error> {
        // "(" {expression} ")" ?
        // ---                  -
        // ^                    ^-- to here
        // | current, validated
        //
        // also "(?:" {expression} ")"

        // consume "(", "(?:" or "(?<...>"
        let head_token = self.next_token().unwrap();
        let expression = self.parse_expression()?;

        // consume ")"
        self.expect_token(&Token::GroupEnd, "right parenthese \")\"")?;

        let group_expression = match head_token {
            Token::GroupStart => {
                // regex group == ANRE indexed capture group
                let function_call = FunctionCall {
                    name: FunctionName::Index,
                    expression: Box::new(expression),
                    args: vec![],
                };
                Expression::FunctionCall(Box::new(function_call))
            }
            Token::NonCapturing => {
                // regex non-capturing == ANRE group
                expression
            }
            Token::NamedCapture(name) => {
                // named capture group
                let function_call = FunctionCall {
                    name: FunctionName::Name,
                    expression: Box::new(expression),
                    args: vec![FunctionCallArg::Identifier(name)],
                };
                Expression::FunctionCall(Box::new(function_call))
            }
            _ => unreachable!(),
        };

        Ok(group_expression)
    }

    fn parse_literal(&mut self) -> Result<Literal, Error> {
        // token ...
        // -----
        // ^
        // | current, Some(...)

        // literals:
        // - char
        // - charset
        // - preset charset

        let literal = match self.peek_token(0).unwrap() {
            Token::Char(c) => {
                let ch = *c;
                self.next_token(); // consume char
                Literal::Char(ch)
            }
            Token::CharSetStart | Token::CharSetStartNegative => {
                let charset = self.parse_charset()?;
                Literal::CharSet(charset)
            }
            Token::PresetCharSet(preset_charset_name_ref) => {
                let preset_charset_name = match preset_charset_name_ref {
                    'w' => PresetCharSetName::CharWord,
                    'W' => PresetCharSetName::CharNotWord,
                    's' => PresetCharSetName::CharSpace,
                    'S' => PresetCharSetName::CharNotSpace,
                    'd' => PresetCharSetName::CharDigit,
                    'D' => PresetCharSetName::CharNotDigit,
                    _ => unreachable!(),
                };
                self.next_token(); // consume preset charset
                Literal::PresetCharSet(preset_charset_name)
            }
            Token::Dot => {
                self.next_token(); // consume special char
                Literal::Special(SpecialCharName::CharAny)
            }
            _ => {
                return Err(Error::MessageWithLocation(
                    "Expect a literal.".to_owned(),
                    self.last_range,
                ));
            }
        };

        Ok(literal)
    }

    fn parse_charset(&mut self) -> Result<CharSet, Error> {
        // "[" {char | char_range | preset_charset} "]" ?
        // ---                                          -
        // ^                                            ^__ to here
        // | current, validated
        //
        // also: "[^" ...

        let head_token = self.next_token().unwrap(); // consume '[' or '[^'

        let mut elements = vec![];
        while let Some(token) = self.peek_token(0) {
            if token == &Token::CharSetEnd {
                break;
            }

            match token {
                Token::Char(c_ref) => {
                    // char
                    let c = *c_ref;

                    self.next_token(); // consume char
                    elements.push(CharSetElement::Char(c));
                }
                Token::CharRange(from, to) => {
                    // char range
                    let char_range = CharRange {
                        start: *from,
                        end_included: *to,
                    };

                    self.next_token(); // consume from
                    self.next_token(); // consume to
                    elements.push(CharSetElement::CharRange(char_range));
                }
                Token::PresetCharSet(preset_charset_name_ref) => {
                    // preset char set
                    let preset_charset_name = match preset_charset_name_ref {
                        'w' => PresetCharSetName::CharWord,
                        'W' => PresetCharSetName::CharNotWord,
                        's' => PresetCharSetName::CharSpace,
                        'S' => PresetCharSetName::CharNotSpace,
                        'd' => PresetCharSetName::CharDigit,
                        'D' => PresetCharSetName::CharNotDigit,
                        _ => unreachable!(),
                    };
                    self.next_token(); // consume preset charset
                    elements.push(CharSetElement::PresetCharSet(preset_charset_name));
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        "Unsupported char set element.".to_owned(),
                        self.last_range,
                    ));
                }
            }
        }

        self.expect_token(&Token::CharSetEnd, "right bracket \"]\"")?;

        let charset = CharSet {
            negative: matches!(head_token, Token::CharSetStartNegative),
            elements,
        };

        Ok(charset)
    }
}

pub fn parse_from_str(s: &str) -> Result<Program, Error> {
    let tokens = lex_from_str(s)?;
    let mut token_iter = tokens.into_iter();
    let mut peekable_token_iter = PeekableIter::new(&mut token_iter, PARSER_PEEK_TOKEN_MAX_COUNT);
    let mut parser = Parser::new(&mut peekable_token_iter);
    parser.parse_program()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::ast::{Expression, Literal, PresetCharSetName, Program};

    use super::parse_from_str;

    #[test]
    fn test_parse_literal_simple() {
        {
            let program = parse_from_str(r#"a\w"#).unwrap();

            assert_eq!(
                program,
                Program {
                    expressions: vec![
                        Expression::Literal(Literal::Char('a')),
                        Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharWord)),
                    ]
                }
            );

            assert_eq!(program.to_string(), r#"'a', char_word"#);
        }

        // merge continous chars
        {
            let program = parse_from_str(r#"abc\dmn\dp\dxyz"#).unwrap();

            assert_eq!(
                program,
                Program {
                    expressions: vec![
                        Expression::Literal(Literal::String("abc".to_owned())),
                        Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharDigit)),
                        Expression::Literal(Literal::String("mn".to_owned())),
                        Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharDigit)),
                        Expression::Literal(Literal::Char('p')),
                        Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharDigit)),
                        Expression::Literal(Literal::String("xyz".to_owned())),
                    ]
                }
            );

            assert_eq!(
                program.to_string(),
                r#""abc", char_digit, "mn", char_digit, 'p', char_digit, "xyz""#
            );
        }
    }
}
