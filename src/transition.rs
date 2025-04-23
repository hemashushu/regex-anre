// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use std::fmt::Display;

use crate::{
    ast::{AnchorAssertionName, BoundaryAssertionName},
    context::{Context, MatchRange},
    object_file::ObjectFile,
    process::start_routine,
    utf8reader::{read_char, read_previous_char},
};

/// A `Transition` represents a state transition in a regex engine.
/// Each transition contains logic to match a specific pattern (e.g., a character or string).
/// When executed, it processes the input text starting from a given position
/// and returns the result: either a failure or a success with additional information
/// (e.g., how many characters to move forward).
#[derive(Debug)]
pub enum Transition {
    Jump(JumpTransition),
    Char(CharTransition),
    SpecialChar(SpecialCharTransition),
    String(StringTransition),
    CharSet(CharSetTransition),
    BackReference(BackReferenceTransition),
    AnchorAssertion(AnchorAssertionTransition),
    BoundaryAssertion(BoundaryAssertionTransition),

    // Capture group transitions
    CaptureStart(CaptureStartTransition),
    CaptureEnd(CaptureEndTransition),

    // Counter-related transitions
    CounterReset(CounterResetTransition),
    CounterSave(CounterSaveTransition),
    CounterInc(CounterIncTransition),
    CounterCheck(CounterCheckTransition),
    Repetition(RepetitionTransition),

    // Assertion transitions
    LookAheadAssertion(LookAheadAssertionTransition),
    LookBehindAssertion(LookBehindAssertionTransition),
}

/// Represents a transition that performs an unconditional jump.
#[derive(Debug)]
pub struct JumpTransition;

/// Represents a transition that matches a single character.
#[derive(Debug)]
pub struct CharTransition {
    pub codepoint: u32,     // Unicode codepoint of the character
    pub byte_length: usize, // Length of the character in bytes
}

// Represents a transition for special characters (e.g., any character).
#[derive(Debug)]
pub struct SpecialCharTransition;

/// Represents a transition that matches a specific string.
#[derive(Debug)]
pub struct StringTransition {
    pub codepoints: Vec<u32>, // Unicode codepoints of the string
    pub byte_length: usize,   // Total byte length of the string
}

/// Represents a transition that matches a set of characters or ranges.
#[derive(Debug)]
pub struct CharSetTransition {
    pub items: Vec<CharSetItem>, // List of characters or ranges
    pub negative: bool,          // Whether the set is negated
}

/// Represents an item in a character set, either a single character or a range.
#[derive(Debug)]
pub enum CharSetItem {
    Char(u32),        // A single character
    Range(CharRange), // A range of characters
}

/// Represents a range of characters (inclusive).
#[derive(Debug)]
pub struct CharRange {
    pub start: u32,        // Start of the range
    pub end_included: u32, // End of the range (inclusive)
}

/// Represents a transition that matches a backreference to a capture group.
#[derive(Debug)]
pub struct BackReferenceTransition {
    pub capture_group_index: usize, // Index of the capture group
}

/// Represents a transition that asserts an anchor (e.g., start or end of input).
#[derive(Debug)]
pub struct AnchorAssertionTransition {
    pub name: AnchorAssertionName, // Name of the anchor assertion
}

/// Represents a transition that asserts a boundary (e.g., word boundary).
#[derive(Debug)]
pub struct BoundaryAssertionTransition {
    pub name: BoundaryAssertionName, // Name of the boundary assertion
}

/// Represents the start of a capture group.
#[derive(Debug)]
pub struct CaptureStartTransition {
    pub capture_group_index: usize, // Index of the capture group
}

/// Represents the end of a capture group.
#[derive(Debug)]
pub struct CaptureEndTransition {
    pub capture_group_index: usize, // Index of the capture group
}

/// Represents a transition that resets a counter.
#[derive(Debug)]
pub struct CounterResetTransition;

/// Represents a transition that saves the current counter value.
#[derive(Debug)]
pub struct CounterSaveTransition;

/// Represents a transition that increments the counter.
#[derive(Debug)]
pub struct CounterIncTransition;

/// Represents a transition that checks the counter against a repetition condition.
#[derive(Debug)]
pub struct CounterCheckTransition {
    pub repetition_type: RepetitionType, // Type of repetition to check
}

/// Represents a transition for handling repetitions.
#[derive(Debug)]
pub struct RepetitionTransition {
    pub repetition_type: RepetitionType, // Type of repetition
}

