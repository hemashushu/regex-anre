// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::ops::{Index, Range};

use crate::{
    compiler::compile_from_str,
    error::Error,
    instance::{Instance, Thread},
    route::{Route, MAIN_LINE_INDEX},
    transition::CheckResult,
    utf8reader::read_char,
};

pub struct Regex {
    pub route: Route,
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let route = compile_from_str(pattern)?;
        Ok(Regex { route })
    }

    pub fn from_anre(expression: &str) -> Result<Self, Error> {
        let route = compile_from_str(expression)?;
        Ok(Regex { route })
    }

    pub fn find<'a, 'b>(&'a self, text: &'b str) -> Option<Match<'a, 'b>> {
        let bytes = text.as_bytes();
        let number_of_capture_groups = self.route.capture_groups.len();
        let mut instance = Instance::from_bytes(bytes, number_of_capture_groups);

        if !instance.exec(&self.route, 0) {
            return None;
        }

        let match_range = &instance.match_ranges[0];
        let match_ = Match::new(
            match_range.start,
            match_range.end,
            self.route.get_capture_group_name_by_index(0),
            sub_string(bytes, match_range.start, match_range.end),
        );

        Some(match_)
    }

    pub fn find_iter<'a, 'b>(&'a self, text: &'b str) -> Matches<'a, 'b> {
        let bytes = text.as_bytes();
        let number_of_capture_groups = self.route.capture_groups.len();
        let instance = Instance::from_bytes(bytes, number_of_capture_groups);

        Matches::new(&self.route, instance)
    }

    pub fn captures<'a, 'b>(&'a self, text: &'b str) -> Option<Captures<'a, 'b>> {
        let bytes = text.as_bytes();
        let number_of_capture_groups = self.route.capture_groups.len();
        let mut instance = Instance::from_bytes(bytes, number_of_capture_groups);

        if !instance.exec(&self.route, 0) {
            return None;
        }

        let matches: Vec<Match> = instance
            .match_ranges
            .iter()
            .enumerate()
            .map(|(idx, match_range)| {
                Match::new(
                    match_range.start,
                    match_range.end,
                    self.route.get_capture_group_name_by_index(idx),
                    sub_string(bytes, match_range.start, match_range.end),
                )
            })
            .collect();

        Some(Captures { matches })
    }

    pub fn captures_iter<'a, 'b>(&'a self, text: &'b str) -> CaptureMatches<'a, 'b> {
        let bytes = text.as_bytes();
        let number_of_capture_groups = self.route.capture_groups.len();
        let instance = Instance::from_bytes(bytes, number_of_capture_groups);

        CaptureMatches::new(&self.route, instance)
    }

    pub fn is_match(&self, text: &str) -> bool {
        let bytes = text.as_bytes();
        let number_of_capture_groups = self.route.capture_groups.len();
        let mut instance = Instance::from_bytes(bytes, number_of_capture_groups);

        instance.exec(&self.route, 0)
    }
}

pub struct CaptureMatches<'a, 'b> {
    route: &'a Route,
    instance: Instance<'b>,
    last_position: usize,
}

impl<'a, 'b> CaptureMatches<'a, 'b> {
    fn new(route: &'a Route, instance: Instance<'b>) -> Self {
        CaptureMatches {
            route,
            instance,
            last_position: 0,
        }
    }
}

impl<'a, 'b> Iterator for CaptureMatches<'a, 'b> {
    type Item = Captures<'a, 'b>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.instance.exec(self.route, self.last_position) {
            return None;
        }

        let matches: Vec<Match> = self
            .instance
            .match_ranges
            .iter()
            .enumerate()
            .map(|(idx, match_range)| {
                Match::new(
                    match_range.start,
                    match_range.end,
                    self.route.get_capture_group_name_by_index(idx),
                    sub_string(self.instance.bytes, match_range.start, match_range.end),
                )
            })
            .collect();

        self.last_position = matches[0].end;

        Some(Captures { matches })
    }
}

pub struct Matches<'a, 'b> {
    route: &'a Route,
    instance: Instance<'b>,
    last_position: usize,
}

