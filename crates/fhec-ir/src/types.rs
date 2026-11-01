use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Unit,

    Bool,

    UInt { width: u32 },

    Int { width: u32 },

    Encrypted(Box<Type>),

    Plaintext(Box<Type>),
}

impl Type {
    pub const fn uint(width: u32) -> Self {
        Self::UInt { width }
    }

    pub const fn int(width: u32) -> Self {
        Self::Int { width }
    }

    pub fn encrypted(inner: Type) -> Self {
        Self::Encrypted(Box::new(inner))
    }

    pub fn plaintext(inner: Type) -> Self {
        Self::Plaintext(Box::new(inner))
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(_))
    }

    pub fn encrypted_inner(&self) -> Option<&Type> {
        match self {
            Self::Encrypted(inner) => Some(inner),
            _ => None,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Bool => write!(f, "bool"),
            Self::UInt { width } => write!(f, "u{width}"),
            Self::Int { width } => write!(f, "i{width}"),
            Self::Encrypted(inner) => write!(f, "encrypted<{inner}>"),
            Self::Plaintext(inner) => write!(f, "plaintext<{inner}>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_type_is_detected() {
        let ty = Type::encrypted(Type::uint(8));

        assert!(ty.is_encrypted());
        assert_eq!(ty.to_string(), "encrypted<u8>");
    }

    #[test]
    fn integer_types_format_correctly() {
        assert_eq!(Type::uint(8).to_string(), "u8");
        assert_eq!(Type::int(32).to_string(), "i32");
    }
}
