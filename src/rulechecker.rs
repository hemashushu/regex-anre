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
        Expression::BackReference(_) => MatchLength::Variable,
        Expression::AnchorAssertion(_) => MatchLength::Fixed(0),
        Expression::BoundaryAssertion(_) => MatchLength::Fixed(0),
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
                let base_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };

                let factor = if let FunctionCallArg::Number(f) = &function_call.args[1] {
                    *f
                } else {
                    unreachable!()
                };

                get_match_length(base_exp) * factor
            }
            FunctionName::RepeatRange => MatchLength::Variable,
            FunctionName::AtLeast => MatchLength::Variable,
            FunctionName::OptionalLazy => MatchLength::Variable,
            FunctionName::OneOrMoreLazy => MatchLength::Variable,
            FunctionName::ZeroOrMoreLazy => MatchLength::Variable,
            FunctionName::RepeatRangeLazy => MatchLength::Variable,
            FunctionName::AtLeastLazy => MatchLength::Variable,
            FunctionName::IsBefore => {
                let base_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };

                get_match_length(base_exp)
            }
            FunctionName::IsAfter => {
                let base_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };

                let ref_exp = if let FunctionCallArg::Expression(e) = &function_call.args[1] {
                    e
                } else {
                    unreachable!()
                };

                get_match_length(base_exp) + get_match_length(ref_exp)
            }
            FunctionName::IsNotBefore => {
                let base_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };

                get_match_length(base_exp)
            }
            FunctionName::IsNotAfter => {
                let base_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };

                let ref_exp = if let FunctionCallArg::Expression(e) = &function_call.args[1] {
                    e
                } else {
                    unreachable!()
                };

                get_match_length(base_exp) + get_match_length(ref_exp)
            }
            FunctionName::Name => {
                let base_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };

                get_match_length(base_exp)
            }
            FunctionName::Index => {
                let base_exp = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
                    e
                } else {
                    unreachable!()
                };

                get_match_length(base_exp)
            }
        },
        Expression::Or(left_exp, right_exp) => {
            get_match_length(left_exp) | get_match_length(right_exp)
        }
    }
}