/// Represents a lookahead assertion transition.
#[derive(Debug)]
pub struct LookAheadAssertionTransition {
    pub route_index: usize, // Index of the route to evaluate
    pub negative: bool,     // Whether the assertion is negative
}

/// Represents a lookbehind assertion transition.
#[derive(Debug)]
pub struct LookBehindAssertionTransition {
    pub route_index: usize,          // Index of the route to evaluate
    pub negative: bool,              // Whether the assertion is negative
    pub match_length_in_char: usize, // Length of the match in characters
}

impl CharTransition {
    pub fn new(c: char) -> Self {
        let byte_length = c.len_utf8();
        CharTransition {
            codepoint: (c as u32),
            byte_length,
        }
    }
}

impl StringTransition {
    pub fn new(s: &str) -> Self {
        let chars: Vec<u32> = s.chars().map(|item| item as u32).collect();
        let byte_length = s.len();
        StringTransition {
            codepoints: chars,
            byte_length,
        }
    }
}

impl CharSetItem {
    pub fn new_char(character: char) -> Self {
        CharSetItem::Char(character as u32)
    }

    pub fn new_range(start: char, end_included: char) -> Self {
        let char_range = CharRange {
            start: start as u32,
            end_included: end_included as u32,
        };
        CharSetItem::Range(char_range)
    }
}

impl CharSetTransition {
    pub fn new(items: Vec<CharSetItem>, negative: bool) -> Self {
        CharSetTransition { items, negative }
    }

    pub fn new_preset_word() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_word(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_not_word() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_word(&mut items);
        CharSetTransition::new(items, true)
    }

    pub fn new_preset_space() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_space(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_not_space() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_space(&mut items);
        CharSetTransition::new(items, true)
    }

    pub fn new_preset_digit() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_digit(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_not_digit() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_digit(&mut items);
        CharSetTransition::new(items, true)
    }

    pub fn new_preset_hex() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_hex(&mut items);
        CharSetTransition::new(items, false)
    }
}

pub fn add_char(items: &mut Vec<CharSetItem>, c: char) {
    items.push(CharSetItem::new_char(c));
}

pub fn add_range(items: &mut Vec<CharSetItem>, start: char, end_included: char) {
    items.push(CharSetItem::new_range(start, end_included));
}

pub fn add_preset_space(items: &mut Vec<CharSetItem>) {
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
    // [\f\n\r\t\v\u0020\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000\ufeff]
    add_char(items, ' ');
    add_char(items, '\t');
    add_char(items, '\r');
    add_char(items, '\n');
}

pub fn add_preset_word(items: &mut Vec<CharSetItem>) {
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
    // [A-Za-z0-9_]
    add_range(items, 'A', 'Z');
    add_range(items, 'a', 'z');
    add_range(items, '0', '9');
    add_char(items, '_');
}

pub fn add_preset_digit(items: &mut Vec<CharSetItem>) {
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
    // [0-9]
    add_range(items, '0', '9');
}

pub fn add_preset_hex(items: &mut Vec<CharSetItem>) {
    // [a-fA-F0-9]
    add_range(items, 'A', 'F');
    add_range(items, 'a', 'f');
    add_range(items, '0', '9');
}

impl BackReferenceTransition {
    pub fn new(capture_group_index: usize) -> Self {
        BackReferenceTransition {
            capture_group_index,
        }
    }
}

impl AnchorAssertionTransition {
    pub fn new(name: AnchorAssertionName) -> Self {
        AnchorAssertionTransition { name }
    }
}

impl BoundaryAssertionTransition {
    pub fn new(name: BoundaryAssertionName) -> Self {
        BoundaryAssertionTransition { name }
    }
}

impl CaptureStartTransition {
    pub fn new(capture_group_index: usize) -> Self {
        CaptureStartTransition {
            capture_group_index,
        }
    }
}

