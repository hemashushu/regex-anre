// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::transition::Transition;

pub const MAIN_LINE_INDEX: usize = 0;

// route
// |-- line --\
// |          |-- node --\
// |          |          |- transition item
// |          |          |- transition item
// |          |          |- ...
// |          |          \- transition item
// |          |-- node
// |          |-- ...
// |          |-- node
// |
// |-- line

// the compile target
pub struct Route {
    pub lines: Vec<Line>,
    pub capture_groups: Vec<CaptureGroup>,
}

pub struct Line {
    pub nodes: Vec<Node>,
    pub start_node_index: usize,
    pub end_node_index: usize,

    // it is true when the expression starts with `^`, or encounters is_after and is_before
    pub fixed_start: bool,

    // it is true when the expression ends with `$`, or encounters is_after
    pub fixed_end: bool,
}

// every node has one or more transitions,
// except the exit node has no transition.
pub struct Node {
    pub transition_items: Vec<TransitionItem>,
}

pub struct TransitionItem {
    pub transition: Transition,   // the type of transition
    pub target_node_index: usize, // the index of next node
}

pub struct CaptureGroup {
    pub name: Option<String>,
}

impl Route {
    pub fn new() -> Self {
        Route {
            lines: vec![],
            capture_groups: vec![],
        }
    }

    pub fn new_line(&mut self) -> usize {
        let line = Line {
            nodes: vec![],
            start_node_index: 0,
            end_node_index: 0,
            fixed_start: false,
            fixed_end: false,
        };

        let idx = self.lines.len();
        self.lines.push(line);
        idx
    }

    pub fn new_capture_group(&mut self, name: Option<String>) -> usize {
        let idx = self.capture_groups.len();
        self.capture_groups.push(CaptureGroup { name });
        idx
    }

    pub fn get_capture_group_index_by_name(&self, name: &str) -> Option<usize> {
        self.capture_groups.iter().position(|e| match &e.name {
            Some(n) => n == name,
            None => false,
        })
    }

    pub fn get_capture_group_name_by_index(&self, index:usize) -> Option<&str> {
        let n = &self.capture_groups[index].name;
        if let Some(m) = n {
            Some(m.as_str())
        }else {
            None
        }
    }
    // pub fn get_capture_group_names(&self) -> Vec<Option<&String>> {
    //     self.capture_groups
    //         .iter()
    //         .map(|item| {
    //             if let Some(name) = &item.name {
    //                 Some(name)
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect()
    // }

    // pub fn get_number_of_capture_groups(&self) -> usize {
    //     self.capture_groups.len()
    // }

    // pub fn get_number_of_counters(&self) -> usize {
    //     self.number_of_counters
    // }

    // for debug
    pub fn get_debug_text(&self) -> String {
        let mut ss = vec![];

        // lines
        if self.lines.len() == 1 {
            ss.push(self.lines[0].get_debug_text());
        } else {
            for (node_index, line) in self.lines.iter().enumerate() {
                ss.push(format!("= ${}", node_index));
                ss.push(line.get_debug_text());
            }
        }

        // capture groups
        for (capture_group_index, capture_group) in self.capture_groups.iter().enumerate() {
            let s = if let Some(name) = &capture_group.name {
                format!("# {{{}}}, {}", capture_group_index, name)
            } else {
                format!("# {{{}}}", capture_group_index)
            };
            ss.push(s);
        }

        ss.join("\n")
    }
}

impl Default for Route {
    fn default() -> Self {
        Self::new()
    }
}

impl Line {
    // return the index of the new node
    pub fn new_node(&mut self) -> usize {
        let node = Node {
            transition_items: vec![],
        };
        let idx = self.nodes.len();
        self.nodes.push(node);

        idx
    }

    pub fn append_transition(
        &mut self,
        source_node_index: usize,
        target_node_index: usize,
        transition: Transition,
    ) -> usize {
        let transition_node = TransitionItem {
            transition,
            target_node_index,
        };

        let idx = self.nodes[source_node_index].transition_items.len();

        self.nodes[source_node_index]
            .transition_items
            .push(transition_node);

        idx
    }

