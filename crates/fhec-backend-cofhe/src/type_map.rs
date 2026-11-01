use fhec_frontend::ast::Type;

use super::emitter::CoFheError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoType {
    Bool,
    Uint(u32),
}

impl CoType {
    pub(crate) fn from_ast(ty: &Type) -> Result<Self, CoFheError> {
        match ty {
            Type::Encrypted(inner) => match inner.as_ref() {
                Type::Bool => Ok(Self::Bool),
                Type::UInt(width) if [8, 16, 32, 64].contains(width) => Ok(Self::Uint(*width)),
                _ => Err(CoFheError(format!(
                    "Unsupported CoFHE encrypted type: {ty:?}"
                ))),
            },
            _ => Err(CoFheError(format!(
                "CoFHE requires encrypted parameters and values, found {ty:?}"
            ))),
        }
    }

    pub(crate) fn solidity(self) -> String {
        match self {
            Self::Bool => "ebool".to_owned(),
            Self::Uint(width) => format!("euint{width}"),
        }
    }

    pub(crate) fn literal(self, value: u64) -> Result<String, CoFheError> {
        let Self::Uint(width) = self else {
            return Err(CoFheError(
                "Numeric literal cannot be used as encrypted<bool>".into(),
            ));
        };
        if width < 64 && value >= (1u64 << width) {
            return Err(CoFheError(format!(
                "Integer {value} exceeds encrypted<u{width}> range"
            )));
        }
        Ok(format!("FHE.asEuint{width}({value})"))
    }
}