impl CaptureEndTransition {
    pub fn new(capture_group_index: usize) -> Self {
        CaptureEndTransition {
            capture_group_index,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RepetitionType {
    Specified(usize),
    Range(usize, usize),
}

impl CounterCheckTransition {
    pub fn new(repetition_type: RepetitionType) -> Self {
        CounterCheckTransition { repetition_type }
    }
}

impl RepetitionTransition {
    pub fn new(repetition_type: RepetitionType) -> Self {
        RepetitionTransition { repetition_type }
    }
}

impl LookAheadAssertionTransition {
    pub fn new(route_index: usize, negative: bool) -> Self {
        LookAheadAssertionTransition {
            route_index,
            negative,
        }
    }
}

impl LookBehindAssertionTransition {
    pub fn new(route_index: usize, negative: bool, match_length_in_char: usize) -> Self {
        LookBehindAssertionTransition {
            route_index,
            negative,
            match_length_in_char,
        }
    }
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::Jump(t) => write!(f, "{}", t),
            Transition::Char(t) => write!(f, "{}", t),
            Transition::String(t) => write!(f, "{}", t),
            Transition::CharSet(t) => write!(f, "{}", t),
            Transition::SpecialChar(t) => write!(f, "{}", t),
            Transition::BackReference(t) => write!(f, "{}", t),
            Transition::AnchorAssertion(t) => write!(f, "{}", t),
            Transition::BoundaryAssertion(t) => write!(f, "{}", t),
            Transition::CaptureStart(t) => write!(f, "{}", t),
            Transition::CaptureEnd(t) => write!(f, "{}", t),
            Transition::CounterReset(t) => write!(f, "{}", t),
            Transition::CounterSave(t) => write!(f, "{}", t),
            Transition::CounterInc(t) => write!(f, "{}", t),
            Transition::CounterCheck(t) => write!(f, "{}", t),
            Transition::Repetition(t) => write!(f, "{}", t),
            Transition::LookAheadAssertion(t) => write!(f, "{}", t),
            Transition::LookBehindAssertion(t) => write!(f, "{}", t),
        }
    }
}

impl Display for JumpTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Jump")
    }
}

impl Display for CharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = unsafe { char::from_u32_unchecked(self.codepoint) };
        write!(f, "Char '{}'", c)
    }
}

impl Display for SpecialCharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Any char")
    }
}

impl Display for StringTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        /*
         * convert Vec<char> into String:
         * `let s:String = chars.iter().collect()`
         * or
         * `let s = String::from_iter(&chars)`
         */
        let cs: Vec<char> = self
            .codepoints
            .iter()
            .map(|item| unsafe { char::from_u32_unchecked(*item) })
            .collect();
        let s = String::from_iter(&cs);
        write!(f, "String \"{}\"", s)
    }
}

impl Display for CharSetTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        for item in &self.items {
            let line = match item {
                CharSetItem::Char(codepoint) => {
                    let c = unsafe { char::from_u32_unchecked(*codepoint) };
                    match c {
                        '\t' => "'\\t'".to_owned(),
                        '\r' => "'\\r'".to_owned(),
                        '\n' => "'\\n'".to_owned(),
                        _ => format!("'{}'", c),
                    }
                }
                CharSetItem::Range(r) => {
                    let start = unsafe { char::from_u32_unchecked(r.start) };
                    let end_included = unsafe { char::from_u32_unchecked(r.end_included) };
                    format!("'{}'..'{}'", start, end_included)
                }
            };
            lines.push(line);
        }

        let content = lines.join(", ");
        if self.negative {
            write!(f, "Charset ![{}]", content)
        } else {
            write!(f, "Charset [{}]", content)
        }
    }
}

impl Display for BackReferenceTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Back reference {{{}}}", self.capture_group_index)
    }
}

impl Display for AnchorAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Anchor assertion \"{}\"", self.name)
    }
}

impl Display for BoundaryAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Boundary assertion \"{}\"", self.name)
    }
}

impl Display for CaptureStartTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Capture start {{{}}}", self.capture_group_index)
    }
}

impl Display for CaptureEndTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Capture end {{{}}}", self.capture_group_index)
    }
}

impl Display for CounterResetTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Counter reset")
    }
}

impl Display for CounterSaveTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Counter save")
    }
}

impl Display for CounterIncTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Counter inc")
    }
}

impl Display for CounterCheckTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Counter check {}", self.repetition_type)
    }
}

impl Display for RepetitionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Repetition {}", self.repetition_type)
    }
}

impl Display for RepetitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepetitionType::Specified(n) => write!(f, "times {}", n),
            RepetitionType::Range(m, n) => {
                if n == &usize::MAX {
                    write!(f, "from {} to MAX", m)
                } else {
                    write!(f, "from {} to {}", m, n)
                }
            }
        }
    }
}

impl Display for LookAheadAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(f, "Look ahead negative ${}", self.route_index)
        } else {
            write!(f, "Look ahead ${}", self.route_index)
        }
    }
}

impl Display for LookBehindAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(
                f,
                "Look behind negative ${}, match length {}",
                self.route_index, self.match_length_in_char
            )
        } else {
            write!(
                f,
                "Look behind ${}, match length {}",
                self.route_index, self.match_length_in_char
            )
        }
    }
}

