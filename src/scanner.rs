//! Byte-oriented J word formation, separate from primitive/noun interpretation.
//! States follow the behavioral model in pinned jsrc/w.c (wordil).
use crate::{Error, Result};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Space,
    AfterNumber,
    Symbol,
    Name,
    N,
    Nb,
    ClosedQuote,
    Number,
    MoreNumber,
    Quote,
    NbDot,
    Comment,
    Newline,
    OpenBrace,
    CloseBrace,
    DoubleBrace,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Other,
    Space,
    Letter,
    N,
    B,
    Number,
    Dot,
    Colon,
    Quote,
    Newline,
    OpenBrace,
    CloseBrace,
}
fn class(b: u8) -> Class {
    match b {
        b' ' | b'\t' => Class::Space,
        b'\n' => Class::Newline,
        b'N' => Class::N,
        b'B' => Class::B,
        b'A'..=b'Z' | b'a'..=b'z' => Class::Letter,
        b'0'..=b'9' | b'_' => Class::Number,
        b'.' => Class::Dot,
        b':' => Class::Colon,
        b'\'' => Class::Quote,
        b'{' => Class::OpenBrace,
        b'}' => Class::CloseBrace,
        _ => Class::Other,
    }
}
fn initial(c: Class) -> State {
    match c {
        Class::Space => State::Space,
        Class::Letter | Class::B => State::Name,
        Class::N => State::N,
        Class::Number => State::Number,
        Class::Quote => State::Quote,
        Class::Newline => State::Newline,
        Class::OpenBrace => State::OpenBrace,
        Class::CloseBrace => State::CloseBrace,
        _ => State::Symbol,
    }
}
/// The action is the number of alternating end/start boundaries to emit.
fn transition(s: State, c: Class) -> (State, usize, bool) {
    use Class as C;
    use State as S;
    if s == S::Quote {
        return (
            if c == C::Quote {
                S::ClosedQuote
            } else {
                S::Quote
            },
            0,
            false,
        );
    }
    if matches!(s, S::Space | S::AfterNumber) {
        if c == C::Space {
            return (s, 0, false);
        }
        return (
            if s == S::AfterNumber && c == C::Number {
                S::MoreNumber
            } else {
                initial(c)
            },
            1,
            false,
        );
    }
    if matches!(s, S::NbDot | S::Comment) {
        if c == C::Newline {
            return (S::Newline, 2, false);
        }
        if s == S::NbDot && matches!(c, C::Dot | C::Colon) {
            return (S::Symbol, 0, false);
        }
        return (S::Comment, 0, false);
    }
    if c == C::Space {
        return (
            if matches!(s, S::Number | S::MoreNumber) {
                S::AfterNumber
            } else {
                S::Space
            },
            1,
            false,
        );
    }
    if matches!(s, S::Number | S::MoreNumber) {
        if matches!(c, C::Letter | C::N | C::B | C::Number | C::Dot) {
            return (s, 0, false);
        }
        if c == C::Colon {
            return (S::Symbol, 0, false);
        }
    }
    if matches!(s, S::Name | S::N | S::Nb) {
        if matches!(c, C::Letter | C::N | C::B | C::Number) {
            return (
                if s == S::N && c == C::B {
                    S::Nb
                } else {
                    S::Name
                },
                0,
                false,
            );
        }
        if matches!(c, C::Dot | C::Colon) {
            return (
                if s == S::Nb && c == C::Dot {
                    S::NbDot
                } else {
                    S::Symbol
                },
                0,
                false,
            );
        }
    }
    if s == S::ClosedQuote && c == C::Quote {
        return (S::Quote, 0, false);
    }
    if (s == S::OpenBrace && c == C::CloseBrace) || (s == S::CloseBrace && c == C::OpenBrace) {
        return (S::Symbol, 2, false);
    }
    if (s == S::OpenBrace && c == C::OpenBrace) || (s == S::CloseBrace && c == C::CloseBrace) {
        return (S::DoubleBrace, 0, false);
    }
    if matches!(s, S::Symbol | S::OpenBrace | S::CloseBrace | S::DoubleBrace)
        && matches!(c, C::Dot | C::Colon)
    {
        return (S::Symbol, 0, s == S::DoubleBrace);
    }
    (initial(c), 2, false)
}

/// Spans include comments and LF. They are byte offsets, including for UTF-8.
/// Unsupported primitives remain whole words; this does not evaluate syntax.
pub fn scan(source: &[u8]) -> Result<Vec<Range<usize>>> {
    let mut boundaries = Vec::new();
    let mut state = State::Space;
    for (i, &b) in source.iter().enumerate() {
        let c = class(b);
        let (next, emit, split_brace) = transition(state, c);
        if state == State::MoreNumber
            && matches!(
                c,
                Class::Other
                    | Class::Space
                    | Class::Quote
                    | Class::OpenBrace
                    | Class::CloseBrace
                    | Class::Newline
            )
        {
            boundaries.truncate(boundaries.len() - 2);
        }
        if split_brace {
            boundaries.extend([i - 1, i - 1]);
        }
        boundaries.extend(std::iter::repeat_n(i, emit));
        state = next;
    }
    if state == State::Quote {
        return Err(Error::Syntax("unterminated literal".into()));
    }
    if state == State::MoreNumber {
        boundaries.truncate(boundaries.len() - 2);
    }
    boundaries.push(source.len());
    Ok(boundaries.chunks_exact(2).map(|p| p[0]..p[1]).collect())
}
