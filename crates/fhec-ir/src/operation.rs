use crate::{Constant, Type, ValueId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Copy {
        result: ValueId,
        operand: ValueId,
    },

    Argument {
        result: ValueId,
        ty: Type,
    },

    Constant {
        result: ValueId,
        value: Constant,
    },

    Add {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    Sub {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    Mul {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    Eq {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    NotEqual {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    GreaterThan {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    GreaterEqual {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    LessThan {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    LessEqual {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    Not {
        result: ValueId,
        operand: ValueId,
    },

    And {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    Or {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    Xor {
        result: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },

    Mux {
        result: ValueId,
        condition: ValueId,
        if_true: ValueId,
        if_false: ValueId,
    },

    Return {
        value: Option<ValueId>,
    },
}
