// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use crate::location::Location;

#[derive(Debug, PartialEq)]
pub struct CharWithPosition {
    pub character: char,
    pub position: Location,
}

impl CharWithPosition {
    /// Creates a new `CharWithPosition` instance with the given character and position.
    pub fn new(character: char, position: Location) -> Self {
        Self {
            character,
            position,
        }
    }
}

pub struct CharsWithPositionIter<'a> {
    upstream: &'a mut dyn Iterator<Item = char>,
    current_position: Location,
}

impl<'a> CharsWithPositionIter<'a> {
    /// Creates a new `CharsWithPositionIter` instance.
    ///
    /// # Arguments
    /// * `upstream` - A mutable reference to an iterator over characters.
    pub fn new(upstream: &'a mut dyn Iterator<Item = char>) -> Self {
        Self {
            upstream,
            current_position: Location::new_position(0, 0, 0),
        }
    }
}

impl Iterator for CharsWithPositionIter<'_> {
    type Item = CharWithPosition;

    /// Advances the iterator and returns the next `CharWithPosition`.
    /// Updates the position based on the character being processed.
    fn next(&mut self) -> Option<Self::Item> {
        match self.upstream.next() {
            Some(c) => {
                // Save the current position before updating it.
                let last_position = self.current_position;

                // Update the position index.
                self.current_position.index += 1;

                // Handle line breaks by updating line and column numbers.
                if c == '\n' {
                    self.current_position.line += 1;
                    self.current_position.column = 0;
                } else {
                    self.current_position.column += 1;
                }

                // Return the character along with its previous position.
                Some(CharWithPosition::new(c, last_position))
            }
            None => None, // Return None when the upstream iterator is exhausted.
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        charwithposition::{CharWithPosition, CharsWithPositionIter},
        location::Location,
    };

    #[test]
    fn test_chars_with_position_iter() {
        {
            // Test with a string containing multiple lines and characters.
            let mut chars = "a\nmn\nxyz".chars();
            let mut char_position_iter = CharsWithPositionIter::new(&mut chars);

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('a', Location::new_position(0, 0, 0)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('\n', Location::new_position(1, 0, 1)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('m', Location::new_position(2, 1, 0)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('n', Location::new_position(3, 1, 1)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('\n', Location::new_position(4, 1, 2)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('x', Location::new_position(5, 2, 0)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('y', Location::new_position(6, 2, 1)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('z', Location::new_position(7, 2, 2)))
            );

            assert!(char_position_iter.next().is_none());
        }

        {
            // Test with a string containing various newline sequences.
            let mut chars = "\n\r\n\n".chars();
            let mut char_position_iter = CharsWithPositionIter::new(&mut chars);

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('\n', Location::new_position(0, 0, 0)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('\r', Location::new_position(1, 1, 0)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('\n', Location::new_position(2, 1, 1)))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new('\n', Location::new_position(3, 2, 0)))
            );

            assert!(char_position_iter.next().is_none());
        }
    }
}
