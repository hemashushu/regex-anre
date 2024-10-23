// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// syntax summary
//
// meta characters and their meanings:
//
// - [ ]      charset
// - [^ ]     negative charset
// - {m}      repeat
// - {m,n}    repeat range
// - {m,}     at least
// - (xyz)    group
// - *        0 or more
// - +        1 or more
// - ?        optional or lazy repetition
// - |        logic or
// - ^        beginning assertion
// - $        end assertion
// - .        any character except new line (\r and \n)
// - \        to form escape character
//
// meta characters `( ) { } [ ] \ + * ? . | ^ $` need to be escaped when
// used as literal characters, e.g. `\(`, `\*` and `\.`
// whereas, only `[`, `]` and `\` need to be escaped in a charset, and also
// if the hyphen `-` is not in the first or last position in the charset, e.g.:
// `[ab-]`, `[a\-b]`
//
// escape characters:
//
// - \t       horizontal tab
// - \n       new line
// - \r       carriage return
// - \f       form feed
// - \v       vertical tab
// - \u{hhhh} unicode
//
// preset charsets:
//
// - \w       alphanumeric characters: [a-zA-Z0-9_]
// - \W       [^\w]
// - \d       digits: [0-9]
// - \D       [^\d]
// - \s       whitespaces [ \t\r\n\v\f]
// - \S       [^\s]
//
// boundary assertions:
// - \b       word boundary
// - \B       not word boundary
//
// non-capturing groups:
// - (?:...)
//
// named capture group
// - (?<name>...)
//
// back references:
// - \number  by number, e.g. `\1`, `\2` and `\3`
// - \k<name> by name
//
// lookaround assertions:
//
// - (?=...)  lookahead
// - (?!...)  negative lookahead
// - (?<=...) lookbehind
// - (?<!...) negative lookbehind



