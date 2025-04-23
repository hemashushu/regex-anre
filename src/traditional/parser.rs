// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

pub const PARSER_PEEK_TOKEN_MAX_COUNT: usize = 1;

use crate::{
    ast::{
        AnchorAssertionName, BackReference, BoundaryAssertionName, CharRange, CharSet,
        CharSetElement, Expression, FunctionCall, FunctionName, Literal, PresetCharSetName,
        Program, SpecialCharName,
    },
    location::Location,
    peekableiter::PeekableIter,
    AnreError,
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
}

impl Parser<'_> {
    pub fn parse_program(&mut self) -> Result<Program, AnreError> {
        let mut expressions = vec![];

        // there is only one expression in the tradition regular expression
        if self.peek_token(0).is_some() {
            let expression = self.parse_expression()?;
            expressions.push(expression);
        }

        // extract elements from a group if the group
        // contains only one element and the element's type
        // is 'Group'
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

        // in the traditional regular expressions, "groups" are implied
        // on both sides of the "logic or" ("|") operator.

        let mut left = self.parse_consecutive_expression()?;

        // """
        // The || operator has the lowest precedence in a regular expression.
        // If you want to use a disjunction as a part of a bigger pattern,
        // you must group it.
        // """
        //
        // e.g.
        // "ab||cd" == "(ab)||(cd)"
        // "ab||cd" != "a(b||c)d)"
        //
        // ref:
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Disjunction
        while let Some(Token::LogicOr) = self.peek_token(0) {
            self.next_token(); // consume "||"

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

    fn parse_consecutive_expression(&mut self) -> Result<Expression, AnreError> {
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
            return Err(AnreError::MessageWithLocation(
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
                    }
                } else {
                    index -= 1;
                }
            } else {
                index -= 1;
            }
        }

        // escape the group if it contains only one element
        if expressions.len() == 1 {
            let first = expressions.remove(0);
            Ok(first)
        } else {
            // an implied group
            Ok(Expression::Group(expressions))
        }
    }

    fn parse_notations(&mut self) -> Result<Expression, AnreError> {
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
                        args: vec![expression],
                    };
                    expression = Expression::FunctionCall(Box::new(function_call));

                    self.next_token(); // consume notation
                }
                Token::Repetition(repetition, lazy) => {
                    let mut args = vec![];
                    args.push(expression);

                    let name = match repetition {
                        Repetition::Specified(n) => {
                            if *lazy {
                                return Err(AnreError::MessageWithLocation(
                                    "Specified number of repetitions does not support lazy mode, i.e. '{m}?' is not allowed.".to_owned(), self.last_range));
                            }

                            args.push(Expression::Literal(Literal::Number(*n)));
                            FunctionName::Repeat
                        }
                        Repetition::AtLeast(n) => {
                            args.push(Expression::Literal(Literal::Number(*n)));

                            if *lazy {
                                FunctionName::AtLeastLazy
                            } else {
                                FunctionName::AtLeast
                            }
                        }
                        Repetition::Range(m, n) => {
                            if *lazy && m == n {
                                return Err(AnreError::MessageWithLocation(
                                    "Specified number of repetitions does not support lazy mode, i.e. '{m,m}?' is not allowed.".to_owned(), self.last_range));
                            }

                            args.push(Expression::Literal(Literal::Number(*m)));
                            args.push(Expression::Literal(Literal::Number(*n)));

                            if *lazy {
                                FunctionName::RepeatRangeLazy
                            } else {
                                FunctionName::RepeatRange
                            }
                        }
                    };

                    let function_call = FunctionCall {
                        name,
                        // expression: Box::new(expression),
                        args,
                    };
                    expression = Expression::FunctionCall(Box::new(function_call));

                    self.next_token(); // consume notation
                }
                Token::LookAhead | Token::LookAheadNegative => {
                    let name = match token {
                        Token::LookAhead => FunctionName::IsBefore,
                        Token::LookAheadNegative => FunctionName::IsNotBefore,
                        _ => unreachable!(),
                    };

                    self.next_token(); // consume "(?=" or "(?!"
                    let arg0 = self.parse_expression()?;
                    self.next_token(); // consume ")"

                    let function_call = FunctionCall {
                        name,
                        args: vec![expression, arg0],
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

    fn parse_primary_expression(&mut self) -> Result<Expression, AnreError> {
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
            Token::StartAssertion => {
                self.next_token(); // consume '^'
                Expression::AnchorAssertion(AnchorAssertionName::Start)
            }
            Token::EndAssertion => {
                self.next_token(); // consume '$'
                Expression::AnchorAssertion(AnchorAssertionName::End)
            }
            Token::BoundaryAssertion(c) => {
                let ch = *c;
                self.next_token(); // consume boundary assertion

                match ch {
                    'b' => Expression::BoundaryAssertion(BoundaryAssertionName::IsBound),
                    'B' => Expression::BoundaryAssertion(BoundaryAssertionName::IsNotBound),
                    _ => unreachable!(),
                }
            }
            token @ (Token::LookBehind | Token::LookBehindNegative) => {
                let name = match token {
                    Token::LookBehind => FunctionName::IsAfter,
                    Token::LookBehindNegative => FunctionName::IsNotAfter,
                    _ => unreachable!(),
                };

                self.next_token(); // consume "(?<=" or "(?<!"
                let arg0 = self.parse_expression()?;
                self.next_token(); // consume ")"

                let expression = self.parse_expression()?;

                let function_call = FunctionCall {
                    name,
                    args: vec![expression, arg0],
                };
                Expression::FunctionCall(Box::new(function_call))
            }
            Token::GroupStart | Token::NonCapturing | Token::NamedCapture(_) => {
                self.parse_group()?
            }
            Token::BackReferenceNumber(index_ref) => {
                let index = *index_ref;
                self.next_token(); // consume '\num'
                Expression::BackReference(BackReference::Index(index))
            }
            Token::BackReferenceIdentifier(name_ref) => {
                let name = name_ref.to_owned();
                self.next_token(); // consume '\k<name>'
                Expression::BackReference(BackReference::Name(name))
            }
            _ => {
                let literal = self.parse_literal()?;
                Expression::Literal(literal)
            }
        };

        Ok(expression)
    }

    fn parse_group(&mut self) -> Result<Expression, AnreError> {
        // "(" {expression} ")" ?
        // ---                  -
        // ^                    ^-- to here
        // | current, validated
        //
        // also:
        // - "(?:" {expression} ")"
        // - "(?<...>" {expression} ")"

        // consume "(", "(?:" or "(?<...>"
        let head_token = self.next_token().unwrap();
        let expression = self.parse_expression()?;

        // consume ")"
        self.consume_token(&Token::GroupEnd, "right parenthese \")\"")?;

        let group_expression = match head_token {
            Token::GroupStart => {
                // regex group is equivalent to ANRE indexed capture group
                let function_call = FunctionCall {
                    name: FunctionName::Index,
                    args: vec![expression],
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
                    args: vec![
                        expression,
                        Expression::Literal(Literal::String(name.to_owned())),
                    ],
                };
                Expression::FunctionCall(Box::new(function_call))
            }
            _ => unreachable!(),
        };

        Ok(group_expression)
    }

    fn parse_literal(&mut self) -> Result<Literal, AnreError> {
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
                let preset_charset_name = preset_charset_name_from_char(*preset_charset_name_ref);
                self.next_token(); // consume preset charset
                Literal::PresetCharSet(preset_charset_name)
            }
            Token::Dot => {
                self.next_token(); // consume special char
                Literal::Special(SpecialCharName::CharAny)
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

    fn parse_charset(&mut self) -> Result<CharSet, AnreError> {
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

                    self.next_token(); // consume char range
                    elements.push(CharSetElement::CharRange(char_range));
                }
                Token::PresetCharSet(preset_charset_name_ref) => {
                    // preset char set
                    let preset_charset_name =
                        preset_charset_name_from_char(*preset_charset_name_ref);
                    self.next_token(); // consume preset charset
                    elements.push(CharSetElement::PresetCharSet(preset_charset_name));
                }
                _ => {
                    return Err(AnreError::MessageWithLocation(
                        "Unsupported char set element.".to_owned(),
                        self.last_range,
                    ));
                }
            }
        }

        self.consume_token(&Token::CharSetEnd, "right bracket \"]\"")?;

        let charset = CharSet {
            negative: matches!(head_token, Token::CharSetStartNegative),
            elements,
        };

        Ok(charset)
    }
}

