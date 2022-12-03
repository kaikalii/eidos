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

impl EidosError {
    pub fn invalid_argument(function: &Function, position: usize, found_type: Type) -> Self {
        EidosError::InvalidArgument {
            function: function.clone(),
            position,
            found_type,
        }
    }
    pub fn not_enough_arguments(function: &Function, expected: usize, stack_size: usize) -> Self {
        EidosError::NotEnoughArguments {
            function: function.clone(),
            expected,
            stack_size,
        }
    }
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
                but the stack only has {stack_size} {}.",
                plural("value", *stack_size)
            ),
        }
    }
}

impl Error for EidosError {}

fn plural(s: &str, n: usize) -> Cow<str> {
    if n == 1 {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(format!("{s}s"))
    }
}
