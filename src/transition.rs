// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use crate::{
    ast::AssertionName,
    instance::{Instance, MatchRange},
    process::new_thread,
    route::Route,
    utf8reader::{read_char, read_previous_char},
};

#[derive(Debug)]
pub enum Transition {
    Jump(JumpTransition),
    Char(CharTransition),
    SpecialChar(SpecialCharTransition),
    String(StringTransition),
    CharSet(CharSetTransition),
    BackReference(BackReferenceTransition),
    Assertion(AssertionTransition),

    // capture
    CaptureStart(CaptureStartTransition),
    CaptureEnd(CaptureEndTransition),

    // reset the associated counter and the list of anchors
    CounterReset(CounterResetTransition),
    CounterSave(CounterSaveTransition),
    CounterInc(CounterIncTransition),
    CounterCheck(CounterCheckTransition),
    Repetition(RepetitionTransition),

    // assertion
    LookAheadAssertion(LookAheadAssertionTransition),
    LookBehindAssertion(LookBehindAssertionTransition),
}

#[derive(Debug)]
pub struct JumpTransition;

#[derive(Debug)]
pub struct CharTransition {
    pub codepoint: u32,
    pub byte_length: usize,
}

// There is only `char_any` currently
#[derive(Debug)]
pub struct SpecialCharTransition;

#[derive(Debug)]
pub struct StringTransition {
    pub codepoints: Vec<u32>,
    pub byte_length: usize,
}

#[derive(Debug)]
pub struct CharSetTransition {
    pub items: Vec<CharSetItem>,
    pub negative: bool,
}

#[derive(Debug)]
pub enum CharSetItem {
    Char(u32),
    Range(CharRange),
}

#[derive(Debug)]
pub struct CharRange {
    pub start: u32,
    pub end_included: u32,
}

#[derive(Debug)]
pub struct BackReferenceTransition {
    pub capture_group_index: usize,
}

#[derive(Debug)]
pub struct AssertionTransition {
    pub name: AssertionName,
}

#[derive(Debug)]
pub struct CaptureStartTransition {
    pub capture_group_index: usize,
}

#[derive(Debug)]
pub struct CaptureEndTransition {
    pub capture_group_index: usize,
}

#[derive(Debug)]
pub struct CounterResetTransition;

#[derive(Debug)]
pub struct CounterSaveTransition;

#[derive(Debug)]
pub struct CounterIncTransition;

#[derive(Debug)]
pub struct CounterCheckTransition {
    pub repetition_type: RepetitionType,
}

#[derive(Debug)]
pub struct RepetitionTransition {
    pub repetition_type: RepetitionType,
}

#[derive(Debug)]
pub struct LookAheadAssertionTransition {
    pub line_index: usize,
    pub negative: bool,
}

#[derive(Debug)]
pub struct LookBehindAssertionTransition {
    pub line_index: usize,
    pub negative: bool,
    pub match_length_in_char: usize,
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
        let byte_length = s.as_bytes().len();
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

impl BackReferenceTransition {
    pub fn new(capture_group_index: usize) -> Self {
        BackReferenceTransition {
            capture_group_index,
        }
    }
}

impl AssertionTransition {
    pub fn new(name: AssertionName) -> Self {
        AssertionTransition { name }
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
    pub fn new(line_index: usize, negative: bool) -> Self {
        LookAheadAssertionTransition {
            line_index,
            negative,
        }
    }
}

impl LookBehindAssertionTransition {
    pub fn new(line_index: usize, negative: bool, match_length_in_char: usize) -> Self {
        LookBehindAssertionTransition {
            line_index,
            negative,
            match_length_in_char,
        }
    }
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::Jump(j) => write!(f, "{}", j),
            Transition::Char(c) => write!(f, "{}", c),
            Transition::String(s) => write!(f, "{}", s),
            Transition::CharSet(c) => write!(f, "{}", c),
            Transition::SpecialChar(s) => write!(f, "{}", s),
            Transition::BackReference(b) => write!(f, "{}", b),
            Transition::Assertion(a) => write!(f, "{}", a),
            Transition::CaptureStart(m) => write!(f, "{}", m),
            Transition::CaptureEnd(m) => write!(f, "{}", m),
            Transition::CounterReset(c) => write!(f, "{}", c),
            Transition::CounterSave(c) => write!(f, "{}", c),
            Transition::CounterInc(c) => write!(f, "{}", c),
            Transition::CounterCheck(c) => write!(f, "{}", c),
            Transition::Repetition(r) => write!(f, "{}", r),
            Transition::LookAheadAssertion(l) => write!(f, "{}", l),
            Transition::LookBehindAssertion(l) => write!(f, "{}", l),
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

impl Display for AssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assertion \"{}\"", self.name)
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
        write!(
            f,
            "Counter check {}",
            self.repetition_type
        )
    }
}

impl Display for RepetitionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Repetition {}",
            self.repetition_type
        )
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
            write!(f, "Look ahead negative ${}", self.line_index)
        } else {
            write!(f, "Look ahead ${}", self.line_index)
        }
    }
}

impl Display for LookBehindAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(
                f,
                "Look behind negative ${}, match length {}",
                self.line_index, self.match_length_in_char
            )
        } else {
            write!(
                f,
                "Look behind ${}, match length {}",
                self.line_index, self.match_length_in_char
            )
        }
    }
}