fn preset_charset_name_from_char(name_char: char) -> PresetCharSetName {
    match name_char {
        'w' => PresetCharSetName::CharWord,
        'W' => PresetCharSetName::CharNotWord,
        's' => PresetCharSetName::CharSpace,
        'S' => PresetCharSetName::CharNotSpace,
        'd' => PresetCharSetName::CharDigit,
        'D' => PresetCharSetName::CharNotDigit,
        _ => unreachable!(),
    }
}

pub fn parse_from_str(s: &str) -> Result<Program, AnreError> {
    let tokens = lex_from_str(s)?;
    let mut token_iter = tokens.into_iter();
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

    #[test]
    fn test_parse_literal_charset() {
        let program = parse_from_str(r#"[a0-9\w]"#).unwrap();

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
            parse_from_str(r#"[^a-z\s]"#,).unwrap().to_string(),
            r#"!['a'..'z', char_space]"#
        );
    }

    #[test]
    fn test_parse_expression_notations() {
        assert_eq!(
            parse_from_str(r#"a?b+c*x??y+?z*?"#,).unwrap().to_string(),
            r#"optional('a')
one_or_more('b')
zero_or_more('c')
optional_lazy('x')
one_or_more_lazy('y')
zero_or_more_lazy('z')"#
        );

        assert_eq!(
            parse_from_str(r#"a{3}b{5,7}c{11,}y{5,7}?z{11,}?"#,)
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
            parse_from_str(r#"a{3}?"#,),
            Err(AnreError::MessageWithLocation(_, _))
        ));

        // err: '{m,m}?' is not allowed
        assert!(matches!(
            parse_from_str(r#"a{3,3}?"#,),
            Err(AnreError::MessageWithLocation(_, _))
        ));
    }

    #[test]
    fn test_parse_expression_logic_or() {
        {
            let program = parse_from_str(r#"a|b"#).unwrap();

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

        {
            let program = parse_from_str(r#"a|b|c"#).unwrap();

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

        assert_eq!(
            parse_from_str(r#"\d+|[\w-]+"#,).unwrap().to_string(),
            r#"one_or_more(char_digit) || one_or_more([char_word, '-'])"#
        );
    }

    #[test]
    fn test_parse_expression_group() {
        // the group of regex is captured by default
        assert_eq!(
            parse_from_str(r#"(foo\d)(b(bar\d))$"#,)
                .unwrap()
                .to_string(),
            r#"index(("foo", char_digit))
index(('b', index(("bar", char_digit))))
end"#
        );

        // non-capturing
        assert_eq!(
            parse_from_str(r#"(?:foo\d)(?:b(?:bar\d))$"#,)
                .unwrap()
                .to_string(),
            r#"("foo", char_digit), ('b', ("bar", char_digit)), end"#
        );

        assert_eq!(
            parse_from_str(r#"(?:foo\d){3}(?:b(?:bar){5})$"#,)
                .unwrap()
                .to_string(),
            r#"repeat(("foo", char_digit), 3)
('b', repeat("bar", 5)), end"#
        );

        assert_eq!(
            parse_from_str(r#"a|(?:b|c)"#,).unwrap().to_string(),
            r#"'a' || ('b' || 'c')"#
        );

        assert_eq!(
            parse_from_str(r#"(?:a|b)|c"#,).unwrap().to_string(),
            r#"('a' || 'b') || 'c'"#
        );

        // the implied groups when encounter the logic or operator '|'
        assert_eq!(
            parse_from_str(r#"a\w|b\d"#,).unwrap().to_string(),
            r#"('a', char_word) || ('b', char_digit)"#
        );

        // extract elements from the top group
        assert_eq!(
            parse_from_str(r#"(?:(?:a\db))"#,).unwrap().to_string(),
            r#"'a', char_digit, 'b'"#
        );
    }

    #[test]
    fn test_parse_expression_anchor_and_boundary_assertions() {
        assert_eq!(
            parse_from_str(r#"^ab\bcd\Bef$"#,).unwrap().to_string(),
            r#"start, "ab", is_bound, "cd", is_not_bound, "ef", end"#
        );
    }

    #[test]
    fn test_parse_expression_named_captured_group_and_back_reference() {
        assert_eq!(
            parse_from_str(r#"(?<tag>\w+).+\k<tag>\1"#,)
                .unwrap()
                .to_string(),
            r#"name(one_or_more(char_word), "tag")
one_or_more(char_any)
tag, ^1"#
        );
    }

    #[test]
    fn test_parse_expression_lookaround_assertion() {
        assert_eq!(
            parse_from_str(r#"(?<=a)b"#,).unwrap().to_string(),
            r#"is_after('b', 'a')"#
        );

        assert_eq!(
            parse_from_str(r#"(?<!a)b"#,).unwrap().to_string(),
            r#"is_not_after('b', 'a')"#
        );

        assert_eq!(
            parse_from_str(r#"a(?=b)"#,).unwrap().to_string(),
            r#"is_before('a', 'b')"#
        );

        assert_eq!(
            parse_from_str(r#"a(?!b)"#,).unwrap().to_string(),
            r#"is_not_before('a', 'b')"#
        );
    }

    #[test]
    fn test_parse_examples() {
        assert_eq!(
            parse_from_str(r#"\d+"#,).unwrap().to_string(),
            "one_or_more(char_digit)"
        );

        assert_eq!(
            parse_from_str(r#"0x[0-9a-f]+"#,).unwrap().to_string(),
            "\"0x\"
one_or_more(['0'..'9', 'a'..'f'])"
        );

        assert_eq!(
            parse_from_str(r#"^[\w.-]+(\+[\w-]+)?@([a-zA-Z0-9-]+\.)+[a-z]{2,}$"#,)
                .unwrap()
                .to_string(),
            "start
one_or_more([char_word, '.', '-'])
optional(index(('+', one_or_more([char_word, '-']))))
'@'
one_or_more(index((one_or_more(['a'..'z', 'A'..'Z', '0'..'9', '-']), '.')))
at_least(['a'..'z'], 2)
end"
        );

        assert_eq!(
            parse_from_str(
                r#"^((25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)\.){3}(25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)$"#,
            )
            .unwrap()
            .to_string(),
            r#"start
repeat(index((index(("25", ['0'..'5']) || (('2', ['0'..'4'], char_digit) || (('1', char_digit, char_digit) || ((['1'..'9'], char_digit) || char_digit)))), '.')), 3)
index(("25", ['0'..'5']) || (('2', ['0'..'4'], char_digit) || (('1', char_digit, char_digit) || ((['1'..'9'], char_digit) || char_digit))))
end"#
        );

        assert_eq!(
            parse_from_str(r#"<(?<tag_name>\w+)(\s\w+="\w+")*>.+?</\k<tag_name>>"#,)
                .unwrap()
                .to_string(),
            r#"'<'
name(one_or_more(char_word), "tag_name")
zero_or_more(index((char_space, one_or_more(char_word), "="", one_or_more(char_word), '"')))
'>'
one_or_more_lazy(char_any)
"</", tag_name, '>'"#
        );
    }
}