impl Transition {
    pub fn execute(
        &self,
        context: &mut Context,
        object_file: &ObjectFile,

        // the current position, it is like a cursor in the original text.
        position: usize,

        // the current repetition number.
        //
        // it is used by `Transition::CounterSave`,
        // `Transition::CounterCheck` and `Transition::Repetition`.
        //
        // the following illustra a new complete repetition transition.
        //
        // ```diagram
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
        // ```
        repetition_count: usize,
    ) -> ExecuteResult {
        match self {
            Transition::Jump(_) => {
                // jumping transition always success,
                // jumping transition has no character movement.
                ExecuteResult::Success(0, 0)
            }
            Transition::Char(transition) => {
                let thread = context.get_current_routine_ref();

                if position >= thread.end_position {
                    ExecuteResult::Failure
                } else {
                    let (cp, _) = read_char(context.bytes, position);
                    if cp == transition.codepoint {
                        ExecuteResult::Success(transition.byte_length, 0)
                    } else {
                        ExecuteResult::Failure
                    }
                }
            }
            Transition::SpecialChar(_) => {
                // "special char" currently contains only the "char_any".
                //
                // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
                // \n, \r, \u2028 or \u2029

                let thread = context.get_current_routine_ref();

                if position >= thread.end_position {
                    ExecuteResult::Failure
                } else {
                    let (current_char, byte_length) = get_char(context.bytes, position);

                    // "char_any" does not include new-line characters.
                    if current_char != '\n' as u32 && current_char != '\r' as u32 {
                        ExecuteResult::Success(byte_length, 0)
                    } else {
                        ExecuteResult::Failure
                    }
                }
            }
            Transition::String(transition) => {
                let thread = context.get_current_routine_ref();

                if position + transition.byte_length > thread.end_position {
                    ExecuteResult::Failure
                } else {
                    let mut is_same = true;
                    let mut current_position: usize = position;

                    for codepoint in &transition.codepoints {
                        let (cp, length) = read_char(context.bytes, current_position);
                        if *codepoint != cp {
                            is_same = false;
                            break;
                        }
                        current_position += length;
                    }

                    if is_same {
                        ExecuteResult::Success(transition.byte_length, 0)
                    } else {
                        ExecuteResult::Failure
                    }
                }
            }
            Transition::CharSet(transition) => {
                let thread = context.get_current_routine_ref();

                if position >= thread.end_position {
                    return ExecuteResult::Failure;
                }

                let (current_char, byte_length) = get_char(context.bytes, position);
                let mut found: bool = false;

                for item in &transition.items {
                    found = match item {
                        CharSetItem::Char(c) => current_char == *c,
                        CharSetItem::Range(r) => {
                            current_char >= r.start && current_char <= r.end_included
                        }
                    };

                    if found {
                        break;
                    }
                }

                if found ^ transition.negative {
                    ExecuteResult::Success(byte_length, 0)
                } else {
                    ExecuteResult::Failure
                }
            }
            Transition::BackReference(transition) => {
                let MatchRange { start, end } =
                    &context.match_ranges[transition.capture_group_index];

                let bytes = &context.bytes[*start..*end];
                let byte_length = end - start;

                let thread = context.get_current_routine_ref();

                if position + byte_length >= thread.end_position {
                    ExecuteResult::Failure
                } else {
                    let mut is_same = true;

                    for (idx, c) in bytes.iter().enumerate() {
                        if c != &context.bytes[idx + position] {
                            is_same = false;
                            break;
                        }
                    }

                    if is_same {
                        ExecuteResult::Success(byte_length, 0)
                    } else {
                        ExecuteResult::Failure
                    }
                }
            }
            Transition::AnchorAssertion(transition) => {
                let bytes = context.bytes;
                let success = match transition.name {
                    AnchorAssertionName::Start => is_first_char(position),
                    AnchorAssertionName::End => is_end(bytes, position),
                };

                if success {
                    ExecuteResult::Success(0, 0)
                } else {
                    ExecuteResult::Failure
                }
            }
            Transition::BoundaryAssertion(transition) => {
                let bytes = context.bytes;
                let success = match transition.name {
                    BoundaryAssertionName::IsBound => is_word_bound(bytes, position),
                    BoundaryAssertionName::IsNotBound => !is_word_bound(bytes, position),
                };

                if success {
                    ExecuteResult::Success(0, 0)
                } else {
                    ExecuteResult::Failure
                }
            }
            Transition::CaptureStart(transition) => {
                context.match_ranges[transition.capture_group_index].start = position;
                ExecuteResult::Success(0, 0)
            }
            Transition::CaptureEnd(transition) => {
                context.match_ranges[transition.capture_group_index].end = position;
                ExecuteResult::Success(0, 0)
            }
            Transition::CounterReset(_) => ExecuteResult::Success(0, 0),
            Transition::CounterSave(_) => {
                context.counter_stack.push(repetition_count);
                ExecuteResult::Success(0, 0)
            }
            Transition::CounterInc(_) => {
                let last_count = context.counter_stack.pop().unwrap();
                ExecuteResult::Success(0, last_count + 1)
            }
            Transition::CounterCheck(transition) => {
                let can_forward = match transition.repetition_type {
                    RepetitionType::Specified(m) => repetition_count == m,
                    RepetitionType::Range(from, to) => {
                        repetition_count >= from && repetition_count <= to
                    }
                };
                if can_forward {
                    ExecuteResult::Success(0, repetition_count)
                } else {
                    ExecuteResult::Failure
                }
            }
            Transition::Repetition(transition) => {
                let can_backward = match transition.repetition_type {
                    RepetitionType::Specified(times) => repetition_count < times,
                    RepetitionType::Range(_, to) => repetition_count < to,
                };
                if can_backward {
                    ExecuteResult::Success(0, repetition_count)
                } else {
                    ExecuteResult::Failure
                }
            }
            Transition::LookAheadAssertion(transition) => {
                let route_index = transition.route_index;
                let thread_result = start_routine(
                    context,
                    object_file,
                    route_index,
                    position,
                    context.bytes.len(),
                );

                let result = thread_result ^ transition.negative;
                if result {
                    // assertion should not move the position of parent thread
                    const NO_FORWARD: usize = 0;
                    ExecuteResult::Success(NO_FORWARD, 0)
                } else {
                    ExecuteResult::Failure
                }
            }
            Transition::LookBehindAssertion(transition) => {
                let route_index = transition.route_index;
                let thread_result = if let Ok(start) = get_position_by_chars_backward(
                    context.bytes,
                    position,
                    transition.match_length_in_char,
                ) {
                    // the child thread should start at position "current_position - backword_count_in_bytes".
                    start_routine(
                        context,
                        object_file,
                        route_index,
                        start,
                        context.bytes.len(),
                    )
                } else {
                    false
                };

                let result = thread_result ^ transition.negative;
                if result {
                    // assertion should not move the position of parent thread
                    const NO_FORWARD: usize = 0;
                    ExecuteResult::Success(NO_FORWARD, 0)
                } else {
                    ExecuteResult::Failure
                }
            }
        }
    }
}

