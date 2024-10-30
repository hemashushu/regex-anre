# Regex ANRE

_ANRE_ is a brand new regex language that offers all the capabilities of traditional regex but in much simpler form.

Designed with user-friendliness in mind, _ANRE_ requires no prior knowledge to get started and can be seamlessly converted to and from traditional regex.

(_ANRE_ is short for _XiaoXuan Regular Expression_)

[regex-anre](https://github.com/hemashushu/regex-anre) is a lightweight (zero dependency) but comprehensive regular expression engine, it supports both standard regular expression and the ANRE language. It also supports JIT and has excellent performance.

`regex-anre` provides the same API as [the Rust standard regex](https://docs.rs/regex/), you can directly use `regex-anre` to replace the standard regex without changing the existing code.

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Features](#features)
- [Quick Start](#quick-start)

<!-- /code_chunk_output -->

## Features

- **Lightweight** The library [regex-anre](https://github.com/hemashushu/regex-anre) has no dependencies.
- **Comprehensive** Supports most regex features, including back-reference, look-ahead and look-behind assertions.
- **High performance** The JIT edition [regex-jit](https://github.com/hemashushu/regex-anre) provides excellent performance.

## Quick Start

```rust
use regex_anre::Regex;

let re = Regex::new("...").unwrap();

// or, create a regex using ANRE language:
// let re = Regex::from_anre("...").unwrap();

let mut matches = re.find_iter("...");
for m in matches {
    println("{}", m.as_str());
}
```

See https://docs.rs/regex/latest/regex/ for details.

TODO