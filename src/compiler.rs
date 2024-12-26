// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    ast::{
        AnchorAssertionName, BackReference, BoundaryAssertionName, CharRange, CharSet,
        CharSetElement, Expression, FunctionCall, FunctionCallArg, FunctionName, Literal,
        PresetCharSetName, Program,
    },
    route::{Line, Route},
    rulechecker::{get_match_length, MatchLength},
    transition::{
        add_char, add_preset_digit, add_preset_space, add_preset_word, add_range,
        AnchorAssertionTransition, BackReferenceTransition, BoundaryAssertionTransition,
        CaptureEndTransition, CaptureStartTransition, CharSetItem, CharSetTransition,
        CharTransition, CounterCheckTransition, CounterIncTransition, CounterResetTransition,
        CounterSaveTransition, JumpTransition, LookAheadAssertionTransition,
        LookBehindAssertionTransition, RepetitionTransition, RepetitionType, SpecialCharTransition,
        StringTransition, Transition,
    },
    AnreError,
};

pub fn compile_from_regex(s: &str) -> Result<Route, AnreError> {
    let program = crate::tradition::parse_from_str(s)?;
    compile(&program)
}

pub fn compile_from_anre(s: &str) -> Result<Route, AnreError> {
    let program = crate::anre::parse_from_str(s)?;
    compile(&program)
}

pub fn compile(program: &Program) -> Result<Route, AnreError> {
    let mut route = Route::new();
    let mut compiler = Compiler::new(program, &mut route);
    compiler.compile()?;

    Ok(route)
}

pub struct Compiler<'a> {
    // AST
    program: &'a Program,

    // the compilation target
    route: &'a mut Route,

    // index of the current line
    current_line_index: usize,
}

impl<'a> Compiler<'a> {
    fn new(program: &'a Program, route: &'a mut Route) -> Self {
        let current_line_index = route.new_line();
        Compiler {
            program,
            route,
            current_line_index,
        }
    }

    fn get_current_line_ref_mut(&mut self) -> &mut Line {
        &mut self.route.lines[self.current_line_index]
    }

    fn compile(&mut self) -> Result<(), AnreError> {
        self.emit_program(self.program)
    }

    fn emit_program(&mut self, program: &Program) -> Result<(), AnreError> {
        // the `Program` node is actually a `Group` which omits the parentheses,
        // in addition, there may be the 'start' and 'end' assertions.

        // create the first (index 0) capture group to represent the program itself
        let capture_group_index = self.route.new_capture_group(None);

        let expressions = &program.expressions;

        let mut fixed_start_position = false;
        let mut ports = vec![];

        for (expression_index, expression) in expressions.iter().enumerate() {
            if matches!(
                expression,
                Expression::AnchorAssertion(AnchorAssertionName::Start)
            ) {
                if expression_index != 0 {
                    return Err(AnreError::SyntaxIncorrect(
                        "The assertion \"start\" can only be present at the beginning of expression."
                            .to_owned(),
                    ));
                }

                fixed_start_position = true;
                ports.push(self.emit_anchor_assertion(&AnchorAssertionName::Start)?);
            } else if matches!(
                expression,
                Expression::AnchorAssertion(AnchorAssertionName::End)
            ) {
                if expression_index != expressions.len() - 1 {
                    return Err(AnreError::SyntaxIncorrect(
                        "The assertion \"end\" can only be present at the end of expression."
                            .to_owned(),
                    ));
                }

                ports.push(self.emit_anchor_assertion(&AnchorAssertionName::End)?);
            } else {
                ports.push(self.emit_expression(expression)?);
            }
        }

        let program_port = if ports.is_empty() {
            // empty expression
            self.emit_empty()?
        } else if ports.len() == 1 {
            // single expression
            ports.pop().unwrap()
        } else {
            // multiple expression
            let line = self.get_current_line_ref_mut();
            for idx in 0..(ports.len() - 1) {
                let previous_out_node_index = ports[idx].out_node_index;
                let next_in_node_index = ports[idx + 1].in_node_index;
                let transition = Transition::Jump(JumpTransition);
                line.append_transition(previous_out_node_index, next_in_node_index, transition);
            }

            Port::new(
                ports.first().unwrap().in_node_index,
                ports.last().unwrap().out_node_index,
            )
        };

        //   capture start     box       capture end
        //        trans    /-----------\    trans
        //  ==o==---------==o in  out o==--------==o==
        //   in            \-----------/           out

        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();

        let capture_start_transition = CaptureStartTransition::new(capture_group_index);
        let capture_end_transition = CaptureEndTransition::new(capture_group_index);

        line.append_transition(
            in_node_index,
            program_port.in_node_index,
            Transition::CaptureStart(capture_start_transition),
        );

        line.append_transition(
            program_port.out_node_index,
            out_node_index,
            Transition::CaptureEnd(capture_end_transition),
        );

        // update the program ports and properties
        line.start_node_index = in_node_index;
        line.end_node_index = out_node_index;
        line.fixed_start_position = fixed_start_position;

        Ok(())
    }

    fn emit_expression(&mut self, expression: &Expression) -> Result<Port, AnreError> {
        let result = match expression {
            Expression::Literal(literal) => self.emit_literal(literal)?,
            Expression::BackReference(back_reference) => self.emit_backreference(back_reference)?,
            Expression::AnchorAssertion(name) => {
                match name {
                    AnchorAssertionName::Start => {
                        return Err(AnreError::SyntaxIncorrect(
                            "The assertion \"start\" can only exist at the beginning of an expression.".to_owned()));
                    }
                    AnchorAssertionName::End => {
                        return Err(AnreError::SyntaxIncorrect(
                            "The assertion \"end\" can only exist at the end of an expression."
                                .to_owned(),
                        ));
                    }
                }
            }
            Expression::BoundaryAssertion(name) => self.emit_boundary_assertion(name)?,
            Expression::Group(expressions) => self.emit_group(expressions)?,
            Expression::FunctionCall(function_call) => self.emit_function_call(function_call)?,
            Expression::Or(left, right) => self.emit_logic_or(left, right)?,
        };

        Ok(result)
    }

