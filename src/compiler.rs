// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use crate::{
    ast::{
        AnchorAssertionName, BackReference, BoundaryAssertionName, CharRange, CharSet,
        CharSetElement, Expression, FunctionCall, FunctionName, Literal, PresetCharSetName,
        Program,
    },
    object_file::{ObjectFile, Route},
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

/// Compile from traditional regular expression.
pub fn compile_from_regex(s: &str) -> Result<ObjectFile, AnreError> {
    let program = crate::traditional::parse_from_str(s)?;
    compile(&program)
}

/// Compile from ANRE regular expression.
pub fn compile_from_anre(s: &str) -> Result<ObjectFile, AnreError> {
    let program = crate::anre::parse_from_str(s)?;
    compile(&program)
}

/// Compile from AST `Program`.
pub fn compile(program: &Program) -> Result<ObjectFile, AnreError> {
    let mut route = ObjectFile::new();
    let mut compiler = Compiler::new(program, &mut route);
    compiler.compile()?;

    Ok(route)
}

pub struct Compiler<'a> {
    // The AST
    program: &'a Program,

    // The compilation target
    object_file: &'a mut ObjectFile,

    // Index of the current route
    current_route_index: usize,
}

impl<'a> Compiler<'a> {
    fn new(program: &'a Program, object_file: &'a mut ObjectFile) -> Self {
        let current_route_index = object_file.create_route();
        Compiler {
            program,
            object_file,
            current_route_index,
        }
    }

    // Get a mutable reference to the current route in the object file.
    fn get_current_route_ref_mut(&mut self) -> &mut Route {
        &mut self.object_file.routes[self.current_route_index]
    }

    // Start the compilation process by emitting the main program.
    fn compile(&mut self) -> Result<(), AnreError> {
        self.emit_program(self.program)
    }

