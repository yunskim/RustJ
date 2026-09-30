//! Byte-oriented J word formation.
//!
//! This module intentionally follows jsource's `w.c::jtwordil` state machine.
//! Rust names/data structures differ, but character classes, transitions and
//! emitted word boundaries are a frontend compatibility surface.
use crate::{Error, Result};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
enum State {
    Space = 0,        // SS
    AfterNumber = 1,  // SS9
    Symbol = 2,       // SX
    Name = 3,         // SA
    N = 4,            // SN
    Nb = 5,           // SNB
    ClosedQuote = 6,  // SQQ
    Number = 7,       // S9
    MoreNumber = 8,   // S99
    Quote = 9,        // SQ
    NbDot = 10,       // SNZ
    Comment = 11,     // SZ
    Uninflectable = 12, // SU
    OpenBrace = 13,   // SDD
    CloseBrace = 14,  // SDDZ
    DoubleBrace = 15, // SDDD
}

/// Column order is chosen for readable Rust tables, but every variant maps
/// one-to-one to the jsource character classes used by `ctype` in `t.c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
enum Class {
    Other = 0,         // CX
    OpenBrace = 1,     // CDD
    CloseBrace = 2,    // CDDZ
    Uninflectable = 3, // CU (LF in the current ctype table)
    Space = 4,         // CS
    Letter = 5,        // CA
    N = 6,             // CN
    B = 7,             // CB
    Number = 8,        // C9
    Dot = 9,           // CD
    Colon = 10,        // CC
    Quote = 11,        // CQ
}

fn class(b: u8) -> Class {
    match b {
        b' ' | b'\t' => Class::Space,
        b'\n' => Class::Uninflectable,
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

#[derive(Clone, Copy)]
struct Transition {
    next: State,
    /// jsource E0/EI/EN/EZ encoded as the number of alternating
    /// end/start boundaries emitted at the current byte offset.
    emit: u8,
    /// jsource UNDD: `{{.` / `}}.`-style inflection means the doubled
    /// brace was not one word; restore the boundary between the braces.
    undouble_brace: bool,
}

const fn t(next: State, emit: u8) -> Transition {
    Transition {
        next,
        emit,
        undouble_brace: false,
    }
}
const fn undd(next: State) -> Transition {
    Transition {
        next,
        emit: 0,
        undouble_brace: true,
    }
}

use Class as C;
use State as S;

/// Direct semantic port of jsource `w.c::state[SDDD+1][16]`.
///
/// Columns:
/// CX, CDD, CDDZ, CU, CS, CA, CN, CB, C9, CD, CC, CQ.
///
/// The C table uses sparse numeric character-class indices; Rust uses this
/// dense column order so every meaningful transition is visible and auditable.
const TRANSITIONS: [[Transition; 12]; 16] = [
    // SS
    [t(S::Symbol,1), t(S::OpenBrace,1), t(S::CloseBrace,1), t(S::Uninflectable,1), t(S::Space,0), t(S::Name,1), t(S::N,1), t(S::Name,1), t(S::Number,1), t(S::Symbol,1), t(S::Symbol,1), t(S::Quote,1)],
    // SS9
    [t(S::Symbol,1), t(S::OpenBrace,1), t(S::CloseBrace,1), t(S::Uninflectable,1), t(S::AfterNumber,0), t(S::Name,1), t(S::N,1), t(S::Name,1), t(S::MoreNumber,1), t(S::Symbol,1), t(S::Symbol,1), t(S::Quote,1)],
    // SX
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,2), t(S::N,2), t(S::Name,2), t(S::Number,2), t(S::Symbol,0), t(S::Symbol,0), t(S::Quote,2)],
    // SA
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,0), t(S::Name,0), t(S::Name,0), t(S::Name,0), t(S::Symbol,0), t(S::Symbol,0), t(S::Quote,2)],
    // SN
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,0), t(S::Name,0), t(S::Nb,0), t(S::Name,0), t(S::Symbol,0), t(S::Symbol,0), t(S::Quote,2)],
    // SNB
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,0), t(S::Name,0), t(S::Name,0), t(S::Name,0), t(S::NbDot,0), t(S::Symbol,0), t(S::Quote,2)],
    // SQQ
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,2), t(S::N,2), t(S::Name,2), t(S::Number,2), t(S::Symbol,2), t(S::Symbol,2), t(S::Quote,0)],
    // S9
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::AfterNumber,1), t(S::Number,0), t(S::Number,0), t(S::Number,0), t(S::Number,0), t(S::Number,0), t(S::Symbol,0), t(S::Quote,2)],
    // S99
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::AfterNumber,1), t(S::MoreNumber,0), t(S::MoreNumber,0), t(S::MoreNumber,0), t(S::MoreNumber,0), t(S::MoreNumber,0), t(S::Symbol,0), t(S::Quote,2)],
    // SQ
    [t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::Quote,0), t(S::ClosedQuote,0)],
    // SNZ
    [t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Uninflectable,2), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Symbol,0), t(S::Symbol,0), t(S::Comment,0)],
    // SZ
    [t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Uninflectable,2), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0), t(S::Comment,0)],
    // SU
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,2), t(S::N,2), t(S::Name,2), t(S::Number,2), t(S::Symbol,2), t(S::Symbol,2), t(S::Quote,2)],
    // SDD
    [t(S::Symbol,2), t(S::DoubleBrace,0), t(S::Symbol,2), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,2), t(S::N,2), t(S::Name,2), t(S::Number,2), t(S::Symbol,0), t(S::Symbol,0), t(S::Quote,2)],
    // SDDZ
    [t(S::Symbol,2), t(S::Symbol,2), t(S::DoubleBrace,0), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,2), t(S::N,2), t(S::Name,2), t(S::Number,2), t(S::Symbol,0), t(S::Symbol,0), t(S::Quote,2)],
    // SDDD
    [t(S::Symbol,2), t(S::OpenBrace,2), t(S::CloseBrace,2), t(S::Uninflectable,2), t(S::Space,1), t(S::Name,2), t(S::N,2), t(S::Name,2), t(S::Number,2), undd(S::Symbol), undd(S::Symbol), t(S::Quote,2)],
];