    // for debug
    pub fn get_debug_text(&self) -> String {
        let mut ss = vec![];

        for (node_index, node) in self.nodes.iter().enumerate() {
            // node
            let prefix = if node_index == self.start_node_index {
                '>'
            } else if node_index == self.end_node_index {
                '<'
            } else {
                '-'
            };

            let s = format!("{} {}", prefix, node_index);
            ss.push(s);

            // transition items
            for transition_item in &node.transition_items {
                let s = format!(
                    "  -> {}, {}",
                    transition_item.target_node_index, transition_item.transition
                );
                ss.push(s);
            }
        }

        ss.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::{assert_eq, assert_str_eq};

    use crate::{
        route::Route,
        transition::{CharTransition, Transition},
    };

    #[test]
    fn test_route_new_line() {
        let mut route = Route::new();

        // create a line
        {
            let line_index = route.new_line();
            let line = &mut route.lines[line_index];

            // create a node
            line.new_node();

            assert_str_eq!(
                line.get_debug_text(),
                "\
> 0"
            );

            // create other nodes
            let idx1 = line.new_node();
            let _idx2 = line.new_node();
            let idx3 = line.new_node();

            line.start_node_index = idx1;
            line.end_node_index = idx3;

            assert_str_eq!(
                line.get_debug_text(),
                "\
- 0
> 1
- 2
< 3"
            );
        }

        // create another line
        {
            let line_index = route.new_line();
            let line = &mut route.lines[line_index];

            // create a node
            line.new_node();

            assert_str_eq!(
                route.get_debug_text(),
                "\
= $0
- 0
> 1
- 2
< 3
= $1
> 0"
            );
        }
    }

    #[test]
    fn test_route_new_capture_group() {
        let mut route = Route::new();
        let line_index = route.new_line();
        let line = &mut route.lines[line_index];

        line.new_node();
        line.new_node();

        route.new_capture_group(None);
        route.new_capture_group(Some("foo".to_owned()));
        route.new_capture_group(None);

        assert_str_eq!(
            route.get_debug_text(),
            "\
> 0
- 1
# {0}
# {1}, foo
# {2}"
        );

        assert_eq!(route.get_capture_group_index_by_name("foo"), Some(1));
        assert!(route.get_capture_group_index_by_name("bar").is_none());
    }

    #[test]
    fn test_line_append_transition() {
        let mut route = Route::new();
        let line_index = route.new_line();
        let line = &mut route.lines[line_index];

        let node_idx0 = line.new_node();
        let node_idx1 = line.new_node();
        let node_idx2 = line.new_node();
        let node_idx3 = line.new_node();

        let trans_idx0 = line.append_transition(
            node_idx0,
            node_idx1,
            Transition::Char(CharTransition::new('a')),
        );

        assert_str_eq!(
            line.get_debug_text(),
            "\
> 0
  -> 1, Char 'a'
- 1
- 2
- 3"
        );

        assert_eq!(trans_idx0, 0);

        let trans_idx1 = line.append_transition(
            node_idx0,
            node_idx2,
            Transition::Char(CharTransition::new('b')),
        );

        let trans_idx2 = line.append_transition(
            node_idx0,
            node_idx3,
            Transition::Char(CharTransition::new('c')),
        );

        assert_str_eq!(
            line.get_debug_text(),
            "\
> 0
  -> 1, Char 'a'
  -> 2, Char 'b'
  -> 3, Char 'c'
- 1
- 2
- 3"
        );

        assert_eq!(trans_idx1, 1);
        assert_eq!(trans_idx2, 2);

        let trans_idx3 = line.append_transition(
            node_idx1,
            node_idx2,
            Transition::Char(CharTransition::new('x')),
        );

        assert_str_eq!(
            line.get_debug_text(),
            "\
> 0
  -> 1, Char 'a'
  -> 2, Char 'b'
  -> 3, Char 'c'
- 1
  -> 2, Char 'x'
- 2
- 3"
        );

        assert_eq!(trans_idx3, 0);
    }
}