impl Transition {
    pub fn check(
        &self,
        instance: &mut Instance,
        route: &Route,
        position: usize,
        repetition_count: usize,
    ) -> CheckResult {
        match self {
            Transition::Jump(_) => {
                // jumping transition always success
                CheckResult::Success(0, 0)
            }
            Transition::Char(transition) => {
                let thread = instance.get_current_thread_ref();

                if position >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let (cp, _) = read_char(instance.bytes, position);
                    if cp == transition.codepoint {
                        CheckResult::Success(transition.byte_length, 0)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::SpecialChar(_) => {
                // 'special char' currently contains only the 'char_any'.
                //
                // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
                // \n, \r, \u2028 or \u2029

                let thread = instance.get_current_thread_ref();

                if position >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let (current_char, byte_length) = get_char(instance.bytes, position);
                    if current_char != '\n' as u32 && current_char != '\r' as u32 {
                        CheckResult::Success(byte_length, 0)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::String(transition) => {
                let thread = instance.get_current_thread_ref();

                if position + transition.byte_length > thread.end_position {
                    CheckResult::Failure
                } else {
                    let mut is_same = true;
                    let mut current_position: usize = position;

                    for codepoint in &transition.codepoints {
                        let (cp, length) = read_char(instance.bytes, current_position);
                        if *codepoint != cp {
                            is_same = false;
                            break;
                        }
                        current_position += length;
                    }

                    if is_same {
                        CheckResult::Success(transition.byte_length, 0)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::CharSet(transition) => {
                let thread = instance.get_current_thread_ref();

                if position >= thread.end_position {
                    return CheckResult::Failure;
                }

                let (current_char, byte_length) = get_char(instance.bytes, position);
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
                    CheckResult::Success(byte_length, 0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::BackReference(transition) => {
                let MatchRange { start, end } =
                    &instance.match_ranges[transition.capture_group_index];

                let bytes = &instance.bytes[*start..*end];
                let byte_length = end - start;

                let thread = instance.get_current_thread_ref();

                if position + byte_length >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let mut is_same = true;

                    for (idx, c) in bytes.iter().enumerate() {
                        if c != &instance.bytes[idx + position] {
                            is_same = false;
                            break;
                        }
                    }

                    if is_same {
                        CheckResult::Success(byte_length, 0)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::Assertion(transition) => {
                let bytes = instance.bytes;
                let success = match transition.name {
                    AssertionName::Start => is_first_char(position),
                    AssertionName::End => is_end(bytes, position),
                    AssertionName::IsBound => is_word_bound(bytes, position),
                    AssertionName::IsNotBound => !is_word_bound(bytes, position),
                };

                if success {
                    CheckResult::Success(0, 0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::CaptureStart(transition) => {
                instance.match_ranges[transition.capture_group_index].start = position;
                CheckResult::Success(0, 0)
            }
            Transition::CaptureEnd(transition) => {
                instance.match_ranges[transition.capture_group_index].end = position;
                CheckResult::Success(0, 0)
            }
            Transition::CounterReset(_) => CheckResult::Success(0, 0),
            Transition::CounterSave(_) => {
                instance.counter_stack.push(repetition_count);
                CheckResult::Success(0, 0)
            }
            Transition::CounterInc(_) => {
                let last_count = instance.counter_stack.pop().unwrap();
                CheckResult::Success(0, last_count + 1)
            }
            Transition::CounterCheck(transition) => {
                let can_forward = match transition.repetition_type {
                    RepetitionType::Specified(m) => repetition_count == m,
                    RepetitionType::Range(from, to) => {
                        repetition_count >= from && repetition_count <= to
                    }
                };
                if can_forward {
                    CheckResult::Success(0, repetition_count)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::Repetition(transition) => {
                let can_backward = match transition.repetition_type {
                    RepetitionType::Specified(times) => repetition_count < times,
                    RepetitionType::Range(_, to) => repetition_count < to,
                };
                if can_backward {
                    CheckResult::Success(0, repetition_count)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::LookAheadAssertion(transition) => {
                let line_index = transition.line_index;
                let thread_result =
                    new_thread(instance, route, line_index, position, instance.bytes.len());

                let result = thread_result ^ transition.negative;
                if result {
                    // assertion should not move the position of parent thread
                    const NO_FORWARD: usize = 0;
                    CheckResult::Success(NO_FORWARD, 0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::LookBehindAssertion(transition) => {
                let line_index = transition.line_index;
                let thread_result = if let Ok(start) = get_position_by_chars_backward(
                    instance.bytes,
                    position,
                    transition.match_length_in_char,
                ) {
                    // the child thread should start at position "current_position - backword_count_in_bytes".
                    new_thread(instance, route, line_index, start, instance.bytes.len())
                } else {
                    false
                };

                let result = thread_result ^ transition.negative;
                if result {
                    // assertion should not move the position of parent thread
                    const NO_FORWARD: usize = 0;
                    CheckResult::Success(NO_FORWARD, 0)
                } else {
                    CheckResult::Failure
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

pub enum CheckResult {
    Success(
        /* forward bytes */ usize,
        /* repetition count */ usize,
    ),
    Failure,
}
