// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub expressions: Vec<Expression>,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    AnchorAssertion(AnchorAssertionName),
    BoundaryAssertion(BoundaryAssertionName),

    /**
     * the "group" of ANREG is different from the "group" of
     * ordinary regular expressions.
     * the "group" of ANREG is just a series of parenthesized patterns
     * that are not captured unless called by the 'name' or 'index' function.
     * e.g.
     * ANREG `('a', 'b', char_word+)` is equivalent to oridinary regex `ab\w+`
     * the "group" of ANREG is used to group patterns and
     * change operator precedence and associativity
     */
    Group(Vec<Expression>),

    FunctionCall(Box<FunctionCall>),

    /**
     * Disjunction
     * https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Disjunction
     */
    Or(Box<Expression>, Box<Expression>),
}

#[derive(Debug, PartialEq)]
pub struct FunctionCall {
    pub name: FunctionName,
    pub expression: Box<Expression>, // the index 0 arg
    pub args: Vec<FunctionCallArg>,
}

#[derive(Debug, PartialEq)]
pub enum FunctionCallArg {
    Number(usize),
    Identifier(String),
    Expression(Box<Expression>),
}

#[derive(Debug, PartialEq)]
pub enum Literal {
    Char(char),
    String(String),
    Special(SpecialCharName),
    CharSet(CharSet),
    PresetCharSet(PresetCharSetName),
}

#[derive(Debug, PartialEq)]
pub struct CharSet {
    pub negative: bool,
    pub elements: Vec<CharSetElement>,
}

#[derive(Debug, PartialEq)]
pub enum CharSetElement {
    Char(char),
    CharRange(CharRange),
    PresetCharSet(PresetCharSetName), // only positive preset charsets are allowed
    CharSet(Box<CharSet>),            // only positive custom charsets are allowed
}

#[derive(Debug, PartialEq)]
pub struct CharRange {
    pub start: char,
    pub end_included: char,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AnchorAssertionName {
    Start,
    End,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BoundaryAssertionName {
    IsBound,
    IsNotBound,
}

impl Display for AnchorAssertionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_str = match self {
            AnchorAssertionName::Start => "start",
            AnchorAssertionName::End => "end",
        };
        f.write_str(name_str)
    }
}

impl Display for BoundaryAssertionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_str = match self {
            BoundaryAssertionName::IsBound => "is_bound",
            BoundaryAssertionName::IsNotBound => "is_not_bound",
        };
        f.write_str(name_str)
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PresetCharSetName {
    CharWord,
    CharNotWord,
    CharDigit,
    CharNotDigit,
    CharSpace,
    CharNotSpace,
}

impl Display for PresetCharSetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_str = match self {
            PresetCharSetName::CharWord => "char_word",
            PresetCharSetName::CharNotWord => "char_not_word",
            PresetCharSetName::CharDigit => "char_digit",
            PresetCharSetName::CharNotDigit => "char_not_digit",
            PresetCharSetName::CharSpace => "char_space",
            PresetCharSetName::CharNotSpace => "char_not_space",
        };
        f.write_str(name_str)
    }
}

// 'special char' currently contains only the 'char_any'.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SpecialCharName {
    CharAny,
}