impl<'a, 'b> Matches<'a, 'b> {
    fn new(route: &'a Route, instance: Instance<'b>) -> Self {
        Matches {
            route,
            instance,
            last_position: 0,
        }
    }
}

impl<'a, 'b> Iterator for Matches<'a, 'b> {
    type Item = Match<'a, 'b>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.instance.exec(self.route, self.last_position) {
            return None;
        }

        let match_range = &self.instance.match_ranges[0];
        let match_ = Match::new(
            match_range.start,
            match_range.end,
            self.route.get_capture_group_name_by_index(0),
            sub_string(self.instance.bytes, match_range.start, match_range.end),
        );

        self.last_position = match_.end;

        Some(match_)
    }
}

impl<'a> Instance<'a> {
    pub fn exec(&mut self, route: &Route, start: usize) -> bool {
        let end = self.bytes.len();
        new_thread(self, route, MAIN_LINE_INDEX, start, end)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Captures<'a, 'b> {
    pub matches: Vec<Match<'a, 'b>>,
}

impl<'a, 'b> Captures<'a, 'b> {
    // the following methods are intended to
    // be compatible with the 'Captures' API of crate 'regex':
    // https://docs.rs/regex/latest/regex/struct.Captures.html

    pub fn get(&self, index: usize) -> Option<&Match> {
        self.matches.get(index)
    }

    pub fn name(&self, name: &str) -> Option<&Match> {
        // Option<Match> {
        self.matches.iter().find(|item| match item.name {
            Some(s) => s == name,
            None => false,
        })
    }

    // e.g.
    //
    // ```
    //   let c = re.find("...").next().unwrap();
    //   let (whole, [one, two, three]) = c.extract();
    // ```
    pub fn extract<const N: usize>(&self) -> (&str, [&str; N]) {
        let mut items: [&str; N] = [""; N];
        for (idx, item) in items.iter_mut().enumerate() {
            *item = self.matches[idx + 1].value;
        }
        (self.matches[0].value, items)
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, 'b> Index<usize> for Captures<'a, 'b> {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .unwrap_or_else(|| panic!(
                "Index {} is out of range of the capture group and the length of capture groups is {}.",
                index, self.len()))
            .as_str()
    }
}

impl<'a, 'b> Index<&str> for Captures<'a, 'b> {
    type Output = str;

