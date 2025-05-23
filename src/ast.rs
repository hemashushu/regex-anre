// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

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
     * A boundary assertion checks the relative position of characters.
     * For example:
     * - `('a', 'c'.is_after('b'))` always fails because it is
     *   impossible for 'a' and 'b' to both precede 'c'.
     * - Similarly, `('c'.is_before('a'), 'b')` always fails because it is
     *   impossible for 'a' and 'b' to both follow 'c'.
     */
    BoundaryAssertion(BoundaryAssertionName),

    /**
     * The "group" in ANRE differs from the "group" in traditional regular expressions.
     * In ANRE, a "group" is a series of parenthesized patterns that are not captured
     * unless explicitly referenced by the `name` or `index` function.
     * In terms of results, an ANRE "group" is equivalent to a "non-capturing group"
     * in traditional regular expressions.
     *
     * Example:
     *
     * ANRE: `('a', 'b', char_word+)`
     * Equivalent regex: `ab\w+`
     *
     * Groups in ANRE are used to group patterns and modify operator precedence
     * and associativity.
     */
    Group(Vec<Expression>),

    FunctionCall(Box<FunctionCall>),

    /**
     * Represents a disjunction (logical OR) between two expressions.
     * For example, `a|b` matches either 'a' or 'b'.
     * Reference: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Disjunction
     */
    Or(Box<Expression>, Box<Expression>),
}

#[derive(Debug, PartialEq)]
pub struct FunctionCall {
    pub name: FunctionName,
    pub args: Vec<Expression>,
}

#[derive(Debug, PartialEq)]
pub enum Literal {
    Number(usize),
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
    PresetCharSet(PresetCharSetName),
    CharSet(Box<CharSet>), // Only positive charsets are allowed.
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
    CharHex, // ANRE only
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
            PresetCharSetName::CharHex => "char_hex",
        };
        f.write_str(name_str)
    }
}

// 'SpecialCharName' currently contains only the 'char_any' variant.
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

    // Assertions (i.e. "判定")
    IsBefore,    // lookahead
    IsAfter,     // lookbehind
    IsNotBefore, // negative lookahead
    IsNotAfter,  // negative lookbehind

    // Capture/Match
    Name,
    Index,
}
