// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::ops::{Add, BitOr, Mul};

use crate::ast::{Expression, FunctionCallArg, FunctionName, Literal};

pub enum MatchLength {
    Variable,
    Fixed(usize), // length by char (unicode char codepoint)
}

impl Add for MatchLength {
    type Output = MatchLength;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            MatchLength::Variable => MatchLength::Variable,
            MatchLength::Fixed(v0) => match rhs {
                MatchLength::Variable => MatchLength::Variable,
                MatchLength::Fixed(v1) => MatchLength::Fixed(v0 + v1),
            },
        }
    }
}

impl Mul<usize> for MatchLength {
    type Output = MatchLength;

    fn mul(self, rhs: usize) -> Self::Output {
        match self {
            MatchLength::Variable => MatchLength::Variable,
            MatchLength::Fixed(v) => MatchLength::Fixed(v * rhs),
        }
    }
}

impl BitOr for MatchLength {
    type Output = MatchLength;

    fn bitor(self, rhs: Self) -> Self::Output {
        match self {
            MatchLength::Variable => MatchLength::Variable,
            MatchLength::Fixed(v0) => match rhs {
                MatchLength::Variable => MatchLength::Variable,
                MatchLength::Fixed(v1) => {
                    if v0 == v1 {
                        MatchLength::Fixed(v0)
                    } else {
                        MatchLength::Variable
                    }
                }
            },
        }
    }
}

pub fn get_match_length(exp: &Expression) -> MatchLength {
    match exp {
        Expression::Literal(literal) => match literal {
            Literal::Char(_) => MatchLength::Fixed(1),
            Literal::String(s) => MatchLength::Fixed(s.chars().count()),
            Literal::Special(_) => MatchLength::Fixed(1),
            Literal::CharSet(_) => MatchLength::Fixed(1),
            Literal::PresetCharSet(_) => MatchLength::Fixed(1),
        },
        Expression::Identifier(_) => MatchLength::Variable,
        Expression::Assertion(_) => MatchLength::Fixed(0),
        Expression::Group(exps) => exps
            .iter()
            .map(get_match_length)
            .reduce(|acc, item| acc + item)
            .unwrap(),
        Expression::FunctionCall(function_call) => match function_call.name {
            FunctionName::Optional => MatchLength::Variable,
            FunctionName::OneOrMore => MatchLength::Variable,
            FunctionName::ZeroOrMore => MatchLength::Variable,
            FunctionName::Repeat => {
                let base = get_match_length(&function_call.expression);
                let factor = if let FunctionCallArg::Number(f) = &function_call.args[0] {
                    *f
                } else {
                    unreachable!()
                };
                base * factor
            }
            FunctionName::RepeatRange => MatchLength::Variable,
            FunctionName::AtLeast => MatchLength::Variable,
            FunctionName::OptionalLazy => MatchLength::Variable,
            FunctionName::OneOrMoreLazy => MatchLength::Variable,
            FunctionName::ZeroOrMoreLazy => MatchLength::Variable,
            FunctionName::RepeatRangeLazy => MatchLength::Variable,
            FunctionName::AtLeastLazy => MatchLength::Variable,
            FunctionName::IsBefore => get_match_length(&function_call.expression),
            FunctionName::IsAfter => {
                let ref_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };
                get_match_length(&function_call.expression) + get_match_length(ref_exp)
            }
            FunctionName::IsNotBefore => get_match_length(&function_call.expression),
            FunctionName::IsNotAfter => {
                let ref_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };
                get_match_length(&function_call.expression) + get_match_length(ref_exp)
            },
            FunctionName::Name => get_match_length(&function_call.expression),
            FunctionName::Index => get_match_length(&function_call.expression),
        },
        Expression::Or(left_exp, right_exp) => {
            get_match_length(left_exp) | get_match_length(right_exp)
        }
    }
}
