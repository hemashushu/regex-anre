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
    BackReference(BackReference),
    AnchorAssertion(AnchorAssertionName),

    /**
     * `('a','c'.is_after('b'))` always fails because it is
     * NOT possible to be both 'a' and 'b' before 'c'.
     * in the same way,
     * `('c'.is_before('a'), 'b')` always fails because it is
     * impossible to be both 'a' and 'b' after 'c'.
     * */
    BoundaryAssertion(BoundaryAssertionName),

    /**
     * the "group" of ANRE is different from the "group" of
     * ordinary regular expressions.
     * the "group" of ANRE is just a series of parenthesized patterns
     * that are not captured unless called by the 'name' or 'index' function.
     * in terms of results, the "group" of ANRE is equivalent to the
     * "non-capturing group" of ordinary regular expressions.
     * e.g.
     * ANRE `('a', 'b', char_word+)` is equivalent to oridinary regex `ab\w+`
     * the "group" of ANRE is used to group patterns and
     * change operator precedence and associativity
     * */
    Group(Vec<Expression>),

    FunctionCall(Box<FunctionCall>),

    /**
     * Disjunction
     * https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Disjunction
     * */
    Or(Box<Expression>, Box<Expression>),
}

#[derive(Debug, PartialEq)]
pub struct FunctionCall {
    pub name: FunctionName,
    // pub expression: Box<Expression>, // the index 0 arg
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

#[derive(Debug, PartialEq)]
pub enum BackReference {
    Index(usize),
    Name(String),
}

impl Display for BackReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackReference::Index(index) => write!(f, "^{}", index),
            BackReference::Name(name) => f.write_str(name),
        }
    }
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
