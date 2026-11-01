use std::error::Error;
use std::fmt;

use fhec_ir::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoweringError {
    UnresolvedVariable(String),
    UnknownFunction(String),
    CallArgumentCount {
        function: String,
        expected: usize,
        actual: usize,
    },
    RecursiveCall(String),
    CallDepthExceeded(String),
    UnsupportedCallBody(String),
    IfWithoutElse,
    EmptyValueBlock {
        branch: &'static str,
    },
    BlockDoesNotProduceValue {
        branch: &'static str,
    },
    IfBranchTypeMismatch {
        then_type: Type,
        else_type: Type,
    },
}

impl fmt::Display for LoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnresolvedVariable(name) => {
                write!(formatter, "variable `{name}` was not resolved")
            }

            Self::UnknownFunction(name) => write!(formatter, "unknown function `{name}`"),
            Self::CallArgumentCount {
                function,
                expected,
                actual,
            } => write!(
                formatter,
                "function `{function}` expects {expected} arguments, found {actual}"
            ),
            Self::RecursiveCall(chain) => write!(formatter, "recursive call: {chain}"),
            Self::CallDepthExceeded(name) => {
                write!(formatter, "function inlining depth exceeded at `{name}`")
            }
            Self::UnsupportedCallBody(name) => {
                write!(formatter, "unsupported inlined function body: {name}")
            }
            Self::IfWithoutElse => {
                write!(
                    formatter,
                    "if expressions used as values require an else branch"
                )
            }

            Self::EmptyValueBlock { branch } => {
                write!(
                    formatter,
                    "{branch} branch is empty and does not produce a value"
                )
            }

            Self::BlockDoesNotProduceValue { branch } => {
                write!(formatter, "{branch} branch does not produce a value")
            }

            Self::IfBranchTypeMismatch {
                then_type,
                else_type,
            } => {
                write!(
                    formatter,
                    "if branches must produce the same type: then={then_type:?}, else={else_type:?}"
                )
            }
        }
    }
}

impl Error for LoweringError {}
