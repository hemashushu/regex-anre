# Regex ANRE

[Regex-anre](https://github.com/hemashushu/regex-anre) is a lightweight and full-featured regular expression engine, it supports both standard regular expression and the new ANRE language. It also supports JIT and has excellent performance.

> Regex-anre provides the same API as [the Rust standard regular expression library](https://docs.rs/regex/), so you can directly replace it with Regex-anre without changing any code.

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Features](#features)
- [Quick Start](#quick-start)
- [The ANRE Language](#the-anre-language)

<!-- /code_chunk_output -->

## Features

- **Lightweight**: The Regex-anre is built from scratch without any dependencies, makeing it extremely lightweight.
- **Full-featured**: Regex-anre supports most regular expression features, including backreferences, look-ahead and look-behind assertions, which are not supported by the Rust standard regular expression library.
- **High-performance**: Regex-anre additionally provides a JIT edition [regex-jit](https://github.com/hemashushu/regex-anre) that can compile regular expressions into native machine code, offering extremely high performance.
- **New language**: In addition to supporting standard regular expressions, Regex-anre also provides a new regular expression language - ANRE. This is very intuitive, easy-to-read and write language that allows you to easily master the power of regular expressions. No more headaches over the traditional regular expression syntax.
- **Good Compatiblity**: ANRE can be translated one-to-one into traditional regular expressions and vice versa. They can even be mixed together, meaning you can smoothly migrate to the new language.
- **Good API design**: Regex-anre provides the same API as the Rust standard regular expression library, so you can directly replace it with Regex-anre without changing any code.

## Quick Start

Add the crate "regex_anre" to your project by command line:

```bash
cargo add regex_anre
```

or by manually adding it to your `Cargo.toml` file:

```toml
[dependencies]
regex_anre = "1.2.0"
```

Then, you can use it in your code:

```

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

> While this library is avaiable, the documentation is still being written. Or refer to [the documentation for the Rust standard regex library](https://docs.rs/regex/latest/regex/).

## The ANRE Language

_ANRE_ is a brand new regex language that offers all the capabilities of traditional regex but in much simpler form.

Designed with user-friendliness in mind, _ANRE_ requires no prior knowledge to get started and can be seamlessly converted to and from traditional regex.

(_ANRE_ is short for _XiaoXuan Regular Expression_)

> The documentation is still being written. You can refer to the unit test code in the source file `process.rs` if you want to see examples in ANRE language.