    // Compile the main route of the program.
    fn emit_program(&mut self, program: &Program) -> Result<(), AnreError> {
        // Compile each sub-expression of the main route.
        //
        // The `Program` node is essentially a `Group` without explicit parentheses.
        // For example, "'a', 'b'+, 'c'" is equivalent to "('a', 'b'+, 'c')".
        //
        // Additionally, the program may include "start" and "end" assertions.
        //
        //                   Main Route
        //                   ----------
        //      Component     Component     Component
        // in   /-----\  jump /------\ jump  /-----\    out
        // o----| 'a' |-------| 'b'+ |-------| 'c' |----o
        // |    \-----/       \------/       \-----/    |
        // |                                            |
        // \----------------Program Component-----------/

        // Create the first (index 0) capture group to represent the program itself.
        let capture_group_index = self.object_file.create_capture_group(None);

        let expressions = &program.expressions;

        let mut is_fixed_start_position = false;
        let mut components = vec![];

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

                is_fixed_start_position = true;
                components.push(self.emit_anchor_assertion(&AnchorAssertionName::Start)?);
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

                components.push(self.emit_anchor_assertion(&AnchorAssertionName::End)?);
            } else {
                components.push(self.emit_expression(expression)?);
            }
        }

        let program_component = if components.is_empty() {
            // empty expression
            self.emit_empty()?
        } else if components.len() == 1 {
            // single expression
            components.pop().unwrap()
        } else {
            // multiple expression
            let route = self.get_current_route_ref_mut();
            for idx in 0..(components.len() - 1) {
                let previous_out_node_index = components[idx].out_node_index;
                let next_in_node_index = components[idx + 1].in_node_index;
                let transition = Transition::Jump(JumpTransition);
                route.create_transition_item(
                    previous_out_node_index,
                    next_in_node_index,
                    transition,
                );
            }

            Component::new(
                components.first().unwrap().in_node_index,
                components.last().unwrap().out_node_index,
            )
        };

        // Add transitions for capturing the start and end of the program component.
        // This capture group is the default group with index 0.
        //
        // The structure of the program component with capture transitions:
        //
        //
        //                    program
        //   capture start   component      capture end
        //        trans    /-----------\    trans
        //  ==o==---------==o in  out o==--------==o==
        // in |            \-----------/           | out
        //    |                                    |
        //    \-------------- route ---------------/
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();

        let capture_start_transition = CaptureStartTransition::new(capture_group_index);
        let capture_end_transition = CaptureEndTransition::new(capture_group_index);

        route.create_transition_item(
            in_node_index,
            program_component.in_node_index,
            Transition::CaptureStart(capture_start_transition),
        );

        route.create_transition_item(
            program_component.out_node_index,
            out_node_index,
            Transition::CaptureEnd(capture_end_transition),
        );

        // update the program ports and properties
        route.start_node_index = in_node_index;
        route.end_node_index = out_node_index;
        route.is_fixed_start_position = is_fixed_start_position;

        Ok(())
    }

    /// Compile an expression to a component
    fn emit_expression(&mut self, expression: &Expression) -> Result<Component, AnreError> {
        let result = match expression {
            // Expression::Identifier(id) => {
            //     return Err(AnreError::SyntaxIncorrect(format!(
            //         "Identifier is only allowed in backreference, identifier: {}.",
            //         id
            //     )));
            // }
            Expression::Literal(literal) => self.emit_literal(literal)?,
            Expression::BackReference(back_reference) => self.emit_backreference(back_reference)?,
            Expression::AnchorAssertion(name) => {
                // syntax error
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

    fn emit_group(&mut self, expressions: &[Expression]) -> Result<Component, AnreError> {
        // Compile each expression into a component and connect adjacent components
        // using "jump transitions".
        //
        // Diagram illustrating the connection between components:
        //
        // ```diagram
        //     prev component  jump      next component
        //     /-----------\   trans    /-----------\
        // ====o in  out o==----------==o in  out o======
        //  |  \-----------/            \-----------/  |
        //  |                                          |
        //  \--------------- component ----------------/
        // ```
        //
        // The "group" in ANRE differs from the "group" in traditional regular expressions.
        // In ANRE, a "group" is a series of parenthesized patterns that are not captured
        // unless explicitly referenced by the 'name' or 'index' function.
        //
        // In terms of functionality, the "group" in ANRE is equivalent to the "non-capturing group"
        // in traditional regular expressions.
        //
        // Example:
        //
        // ANRE: `('a', 'b', char_word+)`
        // Equivalent Regex: `ab\w+`
        //
        // The "group" in ANRE is used to group patterns and modify operator precedence and associativity.
        //
        // Note: Do NOT add "capture transitions" around the group in ANRE.

        let mut components = vec![];
        for expression in expressions {
            components.push(self.emit_expression(expression)?);
        }

        let compontent = if components.is_empty() {
            // empty expression
            self.emit_empty()?
        } else if components.len() == 1 {
            // single expression.
            // maybe a group also, so return the underlay port directly
            // to eliminates the nested group, e.g. '(((...)))'.
            components.pop().unwrap()
        } else {
            // multiple expressions
            let route = self.get_current_route_ref_mut();
            for idx in 0..(components.len() - 1) {
                let current_out_state_index = components[idx].out_node_index;
                let next_in_state_index = components[idx + 1].in_node_index;
                let transition = Transition::Jump(JumpTransition);
                route.create_transition_item(
                    current_out_state_index,
                    next_in_state_index,
                    transition,
                );
            }

            Component::new(
                components.first().unwrap().in_node_index,
                components.last().unwrap().out_node_index,
            )
        };

        Ok(compontent)
    }

    fn emit_logic_or(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Component, AnreError> {
        // ```diagram
        //                    left
        //         jump   /-----------\   jump
        //      /--------==o in  out o==--------\
        //  in  |         \-----------/         |  out
        // ==o--|                               |--o==
        //   |  |             right             |  |
        //   |  |         /-----------\         |  |
        //   |  \--------==o in  out o==--------/  |
        //   |      jump   \-----------/   jump    |
        //   |                                     |
        //   \-------------- component ------------/
        // ```

        let left_port = self.emit_expression(left)?;
        let right_port = self.emit_expression(right)?;

        let route = self.get_current_route_ref_mut();

        let in_state_index = route.create_node();
        let out_state_index = route.create_node();

        route.create_transition_item(
            in_state_index,
            left_port.in_node_index,
            Transition::Jump(JumpTransition),
        );

        route.create_transition_item(
            in_state_index,
            right_port.in_node_index,
            Transition::Jump(JumpTransition),
        );

        route.create_transition_item(
            left_port.out_node_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        route.create_transition_item(
            right_port.out_node_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        Ok(Component::new(in_state_index, out_state_index))
    }

    fn emit_function_call(&mut self, function_call: &FunctionCall) -> Result<Component, AnreError> {
        let expression = &function_call.args[0];
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
                let component = self.emit_repeat_range(expression, 1, usize::MAX, is_lazy)?;
                self.continue_emit_optional(component, is_lazy)
            }
            FunctionName::Repeat => {
                let times = if let Expression::Literal(Literal::Number(n)) = &args[0] {
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
                let from = if let Expression::Literal(Literal::Number(n)) = &args[0] {
                    *n
                } else {
                    unreachable!()
                };

                let to = if let Expression::Literal(Literal::Number(n)) = &args[1] {
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
                        let component = self.emit_repeat_range(expression, 1, to, is_lazy)?;
                        self.continue_emit_optional(component, is_lazy)
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
                let from = if let Expression::Literal(Literal::Number(n)) = &args[0] {
                    *n
                } else {
                    unreachable!()
                };

                if from == 0 {
                    // {0,MAX} == optional + one_or_more
                    let component = self.emit_repeat_range(expression, 1, usize::MAX, is_lazy)?;
                    self.continue_emit_optional(component, is_lazy)
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

                let next_expression = &args[0];

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

                let previous_expression = &args[0];

                let negative = function_call.name == FunctionName::IsNotAfter;
                self.emit_lookbehind_assertion(expression, previous_expression, negative)
            }

            // Capture
            FunctionName::Name => self.emit_capture_group_by_name(expression, args),
            FunctionName::Index => self.emit_capture_group_by_index(expression),
        }
    }

    /// Short-cut component.
    fn emit_empty(&mut self) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();

        route.create_transition_item(
            in_node_index,
            out_node_index,
            Transition::Jump(JumpTransition),
        );
        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_literal(&mut self, literal: &Literal) -> Result<Component, AnreError> {
        let component = match literal {
            Literal::Number(n) => {
                return Err(AnreError::SyntaxIncorrect(format!(
                    "Number literal is only allowed in repetition, number: {}.",
                    n
                )));
            }
            Literal::Char(character) => self.emit_literal_char(*character)?,
            Literal::String(s) => self.emit_literal_string(s)?,
            Literal::CharSet(charset) => self.emit_literal_charset(charset)?,
            Literal::PresetCharSet(name) => self.emit_literal_preset_charset(name)?,
            Literal::Special(_) => self.emit_literal_special_char()?,
        };

        Ok(component)
    }

    fn emit_literal_char(&mut self, character: char) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();
        let transition = Transition::Char(CharTransition::new(character));

        route.create_transition_item(in_node_index, out_node_index, transition);
        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_literal_special_char(&mut self) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_out_index = route.create_node();
        let out_out_index = route.create_node();
        let transition = Transition::SpecialChar(SpecialCharTransition);

        route.create_transition_item(in_out_index, out_out_index, transition);
        Ok(Component::new(in_out_index, out_out_index))
    }

    fn emit_literal_string(&mut self, s: &str) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();
        let transition = Transition::String(StringTransition::new(s));

        route.create_transition_item(in_node_index, out_node_index, transition);
        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_literal_preset_charset(
        &mut self,
        name: &PresetCharSetName,
    ) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();

        let charset_transition = match name {
            PresetCharSetName::CharWord => CharSetTransition::new_preset_word(),
            PresetCharSetName::CharNotWord => CharSetTransition::new_preset_not_word(),
            PresetCharSetName::CharSpace => CharSetTransition::new_preset_space(),
            PresetCharSetName::CharNotSpace => CharSetTransition::new_preset_not_space(),
            PresetCharSetName::CharDigit => CharSetTransition::new_preset_digit(),
            PresetCharSetName::CharNotDigit => CharSetTransition::new_preset_not_digit(),
            PresetCharSetName::CharHex => CharSetTransition::new_preset_hex(),
        };

        let transition = Transition::CharSet(charset_transition);
        route.create_transition_item(in_node_index, out_node_index, transition);
        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_literal_charset(&mut self, charset: &CharSet) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();

        let mut items: Vec<CharSetItem> = vec![];
        append_charset(charset, &mut items)?;

        let transition = Transition::CharSet(CharSetTransition::new(items, charset.negative));
        route.create_transition_item(in_node_index, out_node_index, transition);
        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_anchor_assertion(
        &mut self,
        name: &AnchorAssertionName,
    ) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();
        let transition = Transition::AnchorAssertion(AnchorAssertionTransition::new(*name));

        route.create_transition_item(in_node_index, out_node_index, transition);
        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_boundary_assertion(
        &mut self,
        name: &BoundaryAssertionName,
    ) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();
        let transition = Transition::BoundaryAssertion(BoundaryAssertionTransition::new(*name));

        route.create_transition_item(in_node_index, out_node_index, transition);
        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_backreference(
        &mut self,
        back_reference: &BackReference,
    ) -> Result<Component, AnreError> {
        match back_reference {
            BackReference::Index(index) => self.emit_backreference_by_index(*index),
            BackReference::Name(name) => self.emit_backreference_by_name(name),
        }
    }

    fn emit_backreference_by_index(
        &mut self,
        capture_group_index: usize,
    ) -> Result<Component, AnreError> {
        if capture_group_index >= self.object_file.capture_group_names.len() {
            return Err(AnreError::SyntaxIncorrect(format!(
                "The group index ({}) of back-reference is out of range, the max index should be: {}.",
                capture_group_index, self.object_file.capture_group_names.len() - 1
            )));
        }

        self.continue_emit_backreference(capture_group_index)
    }

    fn emit_backreference_by_name(&mut self, name: &str) -> Result<Component, AnreError> {
        let capture_group_index_option = self.object_file.get_capture_group_index_by_name(name);
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
    ) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();
        let transition =
            Transition::BackReference(BackReferenceTransition::new(capture_group_index));

        route.create_transition_item(in_node_index, out_node_index, transition);
        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_capture_group_by_name(
        &mut self,
        expression: &Expression,
        args: &[Expression],
    ) -> Result<Component, AnreError> {
        let name = if let Expression::Literal(Literal::String(s)) = &args[0] {
            s.to_owned()
        } else {
            unreachable!();
        };

        self.continue_emit_capture_group(expression, Some(name))
    }

    fn emit_capture_group_by_index(
        &mut self,
        expression: &Expression,
    ) -> Result<Component, AnreError> {
        self.continue_emit_capture_group(expression, None)
    }

    fn continue_emit_capture_group(
        &mut self,
        expression: &Expression,
        name_option: Option<String>,
    ) -> Result<Component, AnreError> {
        let capture_group_index = self.object_file.create_capture_group(name_option);
        let component = self.emit_expression(expression)?;

        //   capture start   component    capture end
        //        trans    /-----------\    trans
        //  ==o==---------==o in  out o==--------==o==
        // in |            \-----------/           | out
        //    |                                    |
        //    \-------------- component -----------/

        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();
        let capture_start_transition = CaptureStartTransition::new(capture_group_index);
        let capture_end_transition = CaptureEndTransition::new(capture_group_index);

        route.create_transition_item(
            in_node_index,
            component.in_node_index,
            Transition::CaptureStart(capture_start_transition),
        );

        route.create_transition_item(
            component.out_node_index,
            out_node_index,
            Transition::CaptureEnd(capture_end_transition),
        );

        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_optional(
        &mut self,
        expression: &Expression,
        is_lazy: bool,
    ) -> Result<Component, AnreError> {
        // Append nodes and jump transitions around the component to
        // implement the "optional" function.
        //
        // for greedy optional:
        //
        //                 component
        //   in     jmp  /-----------\  jmp
        //  ==o|o==-----==o in  out o==---==o==
        //     |o==\     \-----------/      ^ out
        //         |                        |
        //         \------------------------/
        //                jump trans
        //
        // for lazy optional:
        //
        //                jump trans
        //         /------------------------\
        //         |                        |
        //     |o==/     /-----------\      v out
        //  ==o|o==-----==o in  out o==---==o==
        //   in     jmp  \-----------/  jmp
        //                 component

        let component = self.emit_expression(expression)?;
        self.continue_emit_optional(component, is_lazy)
    }

    fn continue_emit_optional(
        &mut self,
        port: Component,
        is_lazy: bool,
    ) -> Result<Component, AnreError> {
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();

        if is_lazy {
            route.create_transition_item(
                in_node_index,
                out_node_index,
                Transition::Jump(JumpTransition),
            );
        }

        route.create_transition_item(
            in_node_index,
            port.in_node_index,
            Transition::Jump(JumpTransition),
        );

        route.create_transition_item(
            port.out_node_index,
            out_node_index,
            Transition::Jump(JumpTransition),
        );

        if !is_lazy {
            route.create_transition_item(
                in_node_index,
                out_node_index,
                Transition::Jump(JumpTransition),
            );
        }

        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_repeat_specified(
        &mut self,
        expression: &Expression,
        times: usize,
    ) -> Result<Component, AnreError> {
        assert!(times > 1);
        self.continue_emit_repetition(expression, RepetitionType::Specified(times), true)
    }

    fn emit_repeat_range(
        &mut self,
        expression: &Expression,
        from: usize,
        to: usize,
        is_lazy: bool,
    ) -> Result<Component, AnreError> {
        assert!(from > 0 && to > 1 && to > from);
        self.continue_emit_repetition(expression, RepetitionType::Range(from, to), is_lazy)
    }

    fn continue_emit_repetition(
        &mut self,
        expression: &Expression,
        repetition_type: RepetitionType,
        is_lazy: bool,
    ) -> Result<Component, AnreError> {
        // Append nodes and transitions around the component to
        // implement the "repetition" function.
        //
        // for lazy repetition:
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
        // for greedy repetion:
        //
        //                             repetition trans
        //                   /---------------------------------------\
        //                   |                                       |
        //                   |   | counter              | counter    |
        //                   |   | save                 | restore &  |
        //                   |   | trans                | inc        |
        //   in              v   v       /-----------\  v trans      |
        //  ==o==-------=====o==--------==o in  out o==-------==o|o==/     out
        //        ^ counter  left        \-----------/     right |o==----==o==
        //        | reset                                             ^
        //        | trans                               counter check |
        //                                                      trans |

        let component = self.emit_expression(expression)?;

        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let left_node_index = route.create_node();
        let right_node_index = route.create_node();

        route.create_transition_item(
            in_node_index,
            left_node_index,
            Transition::CounterReset(CounterResetTransition),
        );

        route.create_transition_item(
            left_node_index,
            component.in_node_index,
            Transition::CounterSave(CounterSaveTransition),
        );

        route.create_transition_item(
            component.out_node_index,
            right_node_index,
            Transition::CounterInc(CounterIncTransition),
        );

        let out_node_index = route.create_node();

        if is_lazy {
            route.create_transition_item(
                right_node_index,
                out_node_index,
                Transition::CounterCheck(CounterCheckTransition::new(
                    // counter_index,
                    repetition_type.clone(),
                )),
            );

            route.create_transition_item(
                right_node_index,
                left_node_index,
                Transition::Repetition(RepetitionTransition::new(
                    // counter_index,
                    repetition_type,
                )),
            );
        } else {
            route.create_transition_item(
                right_node_index,
                left_node_index,
                Transition::Repetition(RepetitionTransition::new(
                    //counter_index,
                    repetition_type.clone(),
                )),
            );

            route.create_transition_item(
                right_node_index,
                out_node_index,
                Transition::CounterCheck(CounterCheckTransition::new(
                    // counter_index,
                    repetition_type,
                )),
            );
        }

        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_lookahead_assertion(
        &mut self,
        current_expression: &Expression,
        next_expression: &Expression,
        negative: bool,
    ) -> Result<Component, AnreError> {
        // Compile two kinds of look ahead assertions:
        //
        // - is_before(A, B), A.is_before(B), A(?=B)
        // - is_not_before(A, B), A.is_not_before(B), A(?!B)
        //
        //                              | lookahead
        //  in       /-----------\      v trans
        // ==o==----==o in  out o==----------==o==
        //      jump \-----------/            out

        let component = self.emit_expression(current_expression)?;

        // 1. save the current route index
        // 2. create new route
        let saved_route_index = self.current_route_index;
        let sub_route_index = self.object_file.create_route();

        // 3. switch to the new route
        self.current_route_index = sub_route_index;

        {
            let sub_component = self.emit_expression(next_expression)?;

            // update the sub-route
            let sub_route = self.get_current_route_ref_mut();
            sub_route.start_node_index = sub_component.in_node_index;
            sub_route.end_node_index = sub_component.out_node_index;
            sub_route.is_fixed_start_position = true;
        }

        // 4. restore to the previous route
        self.current_route_index = saved_route_index;

        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();

        // 5. join the sub_route to the current route by
        // appending jump transitions around the sub-component.
        route.create_transition_item(
            in_node_index,
            component.in_node_index,
            Transition::Jump(JumpTransition),
        );

        route.create_transition_item(
            component.out_node_index,
            out_node_index,
            Transition::LookAheadAssertion(LookAheadAssertionTransition::new(
                sub_route_index,
                negative,
            )),
        );

        Ok(Component::new(in_node_index, out_node_index))
    }

    fn emit_lookbehind_assertion(
        &mut self,
        current_expression: &Expression,
        previous_expression: &Expression,
        negative: bool,
    ) -> Result<Component, AnreError> {
        // Compile two kinds of look behind assertions:
        //
        // - is_after(A, B), A.is_after(B), (?<=B)A
        // - is_not_after(A, B, A.is_not_after(B), (?<!B)A
        //
        //       | lookbehind
        //       v trans     /-----------\        out
        // ==o==------------==o in  out o==-----==o==
        //  in               \-----------/  jump

        // 1. save the current route index
        // 2. create new route
        let saved_route_index = self.current_route_index;
        let sub_route_index = self.object_file.create_route();

        // 3. switch to the new route
        self.current_route_index = sub_route_index;

        let match_length_in_char = {
            // calculate the total length (in char) of patterns
            let enum_length = get_match_length(previous_expression);
            let length = if let MatchLength::Fixed(val) = enum_length {
                val
            } else {
                return Err(AnreError::SyntaxIncorrect("Look behind assertion (is_after, is_not_after) requires a fixed length pattern.".to_owned()));
            };

            let sub_component = self.emit_expression(previous_expression)?;
            let sub_route = self.get_current_route_ref_mut();

            // update the sub-route
            sub_route.start_node_index = sub_component.in_node_index;
            sub_route.end_node_index = sub_component.out_node_index;
            sub_route.is_fixed_start_position = true;

            length
        };

        // 4. restore to the previous route
        self.current_route_index = saved_route_index;

        let component = self.emit_expression(current_expression)?;
        let route = self.get_current_route_ref_mut();
        let in_node_index = route.create_node();
        let out_node_index = route.create_node();

        // 5. join the sub_route to the current route by
        // appending jump transitions around the sub-component.
        route.create_transition_item(
            in_node_index,
            component.in_node_index,
            Transition::LookBehindAssertion(LookBehindAssertionTransition::new(
                sub_route_index,
                negative,
                match_length_in_char,
            )),
        );

        route.create_transition_item(
            component.out_node_index,
            out_node_index,
            Transition::Jump(JumpTransition),
        );

        Ok(Component::new(in_node_index, out_node_index))
    }
}

// A component is a pair of input node and output node.
struct Component {
    in_node_index: usize,
    out_node_index: usize,
}

impl Component {
    fn new(in_node_index: usize, out_node_index: usize) -> Self {
        Component {
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
        object_file::{ObjectFile, MAIN_ROUTE_INDEX},
        AnreError,
    };

    use super::{compile_from_anre, compile_from_regex};

    fn generate_routes(anre: &str, regex: &str) -> [ObjectFile; 2] {
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
            assert!(route.routes[MAIN_ROUTE_INDEX].is_fixed_start_position);
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
            assert!(!route.routes[MAIN_ROUTE_INDEX].is_fixed_start_position);
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
        for route in generate_routes(r#"name('a', "foo"), 'b'.name("bar")"#, r#"(?<foo>a)(?<bar>b)"#) {
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
            r#"name(('a', char_digit), "foo"), ('x' || 'y').name("bar")"#,
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
            let route = compile_from_anre(r#"name(name('a', "foo"), "bar")"#).unwrap();
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
            let route = compile_from_anre(r#"'a'.name("foo").name("bar")"#).unwrap();
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
            r#"'a'.name("foo"), 'b', foo"#, // anre
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
