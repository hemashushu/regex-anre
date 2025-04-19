// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use std::fmt::Display;

use crate::ast::{
    CharRange, CharSet, CharSetElement, Expression, FunctionCall, FunctionCallArg, FunctionName,
    Literal, Program,
};

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
            FunctionName::Index => f.write_str("index"),
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
        let s: Vec<String> = self.args.iter().map(|e| e.to_string()).collect();
        write!(f, "{}({})", self.name, s.join(", "))
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Literal(e) => write!(f, "{}", e),
            Expression::BackReference(e) => write!(f, "{}", e),
            Expression::AnchorAssertion(e) => write!(f, "{}", e),
            Expression::BoundaryAssertion(e) => write!(f, "{}", e),
            Expression::Group(expressions) => {
                let lines: Vec<String> = expressions.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", lines.join(", "))
            }
            Expression::FunctionCall(e) => write!(f, "{}", e),
            Expression::Or(left, right) => {
                if matches!(left.as_ref(), Expression::Or(_, _)) {
                    if matches!(right.as_ref(), Expression::Or(_, _)) {
                        write!(f, "({}) || ({})", left, right)
                    } else {
                        write!(f, "({}) || {}", left, right)
                    }
                } else if matches!(right.as_ref(), Expression::Or(_, _)) {
                    write!(f, "{} || ({})", left, right)
                } else {
                    write!(f, "{} || {}", left, right)
                }
            }
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
