// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

pub const PARSER_PEEK_TOKEN_MAX_COUNT: usize = 4;

use crate::{
    ast::{
        AnchorAssertionName, BackReference, BoundaryAssertionName, CharRange, CharSet,
        CharSetElement, Expression, FunctionCall, FunctionCallArg, FunctionName, Literal,
        PresetCharSetName, Program, SpecialCharName,
    },
    location::Location,
    peekableiter::PeekableIter,
    AnreError,
};

use super::{
    commentremover::clean,
    lexer::lex_from_str,
    macroexpander::expand,
    normalizer::normalize,
    token::{Token, TokenWithRange},
};

pub struct Parser<'a> {
    upstream: &'a mut PeekableIter<'a, TokenWithRange>,
    last_range: Location,
}

impl<'a> Parser<'a> {
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

    fn peek_range(&self, offset: usize) -> Option<&Location> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { range, .. }) => Some(range),
            None => None,
        }
    }

    fn peek_token(&self, offset: usize) -> Option<&Token> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { token, .. }) => Some(token),
            None => None,
        }
    }

    fn expect_token(&self, offset: usize, expected_token: &Token) -> bool {
        matches!(
            self.peek_token(offset),
            Some(token) if token == expected_token)
    }

    /// Returns:
    /// - `None` if the specified token is not found.
    /// - `Some(false)` found the token without new-line.
    /// - `Some(true)` found the token and new-line.
    fn expect_token_ignore_newline(&self, offset: usize, expected_token: &Token) -> Option<bool> {
        if self.expect_token(offset, expected_token) {
            Some(false)
        } else if self.expect_token(offset, &Token::NewLine)
            && self.expect_token(offset + 1, expected_token)
        {
            Some(true)
        } else {
            None
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

    // consume '\n' or ',' if they exist.
    fn consume_new_line_or_comma_if_exist(&mut self) -> bool {
        match self.peek_token(0) {
            Some(Token::NewLine | Token::Comma) => {
                self.next_token();
                true
            }
            _ => false,
        }
    }

    fn consume_token(
        &mut self,
        expected_token: &Token,
        token_description: &str,
    ) -> Result<(), AnreError> {
        match self.next_token() {
            Some(token) => {
                if &token == expected_token {
                    Ok(())
                } else {
                    Err(AnreError::MessageWithLocation(
                        format!("Expect token: {}.", token_description),
                        self.last_range.get_position_by_range_start(),
                    ))
                }
            }
            None => Err(AnreError::UnexpectedEndOfDocument(format!(
                "Expect token: {}.",
                token_description
            ))),
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

    fn consume_number(&mut self) -> Result<usize, AnreError> {
        match self.peek_token(0) {
            Some(Token::Number(n)) => {
                let num = *n;
                self.next_token();
                Ok(num)
            }
            Some(_) => Err(AnreError::MessageWithLocation(
                "Expect a number.".to_owned(),
                self.peek_range(0).unwrap().get_position_by_range_start(),
            )),
            None => Err(AnreError::UnexpectedEndOfDocument(
                "Expect a number.".to_owned(),
            )),
        }
    }

    fn consume_char(&mut self) -> Result<char, AnreError> {
        match self.peek_token(0) {
            Some(Token::Char(c)) => {
                let ch = *c;
                self.next_token();
                Ok(ch)
            }
            Some(_) => Err(AnreError::MessageWithLocation(
                "Expect a char.".to_owned(),
                self.peek_range(0).unwrap().get_position_by_range_start(),
            )),
            None => Err(AnreError::UnexpectedEndOfDocument(
                "Expect a char.".to_owned(),
            )),
        }
    }

    // '('
    fn consume_left_paren(&mut self) -> Result<(), AnreError> {
        self.consume_token(&Token::LeftParen, "left parenthese")
    }

    // ')'
    fn consume_right_paren(&mut self) -> Result<(), AnreError> {
        self.consume_token(&Token::RightParen, "right parenthese")
    }

    // ']'
    fn consume_right_bracket(&mut self) -> Result<(), AnreError> {
        self.consume_token(&Token::RightBracket, "right bracket")
    }

    // '}'
    fn consume_right_brace(&mut self) -> Result<(), AnreError> {
        self.consume_token(&Token::RightBrace, "right brace")
    }
}

impl Parser<'_> {
    pub fn parse_program(&mut self) -> Result<Program, AnreError> {
        let mut expressions = vec![];

        while self.peek_token(0).is_some() {
            let expression = self.parse_expression()?;
            expressions.push(expression);

            // consume separator
            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        // extract elements from a group if the group contains only one element
        // and the element's type is 'Group'

        let program = if expressions.len() == 1
            && matches!(expressions.first().unwrap(), Expression::Group(_))
        {
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

    fn parse_expression(&mut self) -> Result<Expression, AnreError> {
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

    fn parse_logic_or(&mut self) -> Result<Expression, AnreError> {
        // token ... [ "||" expression ]
        // -----
        // ^
        // | current, None or Some(...)

        let mut left = self.parse_notation_and_rear_function_call()?;

        // while let Some(Token::LogicOr) = self.peek_token(0) {
        while self.peek_token(0).is_some() {
            match self.expect_token_ignore_newline(0, &Token::LogicOr) {
                Some(exists_new_line) => {
                    if exists_new_line {
                        self.next_token(); // consume '\n'
                    }

                    self.next_token(); // consume "||"
                    self.consume_new_line_if_exist(); // consume trailing new-line

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
                    // using `parse_notation_and_rear_function_call` for left-to-right associative, e.g.
                    // `let right = self.parse_notation_and_rear_function_call()?;`
                    //
                    // for the current interpreter, it is more efficient by using right-associative.

                    let right = self.parse_expression()?;
                    let expression = Expression::Or(Box::new(left), Box::new(right));
                    left = expression;
                }
                None => {
                    break;
                }
            }
        }

        Ok(left)
    }

    fn parse_notation_and_rear_function_call(&mut self) -> Result<Expression, AnreError> {
        // token ... [ notations | ("." identifier "(" ...)]
        // -----
        // ^
        // | current, None or Some(...)
        //
        // the new-line rules:
        // - 'expression' notations
        // - 'expression' newline? '.' newline? identifier newline? ( ... )

        let mut expression = self.parse_primary_expression()?;

        while let Some(token) = self.peek_token(0) {
            match token {
                Token::Question
                | Token::Plus
                | Token::Asterisk
                | Token::QuestionLazy
                | Token::PlusLazy
                | Token::AsteriskLazy => {
                    /* notations */

                    let name = match token {
                        // Greedy quantifier
                        Token::Question => FunctionName::Optional,
                        Token::Plus => FunctionName::OneOrMore,
                        Token::Asterisk => FunctionName::ZeroOrMore,

                        // Lazy quantifier
                        Token::QuestionLazy => FunctionName::OptionalLazy,
                        Token::PlusLazy => FunctionName::OneOrMoreLazy,
                        Token::AsteriskLazy => FunctionName::ZeroOrMoreLazy,

                        _ => unreachable!(),
                    };

                    let function_call = FunctionCall {
                        name,
                        // expression: Box::new(expression),
                        args: vec![FunctionCallArg::Expression(Box::new(expression))],
                    };
                    expression = Expression::FunctionCall(Box::new(function_call));

                    self.next_token(); // consume notation
                }
                Token::LeftBrace => {
                    /* repeat specified or range */

                    let (notation_quantifier, lazy) = self.continue_parse_notation_quantifier()?;

                    let mut args = vec![];
                    args.push(FunctionCallArg::Expression(Box::new(expression)));

                    let name = match notation_quantifier {
                        NotationQuantifier::Repeat(n) => {
                            if lazy {
                                return Err(AnreError::MessageWithLocation(
                                    "Specified number of repetitions does not support lazy mode, i.e. '{m}?' is not allowed.".to_owned(), self.last_range));
                            }

                            args.push(FunctionCallArg::Number(n));
                            FunctionName::Repeat
                        }
                        NotationQuantifier::RepeatRange(m, n) => {
                            if lazy && m == n {
                                return Err(AnreError::MessageWithLocation(
                                    "Specified number of repetitions does not support lazy mode, i.e. '{m,m}?' is not allowed.".to_owned(), self.last_range));
                            }

                            args.push(FunctionCallArg::Number(m));
                            args.push(FunctionCallArg::Number(n));

                            if lazy {
                                FunctionName::RepeatRangeLazy
                            } else {
                                FunctionName::RepeatRange
                            }
                        }
                        NotationQuantifier::AtLeast(n) => {
                            args.push(FunctionCallArg::Number(n));

                            if lazy {
                                FunctionName::AtLeastLazy
                            } else {
                                FunctionName::AtLeast
                            }
                        }
                    };

                    let function_call = FunctionCall {
                        name,
                        // expression: Box::new(expression),
                        args,
                    };
                    expression = Expression::FunctionCall(Box::new(function_call));
                }
                _ => {
                    if let Some(exists_newline) = self.expect_token_ignore_newline(0, &Token::Dot) {
                        if exists_newline {
                            self.next_token(); // consume '\n'
                        }

                        let function_call = self.continue_parse_rear_function_call(expression)?;
                        expression = Expression::FunctionCall(Box::new(function_call));
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(expression)
    }

    fn continue_parse_notation_quantifier(
        &mut self,
    ) -> Result<(NotationQuantifier, bool), AnreError> {
        // {m, n}? ?
        // -       -
        // ^       ^__ to here
        // | current, validated

        self.next_token(); // consume '{'
        self.consume_new_line_if_exist(); // consume trailing new-line

        let from = self.consume_number()?;

        // the comma that follows the first number is NOT a separator, it
        // can not be replaced by a newline like a normal comma,
        // its presence indicates that there is a second number, or that
        // the value of the second number is infinite.

        let (dual, to_optional) = if self.expect_token(0, &Token::Comma) {
            // example:
            //
            // `{m,}` `{m,n}`
            //
            // ```
            // {
            //     m,
            // }
            // ```
            //
            // ```
            // {
            //     m, n
            // }
            // ```
            self.next_token(); // consume ','

            let to_optional = if let Some(Token::Number(to_ref)) = self.peek_token(0) {
                let to = *to_ref;
                self.next_token(); // consume number
                Some(to)
            } else {
                None
            };

            (true, to_optional)
        } else if self.expect_token(0, &Token::NewLine)
            && matches!(self.peek_token(1), Some(Token::Number(_)))
        {
            // example:
            //
            // ```
            // {
            //     m
            //     n
            // }
            // ```

            self.next_token(); // consume new-line

            let to_optional = if let Some(Token::Number(to)) = self.next_token() {
                Some(to)
            } else {
                unreachable!()
            };

            (true, to_optional)
        } else {
            // example:
            //
            // `{m}`
            //
            // ```
            // {
            //     m
            // }
            // ```
            (false, None)
        };

        self.consume_new_line_if_exist();
        self.consume_right_brace()?; // consume '}'

        let lazy = if self.expect_token(0, &Token::Question) {
            self.next_token(); // consume trailing '?'
            true
        } else {
            false
        };

        let quantifier = if dual {
            if let Some(to) = to_optional {
                NotationQuantifier::RepeatRange(from, to)
            } else {
                NotationQuantifier::AtLeast(from)
            }
        } else {
            NotationQuantifier::Repeat(from)
        };

        Ok((quantifier, lazy))
    }

    fn continue_parse_rear_function_call(
        &mut self,
        expression: Expression,
    ) -> Result<FunctionCall, AnreError> {
        // "." identifier "(" {args} ")"     ?
        // --- ---------- ---                -
        // ^   ^          ^__ NOT validated  ^ to here
        // |   |__ NOT validated
        // | current, validated

        // newline rules:
        // . newline? identifier newline? '(...)'

        self.next_token(); // consume '.'
        self.consume_new_line_if_exist();

        let name_string = self.consume_identifier()?; // consume function name
        self.consume_new_line_if_exist();

        let name = function_name_from_str(&name_string, &self.last_range)?;

        self.consume_left_paren()?; // consume '('
        self.consume_new_line_if_exist(); // consume trailing new-line

        let mut args = vec![];
        args.push(FunctionCallArg::Expression(Box::new(expression)));

        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightParen {
                break;
            }

            match token {
                Token::Number(num_ref) => {
                    let num = *num_ref;
                    self.next_token(); // consume number
                    args.push(FunctionCallArg::Number(num));
                }
                Token::Identifier(id_ref) => {
                    let id = id_ref.to_owned();
                    self.next_token(); // consume identifier
                    args.push(FunctionCallArg::Identifier(id));
                }
                _ => {
                    let expression = self.parse_expression()?;
                    args.push(FunctionCallArg::Expression(Box::new(expression)));
                }
            }

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.consume_right_paren()?; // consume ')'

        let function_call = FunctionCall {
            name,
            // expression: Box::new(expression),
            args,
        };

        Ok(function_call)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, AnreError> {
        // token ...
        // ---------
        // ^
        // | current, None or Some(...)

        // primary expressions:
        // - literal
        // - identifier (backreference)
        // - assertion
        // - group
        // - function call

        let expression = match self.peek_token(0) {
            Some(token) => {
                match token {
                    Token::LeftParen => {
                        // group
                        self.parse_group()?
                    }
                    Token::Identifier(_)
                        if self
                            .expect_token_ignore_newline(1, &Token::LeftParen)
                            .is_some() =>
                    {
                        // function call
                        self.parse_function_call()?
                    }
                    Token::Identifier(identifier_ref) => {
                        // identifier
                        let identifier = identifier_ref.to_owned();
                        self.next_token(); // consume identifier
                        Expression::BackReference(BackReference::Name(identifier))
                    }
                    Token::AnchorAssertion(name_ref) => {
                        // anchor assertion
                        let name = anchor_assertion_name_from_str(name_ref, &self.last_range)?;
                        self.next_token(); // consume assertion

                        Expression::AnchorAssertion(name)
                    }
                    Token::BoundaryAssertion(name_ref) => {
                        // boundary assertion
                        let name = boundary_assertion_name_from_str(name_ref, &self.last_range)?;
                        self.next_token(); // consume assertion

                        Expression::BoundaryAssertion(name)
                    }
                    _ => {
                        let literal = self.parse_literal()?;
                        Expression::Literal(literal)
                    }
                }
            }
            None => {
                return Err(AnreError::UnexpectedEndOfDocument(
                    "Expect an expression.".to_owned(),
                ));
            }
        };

        Ok(expression)
    }

    fn parse_group(&mut self) -> Result<Expression, AnreError> {
        // "(" {expression} ")" ?
        // ---                  -
        // ^                    ^-- to here
        // | current, validated

        self.next_token(); // consume "("
        self.consume_new_line_if_exist(); // consume trailing new-line

        let mut expressions: Vec<Expression> = vec![];

        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightParen {
                break;
            }

            let expression = self.parse_expression()?;
            expressions.push(expression);

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.consume_right_paren()?; // consume ")"

        // escape the group if it contains only one element
        if expressions.len() == 1 {
            let first = expressions.remove(0);
            Ok(first)
        } else {
            Ok(Expression::Group(expressions))
        }
    }

    fn parse_function_call(&mut self) -> Result<Expression, AnreError> {
        // identifier "(" expression ["," args... ] ")" ?
        // ---------- ---                               -
        // ^          ^__ validated                     ^__ to here
        // | current, validated

        // newline rules:
        // identifier newline? '(' newline? expression newline/comma ... ')'

        let name_string = self.consume_identifier()?;
        self.consume_new_line_if_exist();

        let name = function_name_from_str(&name_string, &self.last_range)?;

        self.consume_left_paren()?; // consume '('
        self.consume_new_line_if_exist(); // consume trailing new-line

        // let expression = self.parse_expression()?;
        // self.consume_new_line_or_comma_if_exist(); // consume trailing new-line

        let mut args = vec![];

        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightParen {
                break;
            }

            match token {
                Token::Number(num_ref) => {
                    let num = *num_ref;
                    self.next_token(); // consume number
                    args.push(FunctionCallArg::Number(num));
                }
                Token::Identifier(id_ref)
                    if self
                        .expect_token_ignore_newline(1, &Token::LeftParen)
                        .is_none() =>
                {
                    let id = id_ref.to_owned();
                    self.next_token(); // consume identifier
                    args.push(FunctionCallArg::Identifier(id));
                }
                _ => {
                    let expression = self.parse_expression()?;
                    args.push(FunctionCallArg::Expression(Box::new(expression)));
                }
            }

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.consume_right_paren()?; // consume ')'

        let function_call = FunctionCall {
            name,
            // expression: Box::new(expression),
            args,
        };

        Ok(Expression::FunctionCall(Box::new(function_call)))
    }

    fn parse_literal(&mut self) -> Result<Literal, AnreError> {
        // token ...
        // -----
        // ^
        // | current, Some(...)

        // literals:
        //   - char
        //   - string
        //   - charset
        //   - preset_charset
        //   - special char

        match self.peek_token(0) {
            Some(token) => {
                let literal = match token {
                    Token::LeftBracket => {
                        let elements = self.parse_charset()?;
                        Literal::CharSet(CharSet {
                            negative: false,
                            elements,
                        })
                    }
                    Token::Exclamation if self.expect_token(1, &Token::LeftBracket) => {
                        // negative charset
                        self.next_token();

                        let elements = self.parse_charset()?;
                        Literal::CharSet(CharSet {
                            negative: true,
                            elements,
                        })
                    }
                    Token::Char(char_ref) => {
                        let c = *char_ref;
                        self.next_token(); // consume char
                        Literal::Char(c)
                    }
                    Token::String(string_ref) => {
                        let string = string_ref.to_owned();
                        self.next_token(); // consume string
                        Literal::String(string)
                    }
                    Token::PresetCharSet(preset_charset_name_ref) => {
                        let preset_charset_name = preset_charset_name_from_str(
                            preset_charset_name_ref,
                            &self.last_range,
                        )?;
                        self.next_token(); // consume preset charset
                        Literal::PresetCharSet(preset_charset_name)
                    }
                    Token::Special(special_char_name_ref) => {
                        let special_char_name =
                            special_char_name_from_str(special_char_name_ref, &self.last_range)?;
                        self.next_token(); // consume special
                        Literal::Special(special_char_name)
                    }
                    _ => {
                        return Err(AnreError::MessageWithLocation(
                            "Expect a literal.".to_owned(),
                            self.last_range,
                        ));
                    }
                };

                Ok(literal)
            }
            None => {
                unreachable!()
            }
        }
    }

    fn parse_charset(&mut self) -> Result<Vec<CharSetElement>, AnreError> {
        // "[" {char | char_range | preset_charset | char_set} "]" ?
        // ---                                                   -
        // ^                                                     ^__ to here
        // | current, validated

        self.next_token(); // consume '['
        self.consume_new_line_if_exist(); // consume trailing new-line

        let mut elements = vec![];
        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightBracket {
                break;
            }

            match token {
                Token::Char(_) if self.expect_token_ignore_newline(1, &Token::Range).is_some() => {
                    // char range
                    let char_range = self.parse_char_range()?;
                    elements.push(CharSetElement::CharRange(char_range));
                }
                Token::Char(c_ref) => {
                    // char
                    let c = *c_ref;
                    self.next_token(); // consume char
                    elements.push(CharSetElement::Char(c));
                }
                Token::PresetCharSet(preset_charset_name_ref) => {
                    // preset char set
                    let preset_charset_name =
                        preset_charset_name_from_str(preset_charset_name_ref, &self.last_range)?;
                    self.next_token(); // consume preset charset
                    elements.push(CharSetElement::PresetCharSet(preset_charset_name));
                }
                Token::LeftBracket => {
                    // custom char set
                    // such as ['a'..'f']
                    let custom_charset_elements = self.parse_charset()?;
                    let custom_charset = CharSet {
                        negative: false,
                        elements: custom_charset_elements,
                    };
                    elements.push(CharSetElement::CharSet(Box::new(custom_charset)));
                }
                _ => {
                    return Err(AnreError::MessageWithLocation(
                        "Unexpected char set element.".to_owned(),
                        self.last_range,
                    ));
                }
            }

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.consume_right_bracket()?; // consume ']'

        Ok(elements)
    }

    fn parse_char_range(&mut self) -> Result<CharRange, AnreError> {
        // 'c' [new-line] '..' 'c' ?
        // ---  --------  ----     -
        // ^    ^         ^        ^__ to here
        // |    | vali..  | validated
        // | current, validated

        let char_start = self.consume_char()?; // consume start char
        self.consume_new_line_if_exist();

        self.next_token(); // consume '..'
        self.consume_new_line_if_exist();

        let char_end = self.consume_char()?; // consume end char

        Ok(CharRange {
            start: char_start,
            end_included: char_end,
        })
    }
}

enum NotationQuantifier {
    Repeat(usize),
    RepeatRange(usize, usize),
    AtLeast(usize),
}

fn anchor_assertion_name_from_str(
    name_str: &str,
    range: &Location,
) -> Result<AnchorAssertionName, AnreError> {
    let name = match name_str {
        "start" => AnchorAssertionName::Start,
        "end" => AnchorAssertionName::End,

        // Unexpect
        _ => {
            return Err(AnreError::MessageWithLocation(
                format!("Unexpect anchor assertion name: \"{}\"", name_str),
                range.to_owned(),
            ))
        }
    };

    Ok(name)
}

fn boundary_assertion_name_from_str(
    name_str: &str,
    range: &Location,
) -> Result<BoundaryAssertionName, AnreError> {
    let name = match name_str {
        "is_bound" => BoundaryAssertionName::IsBound,
        "is_not_bound" => BoundaryAssertionName::IsNotBound,

        // Unexpect
        _ => {
            return Err(AnreError::MessageWithLocation(
                format!("Unexpect boundary assertion name: \"{}\"", name_str),
                range.to_owned(),
            ))
        }
    };

    Ok(name)
}

fn special_char_name_from_str(
    name_str: &str,
    range: &Location,
) -> Result<SpecialCharName, AnreError> {
    let name = match name_str {
        "char_any" => SpecialCharName::CharAny,

        // Unexpect
        _ => {
            return Err(AnreError::MessageWithLocation(
                format!("Unexpect special character name: \"{}\"", name_str),
                range.to_owned(),
            ))
        }
    };

    Ok(name)
}

fn preset_charset_name_from_str(
    name_str: &str,
    range: &Location,
) -> Result<PresetCharSetName, AnreError> {
    let name = match name_str {
        "char_word" => PresetCharSetName::CharWord,
        "char_not_word" => PresetCharSetName::CharNotWord,
        "char_space" => PresetCharSetName::CharSpace,
        "char_not_space" => PresetCharSetName::CharNotSpace,
        "char_digit" => PresetCharSetName::CharDigit,
        "char_not_digit" => PresetCharSetName::CharNotDigit,

        // Unexpect
        _ => {
            return Err(AnreError::MessageWithLocation(
                format!("Unexpect preset charset name: \"{}\"", name_str),
                range.to_owned(),
            ))
        }
    };

    Ok(name)
}

fn function_name_from_str(name_str: &str, range: &Location) -> Result<FunctionName, AnreError> {
    let name = match name_str {
        // Greedy Quantifier
        "optional" => FunctionName::Optional,
        "one_or_more" => FunctionName::OneOrMore,
        "zero_or_more" => FunctionName::ZeroOrMore,
        "repeat" => FunctionName::Repeat,
        "repeat_range" => FunctionName::RepeatRange,
        "at_least" => FunctionName::AtLeast,

        // Lazy Quantifier
        "optional_lazy" => FunctionName::OptionalLazy,
        "one_or_more_lazy" => FunctionName::OneOrMoreLazy,
        "zero_or_more_lazy" => FunctionName::ZeroOrMoreLazy,
        "repeat_range_lazy" => FunctionName::RepeatRangeLazy,
        "at_least_lazy" => FunctionName::AtLeastLazy,

        // Assertions
        "is_before" => FunctionName::IsBefore, // lookahead
        "is_after" => FunctionName::IsAfter,   // lookbehind
        "is_not_before" => FunctionName::IsNotBefore, // negative lookahead
        "is_not_after" => FunctionName::IsNotAfter, // negative lookbehind

        // Capture
        "name" => FunctionName::Name,
        "index" => FunctionName::Index,

        // Unexpect
        _ => {
            return Err(AnreError::MessageWithLocation(
                format!("Unexpect function name: \"{}\"", name_str),
                range.to_owned(),
            ))
        }
    };

    Ok(name)
}

pub fn parse_from_str(s: &str) -> Result<Program, AnreError> {
    let tokens = lex_from_str(s)?;
    let clean_tokens = clean(tokens);
    let normalized_tokens = normalize(clean_tokens);
    let expanded_tokens = expand(normalized_tokens)?;
    let expanded_and_normalized_tokens = normalize(expanded_tokens);
    let mut token_iter = expanded_and_normalized_tokens.into_iter();
    let mut peekable_token_iter = PeekableIter::new(&mut token_iter, PARSER_PEEK_TOKEN_MAX_COUNT);
    let mut parser = Parser::new(&mut peekable_token_iter);
    parser.parse_program()
}

#[cfg(test)]
mod tests {

    use pretty_assertions::assert_eq;

    use crate::{
        ast::{
            CharRange, CharSet, CharSetElement, Expression, Literal, PresetCharSetName, Program,
        },
        AnreError,
    };

    use super::parse_from_str;

    #[test]
    fn test_parse_literal_simple() {
        let program = parse_from_str(
            r#"
'a', "foo", char_word
    "#,
        )
        .unwrap();

        assert_eq!(
            program,
            Program {
                expressions: vec![
                    Expression::Literal(Literal::Char('a')),
                    Expression::Literal(Literal::String("foo".to_owned())),
                    Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharWord)),
                ]
            }
        );

        assert_eq!(program.to_string(), r#"'a', "foo", char_word"#);
    }

    #[test]
    fn test_parse_literal_charset() {
        let program = parse_from_str(
            r#"
['a', '0'..'9', char_word]
    "#,
        )
        .unwrap();

        assert_eq!(
            program,
            Program {
                expressions: vec![Expression::Literal(Literal::CharSet(CharSet {
                    negative: false,
                    elements: vec![
                        CharSetElement::Char('a'),
                        CharSetElement::CharRange(CharRange {
                            start: '0',
                            end_included: '9'
                        }),
                        CharSetElement::PresetCharSet(PresetCharSetName::CharWord),
                    ]
                })),]
            }
        );

        assert_eq!(program.to_string(), r#"['a', '0'..'9', char_word]"#);

        // negative
        assert_eq!(
            parse_from_str(
                r#"
!['a'..'z', char_space]
    "#,
            )
            .unwrap()
            .to_string(),
            r#"!['a'..'z', char_space]"#
        );

        // nested
        assert_eq!(
            parse_from_str(
                r#"
['_', ['a'..'f'], ['0'..'9']]
    "#,
            )
            .unwrap()
            .to_string(),
            r#"['_', ['a'..'f'], ['0'..'9']]"#
        );

        // multiline
        assert_eq!(
            parse_from_str(
                r#"
[
    'a'
    '0'
    ..
    '9'
    char_word
]"#,
            )
            .unwrap()
            .to_string(),
            r#"['a', '0'..'9', char_word]"#
        );

        // multiline with comma
        assert_eq!(
            parse_from_str(
                r#"
[
    'a',
    '0'
    ..
    '9',
    char_word,
]"#,
            )
            .unwrap()
            .to_string(),
            r#"['a', '0'..'9', char_word]"#
        );
    }

    #[test]
    fn test_parse_expression_function_call() {
        assert_eq!(
            parse_from_str(
                r#"
optional('a')
one_or_more('b')
zero_or_more_lazy('c')
name("xyz", prefix)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"optional('a')
one_or_more('b')
zero_or_more_lazy('c')
name("xyz", prefix)"#
        );

        // test multiline args
        assert_eq!(
            parse_from_str(
                r#"
repeat(
    'a'
    3
)
repeat_range(
    'b'
    5
    7
)
at_least('c'
    11)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"repeat('a', 3)
repeat_range('b', 5, 7)
at_least('c', 11)"#
        );

        // test expressions as args
        assert_eq!(
            parse_from_str(
                r#"
is_after("bar", "foo" || 'f'{3})
                "#,
            )
            .unwrap()
            .to_string(),
            r#"is_after("bar", "foo" || repeat('f', 3))"#
        );

        // nested
        assert_eq!(
            parse_from_str(r#"optional(one_or_more('a'))"#)
                .unwrap()
                .to_string(),
            r#"optional(one_or_more('a'))"#
        );

        // nested + mulitline
        assert_eq!(
            parse_from_str(
                r#"
optional
(
one_or_more
(
'a'
)
)"#
            )
            .unwrap()
            .to_string(),
            r#"optional(one_or_more('a'))"#
        );
    }

    #[test]
    fn test_parse_expression_function_call_rear() {
        assert_eq!(
            parse_from_str(
                r#"
'a'.optional()
'b'.one_or_more()
'c'.zero_or_more_lazy()
"xyz".name(prefix)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"optional('a')
one_or_more('b')
zero_or_more_lazy('c')
name("xyz", prefix)"#
        );

        // test multiline args
        assert_eq!(
            parse_from_str(
                r#"
'a'.repeat(3)
'b'.repeat_range(
    5
    7
)
'c'.at_least(11
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"repeat('a', 3)
repeat_range('b', 5, 7)
at_least('c', 11)"#
        );

        // test expressions as args
        assert_eq!(
            parse_from_str(
                r#"
"bar".is_after("foo" || 'f'{3})
    "#,
            )
            .unwrap()
            .to_string(),
            r#"is_after("bar", "foo" || repeat('f', 3))"#
        );

        // test chaining
        assert_eq!(
            parse_from_str(
                r#"
['a'..'z'].repeat_range(2, 7).is_before("ing" || "ed")
    "#
            )
            .unwrap()
            .to_string(),
            r#"is_before(repeat_range(['a'..'z'], 2, 7), "ing" || "ed")"#
        );

        // test multiline
        assert_eq!(
            parse_from_str(
                r#"
['a'..'z']
.
repeat_range
(2, 7)
.
is_before
("ing" || "ed")
            "#,
            )
            .unwrap()
            .to_string(),
            r#"is_before(repeat_range(['a'..'z'], 2, 7), "ing" || "ed")"#
        );
    }

    #[test]
    fn test_parse_expression_notations() {
        assert_eq!(
            parse_from_str(
                r#"
'a'?
'b'+
'c'*
'x'??
'y'+?
'z'*?
    "#,
            )
            .unwrap()
            .to_string(),
            r#"optional('a')
one_or_more('b')
zero_or_more('c')
optional_lazy('x')
one_or_more_lazy('y')
zero_or_more_lazy('z')"#
        );

        assert_eq!(
            parse_from_str(
                r#"
'a'{3}
'b'{5,7}
'c'{11,}
'y'{5,7}?
'z'{11,}?
    "#,
            )
            .unwrap()
            .to_string(),
            r#"repeat('a', 3)
repeat_range('b', 5, 7)
at_least('c', 11)
repeat_range_lazy('y', 5, 7)
at_least_lazy('z', 11)"#
        );

        // err: '{m}?' is not allowed
        assert!(matches!(
            parse_from_str(
                r#"
'a'{3}?
"#,
            ),
            Err(AnreError::MessageWithLocation(_, _))
        ));

        // err: '{m,m}?' is not allowed
        assert!(matches!(
            parse_from_str(
                r#"
'a'{3,3}?
"#,
            ),
            Err(AnreError::MessageWithLocation(_, _))
        ));
    }

    #[test]
    fn test_parse_expression_logic_or() {
        {
            let program = parse_from_str(
                r#"
'a' || 'b'
"#,
            )
            .unwrap();

            assert_eq!(
                program,
                Program {
                    expressions: vec![Expression::Or(
                        Box::new(Expression::Literal(Literal::Char('a'))),
                        Box::new(Expression::Literal(Literal::Char('b'))),
                    )]
                }
            );

            assert_eq!(program.to_string(), r#"'a' || 'b'"#);
        }

        // more than 2 operands
        {
            let program = parse_from_str(
                r#"
'a' || 'b' || 'c'
"#,
            )
            .unwrap();

            assert_eq!(
                program,
                Program {
                    expressions: vec![Expression::Or(
                        Box::new(Expression::Literal(Literal::Char('a'))),
                        Box::new(Expression::Or(
                            Box::new(Expression::Literal(Literal::Char('b'))),
                            Box::new(Expression::Literal(Literal::Char('c'))),
                        )),
                    )]
                }
            );

            assert_eq!(program.to_string(), r#"'a' || ('b' || 'c')"#);
        }

        // test expressions as operands
        assert_eq!(
            parse_from_str(
                r#"
char_digit.one_or_more() || [char_word, '-']+
"#,
            )
            .unwrap()
            .to_string(),
            r#"one_or_more(char_digit) || one_or_more([char_word, '-'])"#
        );

        // test multiline
        {
            let program = parse_from_str(
                r#"
'a'
||
'b'
"#,
            )
            .unwrap();

            assert_eq!(
                program,
                Program {
                    expressions: vec![Expression::Or(
                        Box::new(Expression::Literal(Literal::Char('a'))),
                        Box::new(Expression::Literal(Literal::Char('b'))),
                    )]
                }
            );

            assert_eq!(program.to_string(), r#"'a' || 'b'"#);
        }
    }

    #[test]
    fn test_parse_expression_group() {
        assert_eq!(
            parse_from_str(
                r#"
("foo", char_digit)
('b', ("bar", char_digit))
end
"#,
            )
            .unwrap()
            .to_string(),
            r#"("foo", char_digit), ('b', ("bar", char_digit)), end"#
        );

        // function call + group
        assert_eq!(
            parse_from_str(
                r#"
repeat(("foo", char_digit), 3)
('b', repeat("bar", 5))
end
"#,
            )
            .unwrap()
            .to_string(),
            r#"repeat(("foo", char_digit), 3)
('b', repeat("bar", 5)), end"#
        );

        // logic or + group
        assert_eq!(
            parse_from_str(
                r#"
'a' || ('b' || 'c')
"#,
            )
            .unwrap()
            .to_string(),
            r#"'a' || ('b' || 'c')"#
        );

        // group + logic or
        assert_eq!(
            parse_from_str(
                r#"
('a' || 'b') || 'c'
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a' || 'b') || 'c'"#
        );

        // group + logic or + group
        assert_eq!(
            parse_from_str(
                r#"
('a', char_word) || ('b', char_digit)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', char_word) || ('b', char_digit)"#
        );

        // extract elements from a group if the group
        // contains only one element and the element's type
        // is 'Group'
        assert_eq!(
            parse_from_str(
                r#"
((('a', char_digit, 'b')))
"#,
            )
            .unwrap()
            .to_string(),
            r#"'a', char_digit, 'b'"#
        );

        // multiline
        assert_eq!(
            parse_from_str(
                r#"
(
'b'
(
"bar"
char_digit
)
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"'b', ("bar", char_digit)"#
        );
    }

    #[test]
    fn test_parse_macro() {
        assert_eq!(
            parse_from_str(
                r#"
define(a, "abc")
start, a, end
"#,
            )
            .unwrap()
            .to_string(),
            r#"start, "abc", end"#
        );

        assert_eq!(
            parse_from_str(
                r#"
define(a, 'a')
define(b, (a, 'b'))
define(c, ([a, 'c'], optional(b), b.one_or_more()))
define(d, (a || b || 'd'))
start, a, b, c, d, end
"#,
            )
            .unwrap()
            .to_string(),
            r#"start, 'a', ('a', 'b'), (['a', 'c'], optional(('a', 'b')), one_or_more(('a', 'b'))), 'a' || (('a', 'b') || 'd'), end"#
        );
    }

    #[test]
    fn test_parse_examples() {
        assert_eq!(
            parse_from_str(
                r#"
/**
 * Decimal Numbers Regular Expression
 */
char_digit.one_or_more()
"#,
            )
            .unwrap()
            .to_string(),
            "one_or_more(char_digit)"
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * Hex Numbers Regular Expression
 */

// The prefix "0x"
"0x"

// The hex digits
['0'..'9', 'a'..'f'].one_or_more()
"#,
            )
            .unwrap()
            .to_string(),
            "\"0x\"
one_or_more(['0'..'9', 'a'..'f'])"
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * Email Address Validated Regular Expression
 *
 * Ref:
 * https://en.wikipedia.org/wiki/Email_address
 */

// Asserts that the current is the first character
start

// User name
[char_word, '.', '-'].one_or_more()

// Sub-address
('+', [char_word, '-'].one_or_more()).optional()

// The separator
'@'

// Domain name
(
    ['a'..'z', 'A'..'Z', '0'..'9', '-'].one_or_more()
    '.'
).one_or_more()

// Top-level domain
['a'..'z'].at_least(2)

// Asserts that the current is the last character
end
"#,
            )
            .unwrap()
            .to_string(),
            "start
one_or_more([char_word, '.', '-'])
optional(('+', one_or_more([char_word, '-'])))
'@'
one_or_more((one_or_more(['a'..'z', 'A'..'Z', '0'..'9', '-']), '.'))
at_least(['a'..'z'], 2)
end"
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * IPv4 Address Validated Regular Expression
 */
define(num_25x, ("25", ['0'..'5']))
define(num_2xx, ('2', ['0'..'4'], char_digit))
define(num_1xx, ('1', char_digit, char_digit))
define(num_xx, (['1'..'9'], char_digit))
define(num_x, char_digit)
define(ip_num, (num_25x || num_2xx || num_1xx || num_xx || num_x))

start, (ip_num, '.').repeat(3), ip_num, end
"#,
            )
            .unwrap()
            .to_string(),
            r#"start
repeat((("25", ['0'..'5']) || (('2', ['0'..'4'], char_digit) || (('1', char_digit, char_digit) || ((['1'..'9'], char_digit) || char_digit))), '.'), 3)
("25", ['0'..'5']) || (('2', ['0'..'4'], char_digit) || (('1', char_digit, char_digit) || ((['1'..'9'], char_digit) || char_digit))), end"#
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * Simple HTML tag Regular Expression
 */
'<'                                                     // opening tag
name(char_word+, tag_name)                              // tag name
(char_space, char_word+, '=', '"', char_word+, '"')*    // attributes
'>'
char_any+?                                              // text content
'<', '/', tag_name, '>'                                 // closing tag
"#,
            )
            .unwrap()
            .to_string(),
            r#"'<'
name(one_or_more(char_word), tag_name)
zero_or_more((char_space, one_or_more(char_word), '=', '"', one_or_more(char_word), '"'))
'>'
one_or_more_lazy(char_any)
'<', '/', tag_name, '>'"#
        );
    }
}
