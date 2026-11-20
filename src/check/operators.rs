//! F5 operator domains and results, shared by source inference.

use crate::frontend::parse::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Domain {
    Integer,
    Boolean,
    Equality,
}

pub(crate) fn binary(op: &str) -> Option<(Domain, Type)> {
    Some(match op {
        "+" | "-" | "*" | "/" | "%" => (Domain::Integer, Type::I64),
        "<" | "<=" | ">" | ">=" => (Domain::Integer, Type::Bool),
        "==" | "!=" => (Domain::Equality, Type::Bool),
        "and" | "or" => (Domain::Boolean, Type::Bool),
        _ => return None,
    })
}

pub(crate) fn is_arithmetic(op: &str, f5: bool) -> bool {
    if !f5 {
        return matches!(op, "+" | "-");
    }
    matches!(binary(op), Some((Domain::Integer, Type::I64)))
}

pub(crate) fn is_relational(op: &str, f5: bool) -> bool {
    if !f5 {
        return op == "<";
    }
    matches!(binary(op), Some((Domain::Integer, Type::Bool)))
}

pub(crate) fn is_comparison(op: &str, f5: bool) -> bool {
    is_relational(op, f5) || matches!(binary(op), Some((Domain::Equality, Type::Bool)))
}

pub(crate) fn needs_f5(op: &str) -> bool {
    binary(op).is_some() && !matches!(op, "+" | "-" | "<" | "==")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f5_operator_domain_result_matrix_matches_adr() {
        for (op, domain, result) in [
            ("+", Domain::Integer, Type::I64),
            ("-", Domain::Integer, Type::I64),
            ("*", Domain::Integer, Type::I64),
            ("/", Domain::Integer, Type::I64),
            ("%", Domain::Integer, Type::I64),
            ("<", Domain::Integer, Type::Bool),
            ("<=", Domain::Integer, Type::Bool),
            (">", Domain::Integer, Type::Bool),
            (">=", Domain::Integer, Type::Bool),
            ("==", Domain::Equality, Type::Bool),
            ("!=", Domain::Equality, Type::Bool),
            ("and", Domain::Boolean, Type::Bool),
            ("or", Domain::Boolean, Type::Bool),
        ] {
            assert_eq!(binary(op), Some((domain, result)), "{op}");
        }
        for op in ["..<", "..=", "=", "+=", "??"] {
            assert_eq!(binary(op), None, "{op}");
        }
    }
}