impl Display for SpecialCharName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialCharName::CharAny => f.write_str("char_any"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FunctionName {
    // Greedy Quantifier
    Optional,
    OneOrMore,
    ZeroOrMore,
    Repeat,
    RepeatRange,
    AtLeast,

    // Lazy Quantifier
    OptionalLazy,
    OneOrMoreLazy,
    ZeroOrMoreLazy,
    RepeatRangeLazy,
    AtLeastLazy,

    // Assertions ("判定")
    IsBefore,    // lookahead
    IsAfter,     // lookbehind
    IsNotBefore, // negative lookahead
    IsNotAfter,  // negative lookbehind

    // Capture/Match
    Name,
    Index,
}

impl Display for FunctionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionName::Optional => f.write_str("optional"),
            FunctionName::OneOrMore => f.write_str("one_or_more"),
            FunctionName::ZeroOrMore => f.write_str("zero_or_more"),
            FunctionName::Repeat => f.write_str("repeat"),
            FunctionName::RepeatRange => f.write_str("repeat_range"),
            FunctionName::AtLeast => f.write_str("at_least"),
            FunctionName::OptionalLazy => f.write_str("optional_lazy"),
            FunctionName::OneOrMoreLazy => f.write_str("one_or_more_lazy"),
            FunctionName::ZeroOrMoreLazy => f.write_str("zero_or_more_lazy"),
            FunctionName::RepeatRangeLazy => f.write_str("repeat_range_lazy"),
            FunctionName::AtLeastLazy => f.write_str("at_least_lazy"),
            FunctionName::IsBefore => f.write_str("is_before"),
            FunctionName::IsAfter => f.write_str("is_after"),
            FunctionName::IsNotBefore => f.write_str("is_not_before"),
            FunctionName::IsNotAfter => f.write_str("is_not_after"),
            FunctionName::Name => f.write_str("name"),
            FunctionName::Index => f.write_str("capture"),
        }
    }
}

impl Display for CharRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'..'{}'", self.start, self.end_included)
    }
}

impl Display for CharSetElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharSetElement::Char(c) => write!(f, "'{}'", c),
            CharSetElement::CharRange(c) => write!(f, "{}", c),
            CharSetElement::PresetCharSet(p) => write!(f, "{}", p),
            CharSetElement::CharSet(c) => write!(f, "{}", c),
        }
    }
}

impl Display for CharSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: Vec<String> = self.elements.iter().map(|e| e.to_string()).collect();
        if self.negative {
            write!(f, "![{}]", s.join(", "))
        } else {
            write!(f, "[{}]", s.join(", "))
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Char(c) => write!(f, "'{}'", c),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::CharSet(c) => write!(f, "{}", c),
            Literal::PresetCharSet(p) => write!(f, "{}", p),
            Literal::Special(s) => write!(f, "{}", s),
        }
    }
}

impl Display for FunctionCallArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionCallArg::Number(i) => write!(f, "{}", i),
            FunctionCallArg::Identifier(s) => write!(f, "{}", s),
            FunctionCallArg::Expression(e) => write!(f, "{}", e),
        }
    }
}

impl Display for FunctionCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.args.is_empty() {
            write!(f, "{}({})", self.name, self.expression)
        } else {
            let s: Vec<String> = self.args.iter().map(|e| e.to_string()).collect();
            write!(f, "{}({}, {})", self.name, self.expression, s.join(", "))
        }
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Literal(e) => write!(f, "{}", e),
            Expression::Identifier(s) => f.write_str(s),
            Expression::AnchorAssertion(e) => write!(f, "{}", e),
            Expression::BoundaryAssertion(e) => write!(f, "{}", e),
            Expression::Group(expressions) => {
                let lines: Vec<String> = expressions.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", lines.join(", "))
            }
            Expression::FunctionCall(e) => write!(f, "{}", e),
            Expression::Or(left, right) => write!(f, "{} || {}", left, right),
        }
    }
}

impl Display for Program {
    // for debug
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut exp_strings: Vec<String> = vec![];
        for (idx, expression) in self.expressions.iter().enumerate() {
            match expression {
                Expression::FunctionCall(function_call) => {
                    if idx != 0 {
                        // replace the last ',' with '\n'
                        exp_strings.pop();
                        exp_strings.push("\n".to_owned());
                    }
                    exp_strings.push(function_call.to_string());
                    exp_strings.push("\n".to_owned());
                }
                _ => {
                    exp_strings.push(expression.to_string());
                    exp_strings.push(", ".to_owned());
                }
            }
        }

        if !exp_strings.is_empty() {
            exp_strings.pop(); // remove the last ',' or '\n'
            write!(f, "{}", exp_strings.join(""))
        } else {
            f.write_str("")
        }
    }
}
