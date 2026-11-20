//! Binding levels shared by the parser and canonical printer.
//! F5 §2.1 and D15 §3.2 order the enabled expression forms.

pub(super) const EXPRESSION: u8 = 0;
pub(super) const OPERAND: u8 = 1;

#[derive(Clone, Copy)]
pub(super) struct Levels {
    pub comparison: u8,
    pub range: u8,
    pub addition: u8,
    pub multiplication: u8,
    pub try_guard: u8,
    pub postfix: u8,
    logical_or: u8,
    logical_and: u8,
}

pub(super) const J: Levels = Levels {
    logical_or: 0,
    logical_and: 0,
    comparison: 1,
    range: 0,
    addition: 2,
    multiplication: 0,
    try_guard: 4,
    postfix: 6,
};

pub(super) const F5: Levels = Levels {
    logical_or: 1,
    logical_and: 2,
    comparison: 3,
    range: 4,
    addition: 5,
    multiplication: 6,
    try_guard: 8,
    postfix: 9,
};

impl Levels {
    pub(super) fn binary(self, op: &str) -> u8 {
        match op {
            "or" => self.logical_or,
            "and" => self.logical_and,
            "==" | "<" | "!=" | ">" | "<=" | ">=" => self.comparison,
            "..<" | "..=" => self.range,
            "+" | "-" => self.addition,
            "*" | "/" | "%" => self.multiplication,
            _ => EXPRESSION,
        }
    }
}
