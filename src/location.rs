// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Location {
    pub index: usize,  // The character index in the text
    pub line: usize,   // The line number (0-based index)
    pub column: usize, // The column number (0-based index)
    pub length: usize, // The length of the text range; 0 for a single position
}

impl Location {
    /// Create a new `Location` representing a single position.
    pub fn new_position(/*unit: usize,*/ index: usize, line: usize, column: usize) -> Self {
        Self {
            // unit,
            index,
            line,
            column,
            length: 0,
        }
    }

    /// Create a new `Location` representing a range of text.
    pub fn new_range(
        /*unit: usize,*/ index: usize,
        line: usize,
        column: usize,
        length: usize,
    ) -> Self {
        Self {
            // unit,
            index,
            line,
            column,
            length,
        }
    }

    /// Create a range `Location` from a starting position and a length.
    pub fn from_position_and_length(position: &Location, length: usize) -> Self {
        Self::new_range(
            // position.unit,
            position.index,
            position.line,
            position.column,
            length,
        )
    }

    /// Create a range `Location` from two positions: start and end.
    pub fn from_position_pair(position_start: &Location, position_end: &Location) -> Self {
        Self::new_range(
            // position_start.unit,
            position_start.index,
            position_start.line,
            position_start.column,
            position_end.index - position_start.index,
        )
    }

    /// Create a range `Location` from two positions: start and end (inclusive).
    pub fn from_position_pair_with_end_included(
        position_start: &Location,
        position_end_included: &Location,
    ) -> Self {
        Self::new_range(
            // position_start.unit,
            position_start.index,
            position_start.line,
            position_start.column,
            position_end_included.index - position_start.index + 1,
        )
    }

    /// Combine two ranges into a single range `Location`.
    pub fn from_range_pair(range_start: &Location, range_end: &Location) -> Self {
        Self::new_range(
            // range_start.unit,
            range_start.index,
            range_start.line,
            range_start.column,
            range_end.index - range_start.index + range_end.length,
        )
    }

    /// Get the starting position of a range as a `Location`.
    pub fn get_position_by_range_start(&self) -> Self {
        Self::new_position(/*self.unit,*/ self.index, self.line, self.column)
    }

    // Get the ending position of a range as a `Location`.
    // pub fn get_position_by_range_end(&self) -> Self {
    //     let index = self.index + self.length;
    //     let column = self.column + self.length;
    //     Self::new_position(self.unit, index, self.line, column)
    // }

    /// Move the position forward by one character.
    pub fn move_position_forward(&self) -> Self {
        Self {
            index: self.index + 1,
            column: self.column + 1,
            ..*self
        }
    }

    /// Move the position backward by one character.
    pub fn move_position_backward(&self) -> Self {
        Self {
            index: self.index - 1,
            column: self.column - 1,
            ..*self
        }
    }
}