// return Err if the position it less than 0
fn get_position_by_chars_backward(
    bytes: &[u8],
    mut current_position: usize,
    backward_chars: usize,
) -> Result<usize, ()> {
    for _ in 0..backward_chars {
        if current_position == 0 {
            return Err(());
        }

        let (_, char_length_in_byte) = read_previous_char(bytes, current_position);
        current_position -= char_length_in_byte;
    }

    Ok(current_position)
}

#[inline]
fn get_char(bytes: &[u8], position: usize) -> (u32, usize) {
    read_char(bytes, position)
}

#[inline]
fn is_first_char(position: usize) -> bool {
    position == 0
}

#[inline]
fn is_end(bytes: &[u8], position: usize) -> bool {
    let total_byte_length = bytes.len();
    position >= total_byte_length
}

fn is_word_bound(bytes: &[u8], position: usize) -> bool {
    if bytes.is_empty() {
        false
    } else if position == 0 {
        let (current_char, _) = get_char(bytes, position);
        is_word_char(current_char)
    } else if position >= bytes.len() {
        let (previous_char, _) = get_char(bytes, position - 1);
        is_word_char(previous_char)
    } else {
        let (current_char, _) = get_char(bytes, position);
        let (previous_char, _) = get_char(bytes, position - 1);

        if is_word_char(current_char) {
            !is_word_char(previous_char)
        } else {
            is_word_char(previous_char)
        }
    }
}

fn is_word_char(c: u32) -> bool {
    (c >= 'a' as u32 && c <= 'z' as u32)
        || (c >= 'A' as u32 && c <= 'Z' as u32)
        || (c >= '0' as u32 && c <= '9' as u32)
        || (c == '_' as u32)
}

/// Represents the result of executing a transition.
pub enum ExecuteResult {
    Success(
        usize, // Number of bytes to move forward
        usize, // Updated repetition count
    ),
    Failure, // Indicates that the transition failed
}
