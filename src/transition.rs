// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use crate::{
    ast::AssertionName,
    instance::{Instance, MatchRange},
    utf8reader::read_char,
};

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

// Jump
pub struct JumpTransition;

pub struct CharTransition {
    pub codepoint: u32,
    pub byte_length: usize,
}

// There is only `char_any` currently
pub struct SpecialCharTransition;

pub struct StringTransition {
    pub codepoints: Vec<u32>,
    pub byte_length: usize,
}

pub struct CharSetTransition {
    pub items: Vec<CharSetItem>,
    pub negative: bool,
}

pub enum CharSetItem {
    Char(u32),
    Range(CharRange),
}

pub struct CharRange {
    pub start: u32,
    pub end_included: u32,
}

pub struct BackReferenceTransition {
    pub capture_group_index: usize,
}

pub struct AssertionTransition {
    pub name: AssertionName,
}

pub struct CaptureStartTransition {
    pub capture_group_index: usize,
}

pub struct CaptureEndTransition {
    pub capture_group_index: usize,
}

pub struct CounterResetTransition;

pub struct CounterSaveTransition;

pub struct CounterIncTransition;

pub struct CounterCheckTransition {
    pub repetition_type: RepetitionType,
}

pub struct RepetitionTransition {
    pub repetition_type: RepetitionType,
}

pub struct LookAheadAssertionTransition {
    pub line_index: usize,
    pub negative: bool,
}

pub struct LookBehindAssertionTransition {
    pub line_index: usize,
    pub negative: bool,
    pub pattern_chars_length: usize,
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
        StringTransition { codepoints: chars, byte_length }
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
        CounterCheckTransition {
            repetition_type,
        }
    }
}

impl RepetitionTransition {
    pub fn new(repetition_type: RepetitionType) -> Self {
        RepetitionTransition {
            repetition_type,
        }
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
    pub fn new(line_index: usize, negative: bool, pattern_chars_length: usize) -> Self {
        LookBehindAssertionTransition {
            line_index,
            negative,
            pattern_chars_length,
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
        // let (codepoint, _) = utf8reader::read_char(&self.bytes, 0);
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
        // let data = self.bytes.clone();
        // write!(f, "String \"{}\"", String::from_utf8(data).unwrap())
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
                    // let (codepoint, _) = utf8reader::read_char(bytes, 0);
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
        // write!(f, "Counter reset %{}", self.counter_index)
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
        // write!(f, "Counter inc %{}", self.counter_index)
        f.write_str("Counter inc")
    }
}

impl Display for CounterCheckTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            // "Counter check %{}, {}",
            "Counter check {}",
            // self.counter_index,
            self.repetition_type
        )
    }
}

impl Display for RepetitionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            // "Repetition %{}, {}",
            "Repetition {}",
            // self.counter_index,
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
                "Look behind negative ${}, pattern length {}",
                self.line_index, self.pattern_chars_length
            )
        } else {
            write!(
                f,
                "Look behind ${}, pattern length {}",
                self.line_index, self.pattern_chars_length
            )
        }
    }
}

impl Transition {
    pub fn check(
        &self,
        instance: &mut Instance,
        position: usize,
        repetition_count: usize,
    ) -> CheckResult {
        match self {
            Transition::Jump(_) => {
                // always success
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
                    let (current_char, byte_length) = get_char(instance, position);
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

                let (current_char, byte_length) = get_char(instance, position);
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
                let success = match transition.name {
                    AssertionName::Start => is_first_char(instance, position),
                    AssertionName::End => is_end(instance, position),
                    AssertionName::IsBound => is_word_bound(instance, position),
                    AssertionName::IsNotBound => !is_word_bound(instance, position),
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
            Transition::CounterReset(_) => {
                CheckResult::Success(0, 0)
            }
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
            Transition::LookAheadAssertion(_transition) => todo!(),
            Transition::LookBehindAssertion(_transition) => todo!(),
        }
    }
}

#[inline]
fn get_char(instance: &Instance, position: usize) -> (u32, usize) {
    read_char(instance.bytes, position)
}

#[inline]
fn is_first_char(_instance: &Instance, position: usize) -> bool {
    position == 0
}

#[inline]
fn is_end(instance: &Instance, position: usize) -> bool {
    let total_byte_length = instance.bytes.len();
    position >= total_byte_length
}

fn is_word_bound(instance: &Instance, position: usize) -> bool {
    if instance.bytes.is_empty() {
        false
    } else if position == 0 {
        let (current_char, _) = get_char(instance, position);
        is_word_char(current_char)
    } else if position >= instance.bytes.len() {
        let (previous_char, _) = get_char(instance, position - 1);
        is_word_char(previous_char)
    } else {
        let (current_char, _) = get_char(instance, position);
        let (previous_char, _) = get_char(instance, position - 1);

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