    fn index(&self, name: &str) -> &Self::Output {
        self.name(name)
            .unwrap_or_else(|| panic!("Cannot find the capture group named \"{}\".", name))
            .as_str()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Match<'a, 'b> {
    pub start: usize, // the position of utf-8 byte stream (value included)
    pub end: usize,   // the position of utf-8 byte stream (value excluded)
    pub name: Option<&'a str>,
    pub value: &'b str,
}

impl<'a, 'b> Match<'a, 'b> {
    pub fn new(start: usize, end: usize, name: Option<&'a str>, value: &'b str) -> Self {
        Match {
            start,
            end,
            name,
            value,
        }
    }

    // the following methods are intended to
    // be compatible with the 'Match' API of crate 'regex':
    // https://docs.rs/regex/latest/regex/struct.Match.html

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn range(&self) -> Range<usize> {
        Range {
            start: self.start,
            end: self.end,
        }
    }

    pub fn as_str(&self) -> &'b str {
        self.value
    }
}

fn sub_string(bytes: &[u8], start: usize, end_excluded: usize) -> &str {
    /*
     * convert Vec<char> into String:
     * `let s:String = chars.iter().collect()`
     * or
     * `let s = String::from_iter(&chars)`
     */
    let slice = &bytes[start..end_excluded];
    core::str::from_utf8(slice).unwrap()
}

pub fn new_thread(
    instance: &mut Instance,
    route: &Route,
    line_index: usize,
    mut start: usize,
    end: usize,
) -> bool {
    let thread = Thread::new(start, end, line_index);
    instance.threads.push(thread);

    let mut result = false;
    while start < end {
        if run_thread(instance, route, start) {
            result = true;
            break;
        }

        if route.lines[line_index].fixed_start_position {
            break;
        }

        // move forward one character and try again
        let (_, byte_length) = read_char(instance.bytes, start);
        start += byte_length;
    }

    instance.threads.pop();
    result
}

fn run_thread(instance: &mut Instance, route: &Route, position: usize) -> bool {
    let (line_index, entry_node_index, exit_node_index) = {
        let thread = instance.get_current_thread_ref();
        let line_index = thread.line_index;
        let line = &route.lines[line_index];
        (line_index, line.start_node_index, line.end_node_index)
    };

    // add transitions of the entry node
    instance.append_transition_stack_frames_by_node(route, entry_node_index, position, 0);

    let mut result = false;

    while let Some(frame) = instance.get_current_thread_ref_mut().transition_stack.pop() {
        // get the transition
        let line = &route.lines[line_index];
        let node = &line.nodes[frame.current_node_index];
        let transition_item = &node.transition_items[frame.transition_index];

        let position = frame.position;
        let last_repetition_count = frame.repetition_count;
        let transition = &transition_item.transition;
        let target_node_index = transition_item.target_node_index;

        let check_result = transition.check(instance, route, position, last_repetition_count);
        match check_result {
            CheckResult::Success(move_forward, current_repetition_count) => {
                if target_node_index == exit_node_index {
                    result = true;
                    break;
                }

                instance.append_transition_stack_frames_by_node(
                    route,
                    target_node_index,
                    position + move_forward,
                    current_repetition_count,
                );
            }
            CheckResult::Failure => {
                // check next transition
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{Captures, Match, Regex};
    use pretty_assertions::assert_eq;

    fn new_match(start: usize, end: usize, value: &str) -> Match {
        Match::new(start, end, None, value)
    }

    fn new_captures<'a, 'b>(
        mes: &'a [(
            /*start:*/ usize,
            /*end:*/ usize,
            /*name:*/ Option<&'a str>,
            /*value:*/ &'b str,
        )],
    ) -> Captures<'a, 'b> {
        let matches: Vec<Match> = mes
            .iter()
            .map(|item| Match::new(item.0, item.1, item.2, item.3))
            .collect();

        Captures { matches }
    }

    #[test]
    fn test_process_char() {
        // exists in the middle and at the end of the text
        {
            let re = Regex::from_anre("'a'").unwrap();
            let mut matches = re.find_iter("babbaa");

            assert_eq!(matches.next(), Some(new_match(1, 2, "a")));
            assert_eq!(matches.next(), Some(new_match(4, 5, "a")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "a")));
            assert_eq!(matches.next(), None);
        }

        // exists in the middle and at the beginning of the text
        {
            let re = Regex::from_anre("'a'").unwrap();
            let mut matches = re.find_iter("abaabb");

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "a")));
            assert_eq!(matches.next(), Some(new_match(3, 4, "a")));
            assert_eq!(matches.next(), None);
        }

        // non-existent
        {
            let re = Regex::from_anre("'a'").unwrap();
            let mut matches = re.find_iter("xyz");

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_char_with_utf8() {
        // existent
        {
            let re = Regex::from_anre("'文'").unwrap();
            let mut matches = re.find_iter("abc中文字符文字🌏人文");

            assert_eq!(matches.next(), Some(new_match(6, 9, "文")));
            assert_eq!(matches.next(), Some(new_match(15, 18, "文")));
            assert_eq!(matches.next(), Some(new_match(28, 31, "文")));
            assert_eq!(matches.next(), None);
        }

        // non-existent
        {
            let re = Regex::from_anre("'文'").unwrap();
            let mut matches = re.find_iter("abc正则表达式🌏改");

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_string() {
        // existent
        {
            let re = Regex::from_anre("\"abc\"").unwrap();
            let text = "ababcbcabc";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(2, 5, "abc")));
            assert_eq!(matches.next(), Some(new_match(7, 10, "abc")));
            assert_eq!(matches.next(), None);
        }

        // non-existent
        {
            let re = Regex::from_anre("\"abc\"").unwrap();
            let text = "uvwxyz";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_string_with_utf8() {
        {
            let re = Regex::from_anre("\"文字\"").unwrap();
            let text = "abc文字文本象形文字🎁表情文字";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(3, 9, "文字")));
            assert_eq!(matches.next(), Some(new_match(21, 27, "文字")));
            assert_eq!(matches.next(), Some(new_match(37, 43, "文字")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_preset_charset() {
        {
            let re = Regex::from_anre("char_word").unwrap();
            let text = "a*1**_ **";
            //               "^ ^  ^   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "1")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "_")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("char_not_word").unwrap();
            let text = "!a@12 bc_";
            //               "v v  v   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "!")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "@")));
            assert_eq!(matches.next(), Some(new_match(5, 6, " ")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("char_digit").unwrap();
            let text = "1a2b_3de*";
            //               "^ ^  ^   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "1")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "2")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "3")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("char_not_digit").unwrap();
            let text = "a1_23 456";
            //               "v v  v   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "_")));
            assert_eq!(matches.next(), Some(new_match(5, 6, " ")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("char_space").unwrap();
            let text = " 1\tab\n_*!";
            //               "^ ^-  ^-   "
            //                012 345 678
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, " ")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "\t")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "\n")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("char_not_space").unwrap();
            let text = "a\t1\r\n*   ";
            //               "v  v    v   "
            //                01 23 4 5678
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "1")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_charset() {
        // chars
        {
            let re = Regex::from_anre("['a','b','c']").unwrap();
            let text = "adbefcghi";
            //               "^ ^  ^   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "b")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "c")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let re = Regex::from_anre("!['a','b','c']").unwrap();
            let text = "xa1bb*ccc";
            //               "v v  v   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "x")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "1")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }

        // range
        {
            let re = Regex::from_anre("['a'..'c']").unwrap();
            let text = "adbefcghi";
            //               "^ ^  ^   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "b")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "c")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let re = Regex::from_anre("!['a'..'c']").unwrap();
            let text = "xa1bb*ccc";
            //               "v v  v   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "x")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "1")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }

        // ranges
        {
            let re = Regex::from_anre("['a'..'f', '0'..'9']").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "1")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "f")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let re = Regex::from_anre("!['a'..'f', '0'..'9']").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "m")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "n")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }

        // combine range with preset
        {
            let re = Regex::from_anre("['a'..'f', char_digit]").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "1")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "f")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let re = Regex::from_anre("!['a'..'f', char_digit]").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "m")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "n")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }

        // nested
        {
            let re = Regex::from_anre("[['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "1")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "f")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let re = Regex::from_anre("![['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 1, "m")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "n")));
            assert_eq!(matches.next(), Some(new_match(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_charset_with_utf8() {
        {
            let re = Regex::from_anre("['文','字','🍅']").unwrap();
            let text = "abc正文写字🍉宋体字体🍅测试🍋";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(6, 9, "文")));
            assert_eq!(matches.next(), Some(new_match(12, 15, "字")));
            assert_eq!(matches.next(), Some(new_match(25, 28, "字")));
            assert_eq!(matches.next(), Some(new_match(31, 35, "🍅")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let re = Regex::from_anre("!['文','字','🍅']").unwrap();
            let text = "哦字文🍅文噢字🍅文文字字喔";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 3, "哦")));
            assert_eq!(matches.next(), Some(new_match(16, 19, "噢")));
            assert_eq!(matches.next(), Some(new_match(38, 41, "喔")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_special_char() {
        let re = Regex::from_anre("char_any").unwrap();
        let text = "\na\r\n1 \n";
        //               "  ^    ^^  "
        let mut matches = re.find_iter(text);

        assert_eq!(matches.next(), Some(new_match(1, 2, "a")));
        assert_eq!(matches.next(), Some(new_match(4, 5, "1")));
        assert_eq!(matches.next(), Some(new_match(5, 6, " ")));
        assert_eq!(matches.next(), None);
    }

    #[test]
    fn test_process_group() {
        // anre group = a sequence of patterns
        {
            let re = Regex::from_anre("'a', 'b', 'c'").unwrap();
            let text = "ababcbcabc";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(2, 5, "abc")));
            assert_eq!(matches.next(), Some(new_match(7, 10, "abc")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("'%', char_digit").unwrap();
            let text = "0123%567%9";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(4, 6, "%5")));
            assert_eq!(matches.next(), Some(new_match(8, 10, "%9")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("['+','-'], ('%', char_digit)").unwrap();
            let text = "%12+%56-%9";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(3, 6, "+%5")));
            assert_eq!(matches.next(), Some(new_match(7, 10, "-%9")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_logic_or() {
        // two
        {
            let re = Regex::from_anre("'a' || 'b'").unwrap();
            let text = "012a45b7a9";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(3, 4, "a")));
            assert_eq!(matches.next(), Some(new_match(6, 7, "b")));
            assert_eq!(matches.next(), Some(new_match(8, 9, "a")));
            assert_eq!(matches.next(), None);
        }

        // three
        {
            let re = Regex::from_anre(r#""abc" || "mn" || "xyz""#).unwrap();
            let text = "aabcmmnnxyzz";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(1, 4, "abc")));
            assert_eq!(matches.next(), Some(new_match(5, 7, "mn")));
            assert_eq!(matches.next(), Some(new_match(8, 11, "xyz")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_start_and_end_assertion() {
        {
            let re = Regex::from_anre("start, 'a'").unwrap();
            let text = "ab";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("'a', end").unwrap();
            let text = "ab";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("start, 'a'").unwrap();
            let text = "ba";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("'a', end").unwrap();
            let text = "ba";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(1, 2, "a")));
            assert_eq!(matches.next(), None);
        }

        // both 'start' and 'end'
        {
            let re = Regex::from_anre("start, 'a', end").unwrap();
            let text = "a";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        // both 'start' and 'end' - failed 1
        {
            let re = Regex::from_anre("start, 'a', end").unwrap();
            let text = "ab";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), None);
        }

        // both 'start' and 'end' - failed 2
        {
            let re = Regex::from_anre("start, 'a', end").unwrap();
            let text = "ba";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_boundary_assertion() {
        // matching 'boundary + char'
        {
            let re = Regex::from_anre("is_bound, 'a'").unwrap();
            let text = "ab";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("is_bound, 'a'").unwrap();
            let text = "a";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("is_bound, 'a'").unwrap();
            let text = " a";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(1, 2, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("is_bound, 'a'").unwrap();
            let text = "ba";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), None);
        }

        // matching 'char + boundary'
        {
            let re = Regex::from_anre("'a', is_bound").unwrap();
            let text = "ba";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(1, 2, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("'a', is_bound").unwrap();
            let text = "a";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("'a', is_bound").unwrap();
            let text = "a ";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre("'a', is_bound").unwrap();
            let text = "ab";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_optional() {
        // char optional
        {
            let re = Regex::from_anre("'a', 'b'?, 'c'").unwrap();
            let text = "ababccbacabc";
            //               "  ^^^  ^^vvv"
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(2, 5, "abc")));
            assert_eq!(matches.next(), Some(new_match(7, 9, "ac")));
            assert_eq!(matches.next(), Some(new_match(9, 12, "abc")));
            assert_eq!(matches.next(), None);
        }

        // char optional - greedy
        {
            let re = Regex::from_anre("'a', 'b', 'c'?").unwrap();
            let text = "abcabx";
            //               "^^^vv"
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 3, "abc")));
            assert_eq!(matches.next(), Some(new_match(3, 5, "ab")));
            assert_eq!(matches.next(), None);
        }

        // char optional - lazy
        {
            let re = Regex::from_anre("'a', 'b', 'c'??").unwrap();
            let text = "abcabx";
            //               "^^ ^^ "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 2, "ab")));
            assert_eq!(matches.next(), Some(new_match(3, 5, "ab")));
            assert_eq!(matches.next(), None);
        }

        // group optional
        {
            let re = Regex::from_anre("'a', ('b','c')?, 'd'").unwrap();
            let text = "abcabdacdabcdabacad";
            //               "         ^^^^    ^^"
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(9, 13, "abcd")));
            assert_eq!(matches.next(), Some(new_match(17, 19, "ad")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_repetition_specified() {
        // char repetition
        {
            let re = Regex::from_anre("'a'{3}").unwrap();
            let text = "abaabbaaabbbaaaaa";
            //               "      ^^^   ^^^  "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(6, 9, "aaa")));
            assert_eq!(matches.next(), Some(new_match(12, 15, "aaa")));
            assert_eq!(matches.next(), None);
        }

        // charset repetition
        {
            let re = Regex::from_anre("char_digit{3}").unwrap();
            let text = "a1ab12abc123abcd1234";
            //               "         ^^^    ^^^ "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(9, 12, "123")));
            assert_eq!(matches.next(), Some(new_match(16, 19, "123")));
            assert_eq!(matches.next(), None);
        }

        // group repetition
        {
            let re = Regex::from_anre("('a','b'){3}").unwrap();
            let text = "abbaababbaababababab";
            //               "          ^^^^^^    "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(10, 16, "ababab")));
            assert_eq!(matches.next(), None);
        }

        // repetition + other pattern
        {
            let re = Regex::from_anre("'a'{2}, char_digit").unwrap();
            let text = "abaabbaa1bb1aa123bb123a11b11";
            //               "      ^^^   ^^^             "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(6, 9, "aa1")));
            assert_eq!(matches.next(), Some(new_match(12, 15, "aa1")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_repetition_range() {
        // char repetition
        {
            let re = Regex::from_anre("'a'{1,3}").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "^ ^^  ^^^   ^^^v    "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 4, "aa")));
            assert_eq!(matches.next(), Some(new_match(6, 9, "aaa")));
            assert_eq!(matches.next(), Some(new_match(12, 15, "aaa")));
            assert_eq!(matches.next(), Some(new_match(15, 16, "a")));
            assert_eq!(matches.next(), None);
        }

        // char repetition lazy
        {
            let re = Regex::from_anre("'a'{1,3}?").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "^ ^v  ^v^   ^v^v    "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(2, 3, "a")));
            assert_eq!(matches.next(), Some(new_match(3, 4, "a")));
            assert_eq!(matches.next(), Some(new_match(6, 7, "a")));
            assert_eq!(matches.next(), Some(new_match(7, 8, "a")));
            // omit the follow up
        }

        // char repetition - to MAX
        {
            let re = Regex::from_anre("'a'{2,}").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "  ^^  ^^^   ^^^^    "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(2, 4, "aa")));
            assert_eq!(matches.next(), Some(new_match(6, 9, "aaa")));
            assert_eq!(matches.next(), Some(new_match(12, 16, "aaaa")));
            assert_eq!(matches.next(), None);
        }

        // char repetition - to MAX - lazy
        {
            let re = Regex::from_anre("'a'{2,}?").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "  ^^  ^^    ^^vv    "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(2, 4, "aa")));
            assert_eq!(matches.next(), Some(new_match(6, 8, "aa")));
            assert_eq!(matches.next(), Some(new_match(12, 14, "aa")));
            assert_eq!(matches.next(), Some(new_match(14, 16, "aa")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_optional_and_repetition_range() {
        // implicit
        {
            let re = Regex::from_anre("'a', 'b'{0,3}, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^ ^^^ ^^^^ ^^^^^       "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 2, "ac")));
            assert_eq!(matches.next(), Some(new_match(3, 6, "abc")));
            assert_eq!(matches.next(), Some(new_match(7, 11, "abbc")));
            assert_eq!(matches.next(), Some(new_match(12, 17, "abbbc")));
            assert_eq!(matches.next(), None);
        }

        // explicit
        {
            let re = Regex::from_anre("'a', ('b'{2,3})?, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^     ^^^^ ^^^^^       "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 2, "ac")));
            assert_eq!(matches.next(), Some(new_match(7, 11, "abbc")));
            assert_eq!(matches.next(), Some(new_match(12, 17, "abbbc")));
            assert_eq!(matches.next(), None);
        }

        // repetition specified
        {
            let re = Regex::from_anre("'a', ('b'{2})?, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^     ^^^^             "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 2, "ac")));
            assert_eq!(matches.next(), Some(new_match(7, 11, "abbc")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_repetition_char_any() {
        // repetition specified
        {
            let re = Regex::from_anre("char_any{3}").unwrap();
            let text = "abcdefgh";
            //               "^^^vvv  "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 3, "abc")));
            assert_eq!(matches.next(), Some(new_match(3, 6, "def")));
            assert_eq!(matches.next(), None);
        }

        // repetition range - to MAX
        {
            let re = Regex::from_anre("char_any+").unwrap();
            let text = "abcdefg";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 7, "abcdefg")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_repetition_backtracking() {
        // backtracking
        {
            let re = Regex::from_anre("start, 'a', char_any+, 'c'").unwrap();
            let text = "abbcmn";
            //               "^^^^  "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 4, "abbc")));
        }

        // backtracking - failed
        // because there is no char between 'a' and 'c'
        {
            let re = Regex::from_anre("start, 'a', char_any+, 'c'").unwrap();
            let text = "acmn";
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), None);
        }

        // backtracking - failed
        // because there is not enough char between 'a' and 'c'
        {
            let re = Regex::from_anre("start, 'a', char_any{3,}, 'c'").unwrap();
            let text = "abbcmn";
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), None);
        }

        // lazy repetition - no backtracking
        {
            let re = Regex::from_anre("'a', char_any+?, 'c'").unwrap();
            let text = "abbcmn";
            //               "^^^^  "
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 4, "abbc")));
        }

        // nested backtracking
        {
            let re = Regex::from_anre("start, 'a', char_any{2,}, 'c', char_any{2,}, 'e'").unwrap();
            let text = "a88c88ewwefg";
            let mut matches = re.find_iter(text);
            assert_eq!(matches.next(), Some(new_match(0, 10, "a88c88ewwe")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_capture() {
        // index
        {
            let re = Regex::from_anre(r#"("0x" || "0o" || "0b").index(), (char_digit+).index()"#)
                .unwrap();
            let text = "abc0x23def0o456xyz";

            let mut matches = re.captures_iter(text);

            assert_eq!(
                matches.next(),
                Some(new_captures(&[
                    (3, 7, None, "0x23"),
                    (3, 5, None, "0x"),
                    (5, 7, None, "23")
                ]))
            );

            assert_eq!(
                matches.next(),
                Some(new_captures(&[
                    (10, 15, None, "0o456"),
                    (10, 12, None, "0o"),
                    (12, 15, None, "456")
                ]))
            );
        }

        // named
        {
            let re = Regex::from_anre(
                r#"("0x" || "0o" || "0b").name(prefix), (char_digit+).name(number)"#,
            )
            .unwrap();
            let text = "abc0x23def0o456xyz";

            let mut matches = re.captures_iter(text);

            assert_eq!(
                matches.next(),
                Some(new_captures(&[
                    (3, 7, None, "0x23"),
                    (3, 5, Some("prefix"), "0x"),
                    (5, 7, Some("number"), "23")
                ]))
            );

            assert_eq!(
                matches.next(),
                Some(new_captures(&[
                    (10, 15, None, "0o456"),
                    (10, 12, Some("prefix"), "0o"),
                    (12, 15, Some("number"), "456")
                ]))
            );
        }

        // named - by Regex::captures_iter(...)
        {
            let re = Regex::from_anre(
                r#"("0x" || "0o" || "0b").name(prefix), (char_digit+).name(number)"#,
            )
            .unwrap();
            let text = "abc0x23def0o456xyz";

            let mut matches = re.captures_iter(text);
            let one = matches.next().unwrap();

            assert_eq!(one.len(), 3);

            // test 'Captures::get'
            assert_eq!(one.get(0).unwrap().as_str(), "0x23");
            assert_eq!(one.get(1).unwrap().as_str(), "0x");
            assert_eq!(one.get(2).unwrap().as_str(), "23");

            // test Captures number index trait
            assert_eq!(&one[0], "0x23");
            assert_eq!(&one[1], "0x");
            assert_eq!(&one[2], "23");

            // test 'Captures::name'
            assert_eq!(one.name("prefix").unwrap().as_str(), "0x");
            assert_eq!(one.name("number").unwrap().as_str(), "23");

            // test Captures str index trait
            assert_eq!(&one["prefix"], "0x");
            assert_eq!(&one["number"], "23");

            // test 'Captures::extract()'
            assert_eq!(("0x23", ["0x", "23"]), one.extract());
        }

        // named - by Regex::find_iter(...)
        {
            let re = Regex::from_anre(
                r#"("0x" || "0o" || "0b").name(prefix), (char_digit+).name(number)"#,
            )
            .unwrap();
            let text = "abc0x23def0o456xyz";

            let mut matches = re.find_iter(text);
            let one = matches.next().unwrap();
            let two = matches.next().unwrap();

            assert_eq!(one.as_str(), "0x23");
            assert_eq!(one.range(), 3..7);

            assert_eq!(two.as_str(), "0o456");
            assert_eq!(two.range(), 10..15);
        }
    }

    #[test]
    fn test_process_backreference() {
        {
            let re = Regex::from_anre(
                r#"
            ('<', (char_word+).name(tag), '>'),
            char_any+,
            ("</", tag, '>')
            "#,
            )
            .unwrap();
            let text = "zero<div>one<div>two</div>three</div>four";
            let mut matches = re.captures_iter(text);

            assert_eq!(
                matches.next(),
                Some(new_captures(&[
                    (4, 37, None, "<div>one<div>two</div>three</div>"),
                    (5, 8, Some("tag"), "div")
                ]))
            );
        }

        // backreference + lazy
        {
            let re = Regex::from_anre(
                r#"
            ('<', (char_word+).name(tag), '>'),
            char_any+?,
            ("</", tag, '>')
            "#,
            )
            .unwrap();
            let text = "zero<div>one<div>two</div>three</div>four";
            let mut matches = re.captures_iter(text);

            assert_eq!(
                matches.next(),
                Some(new_captures(&[
                    (4, 26, None, "<div>one<div>two</div>"),
                    (5, 8, Some("tag"), "div")
                ]))
            );
        }
    }

    #[test]
    fn test_process_lookbehind() {
        {
            let re = Regex::from_anre("char_digit.is_after(['a'..'f'])").unwrap();
            let text = "a1 22 f9 cc z3 b2";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(1, 2, "1")));
            assert_eq!(matches.next(), Some(new_match(7, 8, "9")));
            assert_eq!(matches.next(), Some(new_match(16, 17, "2")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre(
                r#"
            [char_digit, 'a'..'f']
                .repeat(2)
                .is_after("0x")
            "#,
            )
            .unwrap();
            let text = "13 0x17 0o19 0x23 29";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(5, 7, "17")));
            assert_eq!(matches.next(), Some(new_match(15, 17, "23")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let re = Regex::from_anre(
                r#"
            [char_digit, 'a'..'f']
                .repeat(2)
                .is_not_after("0x")
            "#,
            )
            .unwrap();
            let text = "13 0x17 0o19 0x23 29";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 2, "13")));
            assert_eq!(matches.next(), Some(new_match(10, 12, "19")));
            assert_eq!(matches.next(), Some(new_match(18, 20, "29")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_lookahead() {
        {
            let re = Regex::from_anre("is_bound, ['a'..'f'].is_before(char_digit)").unwrap();
            let text = "a1 22 f9 cc z3 b2";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 1, "a")));
            assert_eq!(matches.next(), Some(new_match(6, 7, "f")));
            assert_eq!(matches.next(), Some(new_match(15, 16, "b")));
            assert_eq!(matches.next(), None);
        }

        {
            let re = Regex::from_anre(
                r#"
                is_bound
                ['a'..'z']
                    .at_least(2)
                    .is_before("ing" || "ed")
                "#,
            )
            .unwrap();
            let text = "jump jumping aaaabbbbing push pushed fork";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(5, 9, "jump")));
            assert_eq!(matches.next(), Some(new_match(13, 21, "aaaabbbb")));
            assert_eq!(matches.next(), Some(new_match(30, 34, "push")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let re = Regex::from_anre(
                r#"
                is_bound
                ['a'..'z']
                    .repeat(4)
                    .is_not_before("ing" || "ed")
                "#,
            )
            .unwrap();
            let text = "jump jumping aaaabbbbing push pushed fork";
            let mut matches = re.find_iter(text);

            assert_eq!(matches.next(), Some(new_match(0, 4, "jump")));
            assert_eq!(matches.next(), Some(new_match(13, 17, "aaaa")));
            assert_eq!(matches.next(), Some(new_match(25, 29, "push")));
            assert_eq!(matches.next(), Some(new_match(37, 41, "fork")));
            assert_eq!(matches.next(), None);
        }
    }
}
