#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
}

pub mod surface {
    use super::{BinaryOp, UnaryOp};

    #[derive(Debug, Clone, PartialEq)]
    pub struct Program {
        pub items: Vec<Item>,
        pub expr: Option<Expr>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Item {
        Fn(FnDef),
        Let(LetDef),
        ExternFn(ExternFnDef),
        Data(DataDef),
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FnDef {
        pub name: String,
        pub params: Vec<String>,
        pub body: Expr,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LetDef {
        pub name: String,
        pub expr: Expr,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ExternFnDef {
        pub name: String,
        pub params: Vec<String>,
        pub explain: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DataDef {
        pub name: String,
        pub variants: Vec<DataVariant>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DataVariant {
        pub name: String,
        pub fields: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Expr {
        Int(i64),
        Bool(bool),
        Str(String),
        Bytes(Vec<u8>),
        List(Vec<Expr>),
        Map(Vec<(Expr, Expr)>),
        Var(String),
        Construct {
            name: String,
            fields: Vec<(String, Expr)>,
        },
        Unary {
            op: UnaryOp,
            expr: Box<Expr>,
        },
        Binary {
            op: BinaryOp,
            left: Box<Expr>,
            right: Box<Expr>,
        },
        If {
            cond: Box<Expr>,
            then_branch: Box<Expr>,
            else_branch: Box<Expr>,
        },
        Call {
            callee: String,
            args: Vec<Expr>,
        },
        Pipe {
            input: Box<Expr>,
            target: PipeTarget,
        },
        Match {
            scrutinee: Box<Expr>,
            arms: Vec<MatchArm>,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MatchArm {
        pub pattern: MatchPattern,
        pub body: Expr,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum MatchPattern {
        Wildcard,
        Expr(Expr),
        Compare {
            op: BinaryOp,
            expr: Expr,
        },
        Variant {
            name: String,
            fields: Vec<FieldPattern>,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FieldPattern {
        pub field: String,
        pub bind: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum PipeTarget {
        Ident(String),
        Call { name: String, args: Vec<Expr> },
    }
}

pub mod core {
    use super::{BinaryOp, UnaryOp};

    #[derive(Debug, Clone, PartialEq)]
    pub struct Program {
        pub items: Vec<Item>,
        pub expr: Option<Expr>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Item {
        Fn(FnDef),
        Let(LetDef),
        ExternFn(ExternFnDef),
        Data(DataDef),
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FnDef {
        pub name: String,
        pub params: Vec<String>,
        pub body: Expr,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LetDef {
        pub name: String,
        pub expr: Expr,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ExternFnDef {
        pub name: String,
        pub params: Vec<String>,
        pub explain: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DataDef {
        pub name: String,
        pub variants: Vec<DataVariant>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DataVariant {
        pub name: String,
        pub fields: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Expr {
        Int(i64),
        Bool(bool),
        Str(String),
        Bytes(Vec<u8>),
        List(Vec<Expr>),
        Map(Vec<(Expr, Expr)>),
        Var(String),
        Construct {
            name: String,
            fields: Vec<(String, Expr)>,
        },
        Unary {
            op: UnaryOp,
            expr: Box<Expr>,
        },
        Binary {
            op: BinaryOp,
            left: Box<Expr>,
            right: Box<Expr>,
        },
        If {
            cond: Box<Expr>,
            then_branch: Box<Expr>,
            else_branch: Box<Expr>,
        },
        Call {
            callee: String,
            args: Vec<Expr>,
        },
        Match {
            scrutinee: Box<Expr>,
            arms: Vec<MatchArm>,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MatchArm {
        pub pattern: MatchPattern,
        pub body: Expr,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum MatchPattern {
        Wildcard,
        Expr(Expr),
        Compare {
            op: BinaryOp,
            expr: Expr,
        },
        Variant {
            name: String,
            fields: Vec<FieldPattern>,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FieldPattern {
        pub field: String,
        pub bind: Option<String>,
    }
}