fn transition(state: State, class: Class) -> Transition {
    TRANSITIONS[state as usize][class as usize]
}

/// Spans include comments and LF. They are byte offsets, including for UTF-8.
/// Unsupported primitives remain whole words; this does not evaluate syntax.
pub fn scan(source: &[u8]) -> Result<Vec<Range<usize>>> {
    scan_impl(source, false)
}

/// Only for detecting unsupported definition boundaries before stopping input.
/// An unfinished quoted word remains a single word; this is NOT syntax validation.
pub(crate) fn scan_unfinished(source: &[u8]) -> Vec<Range<usize>> {
    scan_impl(source, true).expect("unfinished quotes are permitted for input guards")
}

fn scan_impl(source: &[u8], unfinished: bool) -> Result<Vec<Range<usize>>> {
    let mut boundaries = Vec::new();
    let mut state = State::Space;

    for (i, &byte) in source.iter().enumerate() {
        let class = class(byte);
        let tr = transition(state, class);

        // jtwordil rewinds one start/end pair when leaving S99 through
        // CX/CS/CQ/CDD/CDDZ/CU.  Keep the same observable boundary rule.
        if state == State::MoreNumber
            && matches!(
                class,
                Class::Other
                    | Class::Space
                    | Class::Quote
                    | Class::OpenBrace
                    | Class::CloseBrace
                    | Class::Uninflectable
            )
        {
            boundaries.truncate(boundaries.len().saturating_sub(2));
        }

        if tr.undouble_brace {
            // Mirrors UNDD: the doubled brace was tentatively one word, but an
            // inflection proves the two braces must be separate words.
            boundaries.extend([i - 1, i - 1]);
        }

        boundaries.extend(std::iter::repeat_n(i, tr.emit as usize));
        state = tr.next;
    }

    if state == State::Quote && !unfinished {
        return Err(Error::OpenQuote);
    }

    // jtwordil performs the same S99 rewind before forcing the final EI.
    if state == State::MoreNumber {
        boundaries.truncate(boundaries.len().saturating_sub(2));
    }

    boundaries.push(source.len());
    Ok(boundaries.chunks_exact(2).map(|p| p[0]..p[1]).collect())
}
