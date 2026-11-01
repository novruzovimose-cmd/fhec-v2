#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        ty: Option<Type>,
        value: Expression,
    },

    Return(Option<Expression>),

    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(u64),

    Variable(String),
    Boolean(bool),

    Call {
        function: String,
        arguments: Vec<Expression>,
    },

    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },

    If {
        condition: Box<Expression>,
        then_branch: Block,
        else_branch: Option<Block>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,

    Equal,
    NotEqual,

    Greater,
    GreaterEqual,

    Less,
    LessEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Bool,

    UInt(u32),

    Int(u32),

    Encrypted(Box<Type>),

    Plaintext(Box<Type>),

    Unit,
}
