use std::{borrow::Cow, error::Error, fmt};

use crate::{Function, Type};

#[derive(Debug)]
pub enum EidosError {
    InvalidArgument {
        function: Function,
        position: usize,
        found_type: Type,
    },
    NotEnoughArguments {
        function: Function,
        expected: usize,
        stack_size: usize,
    },
}

impl fmt::Display for EidosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EidosError::InvalidArgument {
                function,
                position,
                found_type,
            } => write!(
                f,
                "Invalid argument {position} to {function}. Found {found_type}."
            ),
            EidosError::NotEnoughArguments {
                function,
                expected,
                stack_size,
            } => write!(
                f,
                "Not enough arguments to {function}. It expects {expected}, \
                but the stack {}.",
                match stack_size {
                    0 => "is empty".into(),
                    1 => "only has 1 value".into(),
                    n => format!("only has {n} values"),
                }
            ),
        }
    }
}

impl Error for EidosError {}

fn _plural(s: &str, n: usize) -> Cow<str> {
    if n == 1 {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(format!("{s}s"))
    }
}