    fn emit_group(&mut self, expressions: &[Expression]) -> Result<Port, AnreError> {
        /*
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
         */

        // connecting two or more boxies by 'jump transition'
        //
        //     prev         jump        next
        //  /-----------\   trans    /-----------\
        // ==o in  out o==----------==o in  out o==
        //  \-----------/            \-----------/

        let mut ports = vec![];
        for expression in expressions {
            ports.push(self.emit_expression(expression)?);
        }

        let port = if ports.is_empty() {
            // empty expression
            self.emit_empty()?
        } else if ports.len() == 1 {
            // single expression.
            // maybe a group also, so return the underlay port directly
            // to eliminates the nested group, e.g. '(((...)))'.
            ports.pop().unwrap()
        } else {
            // multiple expressions
            let line = self.get_current_line_ref_mut();
            for idx in 0..(ports.len() - 1) {
                let current_out_state_index = ports[idx].out_node_index;
                let next_in_state_index = ports[idx + 1].in_node_index;
                let transition = Transition::Jump(JumpTransition);
                line.append_transition(current_out_state_index, next_in_state_index, transition);
            }

            Port::new(
                ports.first().unwrap().in_node_index,
                ports.last().unwrap().out_node_index,
            )
        };

        Ok(port)
    }

    fn emit_logic_or(&mut self, left: &Expression, right: &Expression) -> Result<Port, AnreError> {
        //                    left
        //         jump   /-----------\   jump
        //      /--------==o in  out o==--------\
        //  in  |         \-----------/         |  out
        // ==o--|                               |--o==
        //      |             right             |
        //      |         /-----------\         |
        //      \--------==o in  out o==--------/
        //         jump   \-----------/   jump

        let left_port = self.emit_expression(left)?;
        let right_port = self.emit_expression(right)?;

        let line = self.get_current_line_ref_mut();

        let in_state_index = line.new_node();
        let out_state_index = line.new_node();

        line.append_transition(
            in_state_index,
            left_port.in_node_index,
            Transition::Jump(JumpTransition),
        );

        line.append_transition(
            in_state_index,
            right_port.in_node_index,
            Transition::Jump(JumpTransition),
        );

        line.append_transition(
            left_port.out_node_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        line.append_transition(
            right_port.out_node_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_function_call(&mut self, function_call: &FunctionCall) -> Result<Port, AnreError> {
        let expression = if let FunctionCallArg::Expression(e) = &function_call.args[0] {
            e
        } else {
            unreachable!()
        };

        let args = &function_call.args[1..];

        let is_lazy = matches!(
            function_call.name,
            FunctionName::OptionalLazy
                | FunctionName::OneOrMoreLazy
                | FunctionName::ZeroOrMoreLazy
                | FunctionName::RepeatRangeLazy
                | FunctionName::AtLeastLazy
        );

        match &function_call.name {
            // Quantifier
            FunctionName::Optional | FunctionName::OptionalLazy => {
                self.emit_optional(expression, is_lazy)
            }
            FunctionName::OneOrMore | FunctionName::OneOrMoreLazy => {
                // {1,MAX}
                self.emit_repeat_range(expression, 1, usize::MAX, is_lazy)
            }
            FunctionName::ZeroOrMore | FunctionName::ZeroOrMoreLazy => {
                // {0,MAX} == optional + one_or_more
                let port = self.emit_repeat_range(expression, 1, usize::MAX, is_lazy)?;
                self.continue_emit_optional(port, is_lazy)
            }
            FunctionName::Repeat => {
                let times = if let FunctionCallArg::Number(n) = &args[0] {
                    *n
                } else {
                    unreachable!()
                };

                if times == 0 {
                    // {0}
                    // return an empty transition
                    self.emit_empty()
                } else if times == 1 {
                    // {1}
                    // return the expression without repetition
                    self.emit_expression(expression)
                } else {
                    // {m}
                    // repeat specified
                    self.emit_repeat_specified(expression, times)
                }
            }
            FunctionName::RepeatRange | FunctionName::RepeatRangeLazy => {
                let from = if let FunctionCallArg::Number(n) = &args[0] {
                    *n
                } else {
                    unreachable!()
                };

                let to = if let FunctionCallArg::Number(n) = &args[1] {
                    *n
                } else {
                    unreachable!()
                };

                if from > to {
                    return Err(AnreError::SyntaxIncorrect(
                        "Repeated range values should be from small to large.".to_owned(),
                    ));
                }

                if from == 0 {
                    if to == 0 {
                        // {0,0}
                        // return an empty transition
                        self.emit_empty()
                    } else if to == 1 {
                        // {0,1}
                        // optional
                        self.emit_optional(expression, is_lazy)
                    } else {
                        // {0,m}
                        // optional + range
                        let port = self.emit_repeat_range(expression, 1, to, is_lazy)?;
                        self.continue_emit_optional(port, is_lazy)
                    }
                } else if to == 1 {
                    // {1,1}
                    // return the expression without repetition
                    self.emit_expression(expression)
                } else if from == to {
                    // {m,m}
                    // repeat specified
                    self.emit_repeat_specified(expression, from)
                } else {
                    // {m,n}
                    // repeat range
                    self.emit_repeat_range(expression, from, to, is_lazy)
                }
            }
            FunctionName::AtLeast | FunctionName::AtLeastLazy => {
                let from = if let FunctionCallArg::Number(n) = &args[0] {
                    *n
                } else {
                    unreachable!()
                };

                if from == 0 {
                    // {0,MAX} == optional + one_or_more
                    let port = self.emit_repeat_range(expression, 1, usize::MAX, is_lazy)?;
                    self.continue_emit_optional(port, is_lazy)
                } else {
                    // {m,MAX}
                    // repeat range
                    self.emit_repeat_range(expression, from, usize::MAX, is_lazy)
                }
            }

            // Assertions
            FunctionName::IsBefore | FunctionName::IsNotBefore => {
                // lookahead assertion

                if args.len() != 1 {
                    return Err(AnreError::SyntaxIncorrect(
                        "Expect an expression for the argument of lookahead assertion.".to_owned(),
                    ));
                }

                let next_expression = if let FunctionCallArg::Expression(e) = &args[0] {
                    e
                } else {
                    unreachable!()
                };

                let negative = function_call.name == FunctionName::IsNotBefore;
                self.emit_lookahead_assertion(expression, next_expression, negative)
            }
            FunctionName::IsAfter | FunctionName::IsNotAfter => {
                // lookbehind assertion

                if args.len() != 1 {
                    return Err(AnreError::SyntaxIncorrect(
                        "Expect an expression for the argument of lookbehind assertion.".to_owned(),
                    ));
                }

                let previous_expression = if let FunctionCallArg::Expression(e) = &args[0] {
                    e
                } else {
                    unreachable!()
                };

                let negative = function_call.name == FunctionName::IsNotAfter;
                self.emit_lookbehind_assertion(expression, previous_expression, negative)
            }

            // Capture
            FunctionName::Name => self.emit_capture_group_by_name(expression, args),
            FunctionName::Index => self.emit_capture_group_by_index(expression),
        }
    }

    fn emit_empty(&mut self) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();

        line.append_transition(
            in_node_index,
            out_node_index,
            Transition::Jump(JumpTransition),
        );
        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_literal(&mut self, literal: &Literal) -> Result<Port, AnreError> {
        let port = match literal {
            Literal::Char(character) => self.emit_literal_char(*character)?,
            Literal::String(s) => self.emit_literal_string(s)?,
            Literal::CharSet(charset) => self.emit_literal_charset(charset)?,
            Literal::PresetCharSet(name) => self.emit_literal_preset_charset(name)?,
            Literal::Special(_) => self.emit_literal_special_char()?,
        };

        Ok(port)
    }

    fn emit_literal_char(&mut self, character: char) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();
        let transition = Transition::Char(CharTransition::new(character));

        line.append_transition(in_node_index, out_node_index, transition);
        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_literal_special_char(&mut self) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_out_index = line.new_node();
        let out_out_index = line.new_node();
        let transition = Transition::SpecialChar(SpecialCharTransition);

        line.append_transition(in_out_index, out_out_index, transition);
        Ok(Port::new(in_out_index, out_out_index))
    }

    fn emit_literal_string(&mut self, s: &str) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();
        let transition = Transition::String(StringTransition::new(s));

        line.append_transition(in_node_index, out_node_index, transition);
        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_literal_preset_charset(&mut self, name: &PresetCharSetName) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();

        let charset_transition = match name {
            PresetCharSetName::CharWord => CharSetTransition::new_preset_word(),
            PresetCharSetName::CharNotWord => CharSetTransition::new_preset_not_word(),
            PresetCharSetName::CharSpace => CharSetTransition::new_preset_space(),
            PresetCharSetName::CharNotSpace => CharSetTransition::new_preset_not_space(),
            PresetCharSetName::CharDigit => CharSetTransition::new_preset_digit(),
            PresetCharSetName::CharNotDigit => CharSetTransition::new_preset_not_digit(),
        };

        let transition = Transition::CharSet(charset_transition);
        line.append_transition(in_node_index, out_node_index, transition);
        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_literal_charset(&mut self, charset: &CharSet) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();

        let mut items: Vec<CharSetItem> = vec![];
        append_charset(charset, &mut items)?;

        let transition = Transition::CharSet(CharSetTransition::new(items, charset.negative));
        line.append_transition(in_node_index, out_node_index, transition);
        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_anchor_assertion(&mut self, name: &AnchorAssertionName) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();
        let transition = Transition::AnchorAssertion(AnchorAssertionTransition::new(*name));

        line.append_transition(in_node_index, out_node_index, transition);
        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_boundary_assertion(&mut self, name: &BoundaryAssertionName) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();
        let transition = Transition::BoundaryAssertion(BoundaryAssertionTransition::new(*name));

        line.append_transition(in_node_index, out_node_index, transition);
        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_backreference(&mut self, back_reference: &BackReference) -> Result<Port, AnreError> {
        match back_reference {
            BackReference::Index(index) => self.emit_backreference_by_index(*index),
            BackReference::Name(name) => self.emit_backreference_by_name(name),
        }
    }

    fn emit_backreference_by_index(
        &mut self,
        capture_group_index: usize,
    ) -> Result<Port, AnreError> {
        if capture_group_index >= self.route.capture_groups.len() {
            return Err(AnreError::SyntaxIncorrect(format!(
                "The group index ({}) of back-reference is out of range, the max index should be: {}.",
                capture_group_index, self.route.capture_groups.len() - 1
            )));
        }

        self.continue_emit_backreference(capture_group_index)
    }

    fn emit_backreference_by_name(&mut self, name: &str) -> Result<Port, AnreError> {
        let capture_group_index_option = self.route.get_capture_group_index_by_name(name);
        let capture_group_index = if let Some(i) = capture_group_index_option {
            i
        } else {
            return Err(AnreError::SyntaxIncorrect(format!(
                "Cannot find the match with name: \"{}\".",
                name
            )));
        };

        self.continue_emit_backreference(capture_group_index)
    }

    fn continue_emit_backreference(
        &mut self,
        capture_group_index: usize,
    ) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();
        let transition =
            Transition::BackReference(BackReferenceTransition::new(capture_group_index));

        line.append_transition(in_node_index, out_node_index, transition);
        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_capture_group_by_name(
        &mut self,
        expression: &Expression,
        args: &[FunctionCallArg],
    ) -> Result<Port, AnreError> {
        let name = if let FunctionCallArg::Identifier(s) = &args[0] {
            s.to_owned()
        } else {
            unreachable!();
        };

        self.continue_emit_capture_group(expression, Some(name))
    }

    fn emit_capture_group_by_index(&mut self, expression: &Expression) -> Result<Port, AnreError> {
        self.continue_emit_capture_group(expression, None)
    }

    fn continue_emit_capture_group(
        &mut self,
        expression: &Expression,
        name_option: Option<String>,
    ) -> Result<Port, AnreError> {
        let capture_group_index = self.route.new_capture_group(name_option);
        let port = self.emit_expression(expression)?;

        //   capture start      box      capture end
        //        trans    /-----------\    trans
        //  ==o==---------==o in  out o==--------==o==
        //   in            \-----------/           out

        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();
        let capture_start_transition = CaptureStartTransition::new(capture_group_index);
        let capture_end_transition = CaptureEndTransition::new(capture_group_index);

        line.append_transition(
            in_node_index,
            port.in_node_index,
            Transition::CaptureStart(capture_start_transition),
        );

        line.append_transition(
            port.out_node_index,
            out_node_index,
            Transition::CaptureEnd(capture_end_transition),
        );

        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_optional(&mut self, expression: &Expression, is_lazy: bool) -> Result<Port, AnreError> {
        // greedy optional
        //
        //                    box
        //   in     jmp  /-----------\  jmp
        //  ==o|o==-----==o in  out o==---==o==
        //     |o==\     \-----------/      ^ out
        //         |                        |
        //         \------------------------/
        //                  jump trans

        // lazy optional
        //                  jump trans
        //         /------------------------\
        //         |                        |
        //     |o==/     /-----------\      v out
        //  ==o|o==-----==o in  out o==---==o==
        //   in     jmp  \-----------/  jmp
        //                    box

        let port = self.emit_expression(expression)?;
        self.continue_emit_optional(port, is_lazy)
    }

    fn continue_emit_optional(&mut self, port: Port, is_lazy: bool) -> Result<Port, AnreError> {
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();

        if is_lazy {
            line.append_transition(
                in_node_index,
                out_node_index,
                Transition::Jump(JumpTransition),
            );
        }

        line.append_transition(
            in_node_index,
            port.in_node_index,
            Transition::Jump(JumpTransition),
        );

        line.append_transition(
            port.out_node_index,
            out_node_index,
            Transition::Jump(JumpTransition),
        );

        if !is_lazy {
            line.append_transition(
                in_node_index,
                out_node_index,
                Transition::Jump(JumpTransition),
            );
        }

        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_repeat_specified(
        &mut self,
        expression: &Expression,
        times: usize,
    ) -> Result<Port, AnreError> {
        assert!(times > 1);
        self.continue_emit_repetition(expression, RepetitionType::Specified(times), true)
    }

    fn emit_repeat_range(
        &mut self,
        expression: &Expression,
        from: usize,
        to: usize,
        is_lazy: bool,
    ) -> Result<Port, AnreError> {
        assert!(from > 0 && to > 1 && to > from);
        self.continue_emit_repetition(expression, RepetitionType::Range(from, to), is_lazy)
    }

    fn continue_emit_repetition(
        &mut self,
        expression: &Expression,
        repetition_type: RepetitionType,
        is_lazy: bool,
    ) -> Result<Port, AnreError> {
        // lazy repetition
        //
        //                     counter               counter
        //                   | save                | restore & inc
        //                   | trans               | trans
        //   in        left  v       /-----------\ v        right     out
        //  ==o==------==o==--------==o in  out o==-------==o|o==---==o==
        //       ^ cnter ^           \-----------/           |o-\  ^ counter
        //       | reset |                                      |  | check
        //         trans \--------------------------------------/    trans
        //                         repetition trans
        //
        // greedy repetion
        //
        //                         repetition trans
        //               /--------------------------------------\
        //               |     counter               counter    |
        //               |   | save                | restore &  |
        //               |   | trans               | inc        |
        //   in          v   v       /-----------\ v trans      |
        //  ==o==------==o==--------==o in  out o==-------==o|o=/     out
        //       ^ cnter left        \-----------/     right |o==---==o==
        //       | reset                                        ^
        //         trans                          counter check |
        //                                                trans

        let port = self.emit_expression(expression)?;
        // let counter_index = self.route.new_counter();
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let left_node_index = line.new_node();
        let right_node_index = line.new_node();

        line.append_transition(
            in_node_index,
            left_node_index,
            Transition::CounterReset(CounterResetTransition),
        );

        line.append_transition(
            left_node_index,
            port.in_node_index,
            Transition::CounterSave(CounterSaveTransition),
        );

        line.append_transition(
            port.out_node_index,
            right_node_index,
            Transition::CounterInc(CounterIncTransition),
        );

        let out_node_index = line.new_node();

        if is_lazy {
            //            let out_node_index = line.new_node();

            line.append_transition(
                right_node_index,
                out_node_index,
                Transition::CounterCheck(CounterCheckTransition::new(
                    // counter_index,
                    repetition_type.clone(),
                )),
            );

            line.append_transition(
                right_node_index,
                left_node_index,
                Transition::Repetition(RepetitionTransition::new(
                    // counter_index,
                    repetition_type,
                )),
            );

            // Ok(Port::new(in_node_index, out_node_index))
        } else {
            line.append_transition(
                right_node_index,
                left_node_index,
                Transition::Repetition(RepetitionTransition::new(
                    //counter_index,
                    repetition_type.clone(),
                )),
            );

            line.append_transition(
                right_node_index,
                out_node_index,
                Transition::CounterCheck(CounterCheckTransition::new(
                    // counter_index,
                    repetition_type,
                )),
            );
        }

        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_lookahead_assertion(
        &mut self,
        current_expression: &Expression,
        next_expression: &Expression,
        negative: bool,
    ) -> Result<Port, AnreError> {
        // * is_before(A, B), A.is_before(B), A(?=B)
        // * is_not_before(A, B), A.is_not_before(B), A(?!B)

        //                           | lookahead
        //  in       /-----------\   v trans
        // ==o==----==o in  out o==---==o==
        //      jump \-----------/      out

        let port = self.emit_expression(current_expression)?;

        // 1. save the current line index
        // 2. create new line
        let saved_line_index = self.current_line_index;
        let sub_line_index = self.route.new_line();

        // 3. switch to the new line
        self.current_line_index = sub_line_index;

        {
            let sub_port = self.emit_expression(next_expression)?;
            let sub_line = self.get_current_line_ref_mut();

            // save the sub-program ports
            sub_line.start_node_index = sub_port.in_node_index;
            sub_line.end_node_index = sub_port.out_node_index;
            sub_line.fixed_start_position = true;
        }

        // restore to the previous line
        self.current_line_index = saved_line_index;

        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();

        line.append_transition(
            in_node_index,
            port.in_node_index,
            Transition::Jump(JumpTransition),
        );

        line.append_transition(
            port.out_node_index,
            out_node_index,
            Transition::LookAheadAssertion(LookAheadAssertionTransition::new(
                sub_line_index,
                negative,
            )),
        );

        Ok(Port::new(in_node_index, out_node_index))
    }

    fn emit_lookbehind_assertion(
        &mut self,
        current_expression: &Expression,
        previous_expression: &Expression,
        negative: bool,
    ) -> Result<Port, AnreError> {
        // * is_after(A, B), A.is_after(B), (?<=B)A
        // * is_not_after(A, B, A.is_not_after(B), (?<!B)A

        //       | lookbehind
        //       v trans /-----------\        out
        // ==o==--------==o in  out o==-----==o==
        //  in           \-----------/  jump

        // 1. save the current line index
        // 2. create new line
        let saved_line_index = self.current_line_index;
        let sub_line_index = self.route.new_line();

        // 3. switch to the new line
        self.current_line_index = sub_line_index;

        let match_length_in_char = {
            // calculate the total length (in char) of patterns
            let enum_length = get_match_length(previous_expression);
            let length = if let MatchLength::Fixed(val) = enum_length {
                val
            } else {
                return Err(AnreError::SyntaxIncorrect("Look behind assertion (is_after, is_not_after) requires a fixed length pattern.".to_owned()));
            };

            let sub_port = self.emit_expression(previous_expression)?;
            let sub_line = self.get_current_line_ref_mut();

            // save the sub-program ports
            sub_line.start_node_index = sub_port.in_node_index;
            sub_line.end_node_index = sub_port.out_node_index;
            sub_line.fixed_start_position = true;

            length
        };

        // restore to the previous line
        self.current_line_index = saved_line_index;

        let port = self.emit_expression(current_expression)?;
        let line = self.get_current_line_ref_mut();
        let in_node_index = line.new_node();
        let out_node_index = line.new_node();

        line.append_transition(
            in_node_index,
            port.in_node_index,
            Transition::LookBehindAssertion(LookBehindAssertionTransition::new(
                sub_line_index,
                negative,
                match_length_in_char,
            )),
        );

        line.append_transition(
            port.out_node_index,
            out_node_index,
            Transition::Jump(JumpTransition),
        );

        Ok(Port::new(in_node_index, out_node_index))
    }
}

struct Port {
    in_node_index: usize,
    out_node_index: usize,
}

impl Port {
    fn new(in_node_index: usize, out_node_index: usize) -> Self {
        Port {
            in_node_index,
            out_node_index,
        }
    }
}

fn append_preset_charset_positive_only(
    name: &PresetCharSetName,
    items: &mut Vec<CharSetItem>,
) -> Result<(), AnreError> {
    match name {
        PresetCharSetName::CharWord => {
            add_preset_word(items);
        }
        PresetCharSetName::CharSpace => {
            add_preset_space(items);
        }
        PresetCharSetName::CharDigit => {
            add_preset_digit(items);
        }
        _ => {
            return Err(AnreError::SyntaxIncorrect(format!(
                "Can not append negative preset charset \"{}\" into charset.",
                name
            )));
        }
    }

    Ok(())
}

fn append_charset(charset: &CharSet, items: &mut Vec<CharSetItem>) -> Result<(), AnreError> {
    for element in &charset.elements {
        match element {
            CharSetElement::Char(c) => add_char(items, *c),
            CharSetElement::CharRange(CharRange {
                start,
                end_included,
            }) => add_range(items, *start, *end_included),
            CharSetElement::PresetCharSet(name) => {
                append_preset_charset_positive_only(name, items)?;
            }
            CharSetElement::CharSet(custom_charset) => {
                assert!(!custom_charset.negative);
                append_charset(custom_charset, items)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_str_eq;

    use crate::{
        route::{Route, MAIN_LINE_INDEX},
        AnreError,
    };

    use super::{compile_from_anre, compile_from_regex};

    fn generate_routes(anre: &str, regex: &str) -> [Route; 2] {
        [
            compile_from_anre(anre).unwrap(),
            compile_from_regex(regex).unwrap(),
        ]
    }

    #[test]
    fn test_compile_char() {
        // single char
        for route in generate_routes(r#"'a'"#, r#"a"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"
            );
        }

        // sequence chars
        {
            let route = compile_from_anre(r#"'a', 'b', 'c'"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 4, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 7, Capture end {0}
> 6
  -> 0, Capture start {0}
< 7
# {0}"
            );
        }

        // char group
        // note: the group of anre is different from traditional regex, it is
        // only a sequence pattern.
        {
            let route = compile_from_anre(r#"'a',('b','c'), 'd'"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 4, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 6, Jump
- 6
  -> 7, Char 'd'
- 7
  -> 9, Capture end {0}
> 8
  -> 0, Capture start {0}
< 9
# {0}"
            );
        }

        // nested groups
        {
            let route = compile_from_anre(r#"'a',('b', ('c', 'd'), 'e'), 'f'"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 4, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 6, Jump
- 6
  -> 7, Char 'd'
- 7
  -> 8, Jump
- 8
  -> 9, Char 'e'
- 9
  -> 10, Jump
- 10
  -> 11, Char 'f'
- 11
  -> 13, Capture end {0}
> 12
  -> 0, Capture start {0}
< 13
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_logic_or() {
        // two operands
        for route in generate_routes(r#"'a' || 'b'"#, r#"a|b"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 5, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 5, Jump
- 4
  -> 0, Jump
  -> 2, Jump
- 5
  -> 7, Capture end {0}
> 6
  -> 4, Capture start {0}
< 7
# {0}"
            );
        }

        // three operands
        // operator associativity
        // the current interpreter is right-associative, so:
        // "'a' || 'b' || 'c'" => "'a' || ('b' || 'c')"
        for route in generate_routes(r#"'a' || 'b' || 'c'"#, r#"a|b|c"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 9, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 7, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 7, Jump
- 6
  -> 2, Jump
  -> 4, Jump
- 7
  -> 9, Jump
- 8
  -> 0, Jump
  -> 6, Jump
- 9
  -> 11, Capture end {0}
> 10
  -> 8, Capture start {0}
< 11
# {0}"
            );
        }

        // use "group" to change associativity
        for route in generate_routes(r#"('a' || 'b') || 'c'"#, r#"(?:a|b)|c"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 5, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 5, Jump
- 4
  -> 0, Jump
  -> 2, Jump
- 5
  -> 9, Jump
- 6
  -> 7, Char 'c'
- 7
  -> 9, Jump
- 8
  -> 4, Jump
  -> 6, Jump
- 9
  -> 11, Capture end {0}
> 10
  -> 8, Capture start {0}
< 11
# {0}"
            );
        }

        // operator precedence
        // "||" is higher than ","
        // "'a', 'b' || 'c', 'd'" => "'a', ('b' || 'c'), 'd'"
        for route in generate_routes(r#"'a', 'b' || 'c', 'd'"#, r#"a(?:b|c)d"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 6, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 7, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 7, Jump
- 6
  -> 2, Jump
  -> 4, Jump
- 7
  -> 8, Jump
- 8
  -> 9, Char 'd'
- 9
  -> 11, Capture end {0}
> 10
  -> 0, Capture start {0}
< 11
# {0}"
            );
        }

        // use "group" to change precedence
        {
            let route = compile_from_anre(r#"('a', 'b') || 'c'"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 7, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 7, Jump
- 6
  -> 0, Jump
  -> 4, Jump
- 7
  -> 9, Capture end {0}
> 8
  -> 6, Capture start {0}
< 9
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_special_char() {
        for route in generate_routes(r#"'a', char_any"#, r#"a."#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Any char
- 3
  -> 5, Capture end {0}
> 4
  -> 0, Capture start {0}
< 5
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_preset_charset() {
        // positive preset charset
        for route in generate_routes(r#"'a', char_word, char_space, char_digit"#, r#"a\w\s\d"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Charset ['A'..'Z', 'a'..'z', '0'..'9', '_']
- 3
  -> 4, Jump
- 4
  -> 5, Charset [' ', '\t', '\r', '\n']
- 5
  -> 6, Jump
- 6
  -> 7, Charset ['0'..'9']
- 7
  -> 9, Capture end {0}
> 8
  -> 0, Capture start {0}
< 9
# {0}"#
            );
        }

        // negative preset charset
        for route in generate_routes(
            r#"'a', char_not_word, char_not_space, char_not_digit"#,
            r#"a\W\S\D"#,
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Charset !['A'..'Z', 'a'..'z', '0'..'9', '_']
- 3
  -> 4, Jump
- 4
  -> 5, Charset ![' ', '\t', '\r', '\n']
- 5
  -> 6, Jump
- 6
  -> 7, Charset !['0'..'9']
- 7
  -> 9, Capture end {0}
> 8
  -> 0, Capture start {0}
< 9
# {0}"#
            );
        }
    }

    #[test]
    fn test_compile_charset() {
        // build with char and range
        for route in generate_routes(r#"['a', '0'..'7']"#, r#"[a0-7]"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Charset ['a', '0'..'7']
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"
            );
        }

        // negative charset
        for route in generate_routes(r#"!['a','0'..'7']"#, r#"[^a0-7]"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Charset !['a', '0'..'7']
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"
            );
        }

        // build with preset charset
        for route in generate_routes(r#"[char_word, char_space]"#, r#"[\w\s]"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Charset ['A'..'Z', 'a'..'z', '0'..'9', '_', ' ', '\t', '\r', '\n']
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"#
            );
        }

        // nested charset
        {
            let route = compile_from_anre(r#"['a', ['x'..'z']]"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Charset ['a', 'x'..'z']
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"
            );
        }

        // deep nested charset
        {
            let route =
                compile_from_anre(r#"[['+', '-'], ['0'..'9', ['a'..'f', char_space]]]"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Charset ['+', '-', '0'..'9', 'a'..'f', ' ', '\t', '\r', '\n']
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"#
            );
        }

        // build with marco
        {
            let route = compile_from_anre(
                r#"
define(prefix, ['+', '-'])
define(letter, ['a'..'f', char_space])
[prefix, ['0'..'9', letter]]"#,
            )
            .unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Charset ['+', '-', '0'..'9', 'a'..'f', ' ', '\t', '\r', '\n']
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"#
            );
        }

        // err: negative preset charset in custom charset
        {
            assert!(matches!(
                compile_from_anre(r#"[char_not_word]"#),
                Err(AnreError::SyntaxIncorrect(_))
            ));
        }

        // err: negative custom charset in custom charset
        // "Unexpected char set element."
        {
            assert!(matches!(
                compile_from_anre(r#"['+', !['a'..'f']]"#),
                Err(AnreError::MessageWithLocation(_, _))
            ));
        }
    }

    #[test]
    fn test_compile_assertion() {
        for route in generate_routes(r#"start, is_bound, 'a'"#, r#"^\ba"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Anchor assertion \"start\"
- 1
  -> 2, Jump
- 2
  -> 3, Boundary assertion \"is_bound\"
- 3
  -> 4, Jump
- 4
  -> 5, Char 'a'
- 5
  -> 7, Capture end {0}
> 6
  -> 0, Capture start {0}
< 7
# {0}"
            );

            // check the 'fixed_start_position' property
            assert!(route.lines[MAIN_LINE_INDEX].fixed_start_position);
        }

        for route in generate_routes(r#"is_not_bound, 'a', end"#, r#"\Ba$"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Boundary assertion \"is_not_bound\"
- 1
  -> 2, Jump
- 2
  -> 3, Char 'a'
- 3
  -> 4, Jump
- 4
  -> 5, Anchor assertion \"end\"
- 5
  -> 7, Capture end {0}
> 6
  -> 0, Capture start {0}
< 7
# {0}"
            );

            // check the 'fixed_start_position' property
            assert!(!route.lines[MAIN_LINE_INDEX].fixed_start_position);
        }

        // err: assert "start" can only be present at the beginning of expression
        {
            assert!(matches!(
                compile_from_anre(r#"'a', start, 'b'"#),
                Err(AnreError::SyntaxIncorrect(_))
            ));
        }

        // err: assert "end" can only be present at the end of expression
        {
            assert!(matches!(
                compile_from_anre(r#"'a', end, 'b'"#),
                Err(AnreError::SyntaxIncorrect(_))
            ));
        }
    }

    #[test]
    fn test_compile_capture_group_by_name() {
        // function call, and rear function call
        for route in generate_routes(r#"name('a', foo), 'b'.name(bar)"#, r#"(?<foo>a)(?<bar>b)"#) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Capture end {1}
- 2
  -> 0, Capture start {1}
- 3
  -> 6, Jump
- 4
  -> 5, Char 'b'
- 5
  -> 7, Capture end {2}
- 6
  -> 4, Capture start {2}
- 7
  -> 9, Capture end {0}
> 8
  -> 2, Capture start {0}
< 9
# {0}
# {1}, foo
# {2}, bar"
            );
        }

        // complex expressions as function call args
        for route in generate_routes(
            r#"name(('a', char_digit), foo), ('x' || 'y').name(bar)"#,
            r#"(?<foo>a\d)(?<bar>(?:x|y))"#,
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Charset ['0'..'9']
- 3
  -> 5, Capture end {1}
- 4
  -> 0, Capture start {1}
- 5
  -> 12, Jump
- 6
  -> 7, Char 'x'
- 7
  -> 11, Jump
- 8
  -> 9, Char 'y'
- 9
  -> 11, Jump
- 10
  -> 6, Jump
  -> 8, Jump
- 11
  -> 13, Capture end {2}
- 12
  -> 10, Capture start {2}
- 13
  -> 15, Capture end {0}
> 14
  -> 4, Capture start {0}
< 15
# {0}
# {1}, foo
# {2}, bar"
            );
        }

        // nested function call
        {
            let route = compile_from_anre(r#"name(name('a', foo), bar)"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Capture end {2}
- 2
  -> 0, Capture start {2}
- 3
  -> 5, Capture end {1}
- 4
  -> 2, Capture start {1}
- 5
  -> 7, Capture end {0}
> 6
  -> 4, Capture start {0}
< 7
# {0}
# {1}, bar
# {2}, foo"
            );
        }

        // chaining function call
        {
            let route = compile_from_anre(r#"'a'.name(foo).name(bar)"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Capture end {2}
- 2
  -> 0, Capture start {2}
- 3
  -> 5, Capture end {1}
- 4
  -> 2, Capture start {1}
- 5
  -> 7, Capture end {0}
> 6
  -> 4, Capture start {0}
< 7
# {0}
# {1}, bar
# {2}, foo"
            );
        }
    }

    #[test]
    fn test_compile_capture_group_by_index() {
        // function call, and rear function call
        for route in generate_routes(
            r#"index('a'), 'b'.index()"#, // anre
            r#"(a)(b)"#,                  // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Capture end {1}
- 2
  -> 0, Capture start {1}
- 3
  -> 6, Jump
- 4
  -> 5, Char 'b'
- 5
  -> 7, Capture end {2}
- 6
  -> 4, Capture start {2}
- 7
  -> 9, Capture end {0}
> 8
  -> 2, Capture start {0}
< 9
# {0}
# {1}
# {2}"
            );
        }
    }

    // 'backreference' requires the 'name' and 'index' functions
    // to be completed first
    #[test]
    fn test_compile_backreference() {
        for route in generate_routes(
            r#"'a'.name(foo), 'b', foo"#, // anre
            r#"(?<foo>a)b\k<foo>"#,       // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Capture end {1}
- 2
  -> 0, Capture start {1}
- 3
  -> 4, Jump
- 4
  -> 5, Char 'b'
- 5
  -> 6, Jump
- 6
  -> 7, Back reference {1}
- 7
  -> 9, Capture end {0}
> 8
  -> 2, Capture start {0}
< 9
# {0}
# {1}, foo"
            );
        }
    }

    #[test]
    fn test_compile_optional() {
        // greedy
        for route in generate_routes(
            r#"'a'?"#, // anre
            r#"a?"#,   // regex
        ) {
            let s = route.get_debug_text();
            // println!("{}", s);

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 0, Jump
  -> 3, Jump
- 3
  -> 5, Capture end {0}
> 4
  -> 2, Capture start {0}
< 5
# {0}"
            );
        }

        // lazy
        for route in generate_routes(
            r#"'a'??"#, // anre
            r#"a??"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 3, Jump
  -> 0, Jump
- 3
  -> 5, Capture end {0}
> 4
  -> 2, Capture start {0}
< 5
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_repatition_specified() {
        // repeat >1
        for route in generate_routes(
            r#"'a'{2}"#, // anre
            r#"a{2}"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 5, Counter check times 2
  -> 3, Repetition times 2
- 5
  -> 7, Capture end {0}
> 6
  -> 2, Capture start {0}
< 7
# {0}"
            );
        }

        // repeat 1
        for route in generate_routes(
            r#"'a'{1}"#, // anre
            r#"a{1}"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"
            );
        }

        // repeat 0
        for route in generate_routes(
            r#"'a'{0}"#, // anre
            r#"a{0}"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Jump
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_repatition_range() {
        // greedy
        for route in generate_routes(
            r#"'a'{3,5}"#, // anre
            r#"a{3,5}"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 3, Repetition from 3 to 5
  -> 5, Counter check from 3 to 5
- 5
  -> 7, Capture end {0}
> 6
  -> 2, Capture start {0}
< 7
# {0}"
            );
        }

        // lazy
        for route in generate_routes(
            r#"'a'{3,5}?"#, // anre
            r#"a{3,5}?"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 5, Counter check from 3 to 5
  -> 3, Repetition from 3 to 5
- 5
  -> 7, Capture end {0}
> 6
  -> 2, Capture start {0}
< 7
# {0}"
            );
        }

        // {m, m}
        {
            assert_str_eq!(
                compile_from_anre(r#"'a'{3,3}"#).unwrap().get_debug_text(),
                compile_from_anre(r#"'a'{3}"#).unwrap().get_debug_text()
            )
        }

        // {1, 1}
        {
            assert_str_eq!(
                compile_from_anre(r#"'a'{1,1}"#).unwrap().get_debug_text(),
                compile_from_anre(r#"'a'"#).unwrap().get_debug_text()
            )
        }

        // {0, m}
        for route in generate_routes(
            r#"'a'{0,5}"#, // anre
            r#"a{0,5}"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 3, Repetition from 1 to 5
  -> 5, Counter check from 1 to 5
- 5
  -> 7, Jump
- 6
  -> 2, Jump
  -> 7, Jump
- 7
  -> 9, Capture end {0}
> 8
  -> 6, Capture start {0}
< 9
# {0}"
            );
        }

        // {0, m} lazy
        for route in generate_routes(
            r#"'a'{0,5}?"#, // anre
            r#"a{0,5}?"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 5, Counter check from 1 to 5
  -> 3, Repetition from 1 to 5
- 5
  -> 7, Jump
- 6
  -> 7, Jump
  -> 2, Jump
- 7
  -> 9, Capture end {0}
> 8
  -> 6, Capture start {0}
< 9
# {0}"
            );
        }

        // {0, 1}
        {
            assert_str_eq!(
                compile_from_anre(r#"'a'{0,1}"#).unwrap().get_debug_text(),
                compile_from_anre(r#"'a'?"#).unwrap().get_debug_text()
            )
        }

        // {0, 1} lazy
        {
            assert_str_eq!(
                compile_from_anre(r#"'a'{0,1}?"#).unwrap().get_debug_text(),
                compile_from_anre(r#"'a'??"#).unwrap().get_debug_text()
            )
        }

        // {0, 0}
        {
            let route = compile_from_anre(r#"'a'{0,0}"#).unwrap();
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Jump
- 1
  -> 3, Capture end {0}
> 2
  -> 0, Capture start {0}
< 3
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_repatition_at_least() {
        // {m,}
        for route in generate_routes(
            r#"'a'{3,}"#, // anre
            r#"a{3,}"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 3, Repetition from 3 to MAX
  -> 5, Counter check from 3 to MAX
- 5
  -> 7, Capture end {0}
> 6
  -> 2, Capture start {0}
< 7
# {0}"
            );
        }

        // lazy
        for route in generate_routes(
            r#"'a'{3,}?"#, // anre
            r#"a{3,}?"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 5, Counter check from 3 to MAX
  -> 3, Repetition from 3 to MAX
- 5
  -> 7, Capture end {0}
> 6
  -> 2, Capture start {0}
< 7
# {0}"
            );
        }

        // {1,} == one_or_more
        {
            assert_str_eq!(
                compile_from_anre(r#"'a'{1,}"#).unwrap().get_debug_text(),
                compile_from_anre(r#"'a'+"#).unwrap().get_debug_text()
            );
        }

        // {1,}? == lazy one_or_more
        {
            assert_str_eq!(
                compile_from_anre(r#"'a'{1,}?"#).unwrap().get_debug_text(),
                compile_from_anre(r#"'a'+?"#).unwrap().get_debug_text()
            );
        }

        // {0,} == zero_or_more
        {
            assert_str_eq!(
                compile_from_anre(r#"'a'{0,}"#).unwrap().get_debug_text(),
                compile_from_anre(r#"'a'*"#).unwrap().get_debug_text()
            );
        }

        // {0,}? == lazy zero_or_more
        {
            assert_str_eq!(
                compile_from_anre(r#"'a'{0,}?"#).unwrap().get_debug_text(),
                compile_from_anre(r#"'a'*?"#).unwrap().get_debug_text()
            );
        }
    }

    #[test]
    fn test_compile_notation_optional() {
        // optional
        for route in generate_routes(
            r#"'a'?"#, // anre
            r#"a?"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 0, Jump
  -> 3, Jump
- 3
  -> 5, Capture end {0}
> 4
  -> 2, Capture start {0}
< 5
# {0}"
            );
        }

        // lazy optional
        for route in generate_routes(
            r#"'a'??"#, // anre
            r#"a??"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 3, Jump
  -> 0, Jump
- 3
  -> 5, Capture end {0}
> 4
  -> 2, Capture start {0}
< 5
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_natation_repetition() {
        // one or more
        for route in generate_routes(
            r#"'a'+"#, // anre
            r#"a+"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 3, Repetition from 1 to MAX
  -> 5, Counter check from 1 to MAX
- 5
  -> 7, Capture end {0}
> 6
  -> 2, Capture start {0}
< 7
# {0}"
            );
        }

        // lazy one or more
        for route in generate_routes(
            r#"'a'+?"#, // anre
            r#"a+?"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 5, Counter check from 1 to MAX
  -> 3, Repetition from 1 to MAX
- 5
  -> 7, Capture end {0}
> 6
  -> 2, Capture start {0}
< 7
# {0}"
            );
        }

        // zero or more
        for route in generate_routes(
            r#"'a'*"#, // anre
            r#"a*"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 3, Repetition from 1 to MAX
  -> 5, Counter check from 1 to MAX
- 5
  -> 7, Jump
- 6
  -> 2, Jump
  -> 7, Jump
- 7
  -> 9, Capture end {0}
> 8
  -> 6, Capture start {0}
< 9
# {0}"
            );
        }

        // lazy zero or more
        for route in generate_routes(
            r#"'a'*?"#, // anre
            r#"a*?"#,   // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc
- 2
  -> 3, Counter reset
- 3
  -> 0, Counter save
- 4
  -> 5, Counter check from 1 to MAX
  -> 3, Repetition from 1 to MAX
- 5
  -> 7, Jump
- 6
  -> 7, Jump
  -> 2, Jump
- 7
  -> 9, Capture end {0}
> 8
  -> 6, Capture start {0}
< 9
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_is_before() {
        // positive
        for route in generate_routes(
            r#"'a'.is_before("xyz")"#, // anre
            r#"a(?=xyz)"#,             // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
= $0
- 0
  -> 1, Char 'a'
- 1
  -> 3, Look ahead $1
- 2
  -> 0, Jump
- 3
  -> 5, Capture end {0}
> 4
  -> 2, Capture start {0}
< 5
= $1
> 0
  -> 1, String \"xyz\"
< 1
# {0}"
            );
        }

        // negative
        for route in generate_routes(
            r#"'a'.is_not_before("xyz")"#, // anre
            r#"a(?!xyz)"#,                 // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
= $0
- 0
  -> 1, Char 'a'
- 1
  -> 3, Look ahead negative $1
- 2
  -> 0, Jump
- 3
  -> 5, Capture end {0}
> 4
  -> 2, Capture start {0}
< 5
= $1
> 0
  -> 1, String \"xyz\"
< 1
# {0}"
            );
        }

        // syntax error
        {
            assert!(matches!(
                compile_from_anre(r#"'a'.is_before()"#),
                Err(AnreError::SyntaxIncorrect(_))
            ));
        }
    }

    #[test]
    fn test_compile_is_after() {
        // positive
        for route in generate_routes(
            r#"'a'.is_after("xyz")"#, // anre
            r#"(?<=xyz)a"#,           // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
= $0
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 0, Look behind $1, match length 3
- 3
  -> 5, Capture end {0}
> 4
  -> 2, Capture start {0}
< 5
= $1
> 0
  -> 1, String \"xyz\"
< 1
# {0}"
            );
        }

        // negative
        for route in generate_routes(
            r#"'a'.is_not_after("xyz")"#, // anre
            r#"(?<!xyz)a"#,               // regex
        ) {
            let s = route.get_debug_text();

            assert_str_eq!(
                s,
                "\
= $0
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 0, Look behind negative $1, match length 3
- 3
  -> 5, Capture end {0}
> 4
  -> 2, Capture start {0}
< 5
= $1
> 0
  -> 1, String \"xyz\"
< 1
# {0}"
            );
        }

        // syntax error
        {
            assert!(matches!(
                compile_from_anre(r#"'a'.is_after()"#),
                Err(AnreError::SyntaxIncorrect(_))
            ));
        }

        // variable length
        {
            assert!(matches!(
                compile_from_anre(r#"'a'.is_after("x" || "yz")"#),
                Err(AnreError::SyntaxIncorrect(_))
            ));

            assert!(matches!(
                compile_from_anre(r#"'a'.is_after("x"+)"#),
                Err(AnreError::SyntaxIncorrect(_))
            ));
        }
    }
}
