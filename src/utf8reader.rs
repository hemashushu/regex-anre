// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

/// Read the next char
/// Return `(codepoint:u32, byte_length:usize)`
/// Convert the code point manually to char if you need:
/// `char = unsafe { char::from_u32_unchecked(code) }`
///
/// Note that ensure the data is a valid utf-8 stream before
/// invoke this function
#[inline]
pub fn read_char(data: &[u8], position: usize) -> (u32, usize) {
    // 1 byte:  0_bbb_aaaa
    // 2 bytes: 110_ccc_bb, 10_bb_aaaa
    // 3 bytes: 1110_dddd, 10_cccc_bb, 10_bb_aaaa
    // 4 bytes: 11110_f_ee, 10_ee_dddd, 10_cccc_bb, 10_bb_aaaa
    // ref:
    // https://en.wikipedia.org/wiki/UTF-8

    let mut code: u32 = 0;

    let first_byte = data[position];
    let byte_length = match first_byte.leading_ones() {
        0 => {
            // 0_bbb_aaaa
            code |= first_byte as u32;
            1
        }
        2 => {
            // 110_ccc_bb, 10_bb_aaaa
            code |= ((first_byte & 0b1_1111) as u32) << 6;
            code |= (data[position + 1] & 0b11_1111) as u32;
            2
        }
        3 => {
            // 1110_dddd, 10_cccc_bb, 10_bb_aaaa
            code |= ((first_byte & 0b1111) as u32) << 12;
            code |= ((data[position + 1] & 0b11_1111) as u32) << 6;
            code |= (data[position + 2] & 0b11_1111) as u32;
            3
        }
        4 => {
            // 11110_f_ee, 10_ee_dddd, 10_cccc_bb, 10_bb_aaaa
            code |= ((first_byte & 0b111) as u32) << 18;
            code |= ((data[position + 1] & 0b11_1111) as u32) << 12;
            code |= ((data[position + 2] & 0b11_1111) as u32) << 6;
            code |= (data[position + 3] & 0b11_1111) as u32;
            4
        }
        1 => panic!("Invalid UTF-8 position."),
        _ => panic!("Invalid UTF-8 data."),
    };

    (code, byte_length)
}

#[allow(dead_code)]
pub fn read_previous_char(data: &[u8], mut position: usize) -> (u32, usize) {
    position -= 1;
    while data[position].leading_ones() == 1 {
        position -= 1;
    }
    read_char(data, position)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::utf8reader::{read_char, read_previous_char};

    #[test]
    fn test_next_char() {
        let data = "a文b😋c".bytes().collect::<Vec<u8>>();
        let data_ref = &data[..];

        assert_eq!(read_char(data_ref, 0), ('a' as u32, 1));
        assert_eq!(read_char(data_ref, 1), ('文' as u32, 3));
        assert_eq!(read_char(data_ref, 4), ('b' as u32, 1));
        assert_eq!(read_char(data_ref, 5), ('😋' as u32, 4));
        assert_eq!(read_char(data_ref, 9), ('c' as u32, 1));
    }

    #[test]
    fn test_previous_char() {
        let data = "a文b😋c".bytes().collect::<Vec<u8>>();
        let data_ref = &data[..];

        assert_eq!(read_previous_char(data_ref, 1), ('a' as u32, 1));
        assert_eq!(read_previous_char(data_ref, 4), ('文' as u32, 3));
        assert_eq!(read_previous_char(data_ref, 5), ('b' as u32, 1));
        assert_eq!(read_previous_char(data_ref, 9), ('😋' as u32, 4));
        assert_eq!(read_previous_char(data_ref, 10), ('c' as u32, 1));
    }
}
