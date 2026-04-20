use std::ops::Range;

pub type Span = Range<usize>;

#[derive(Clone, Debug)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Function(Function),
    Static(StaticItem),
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Clone, Debug)]
pub struct StaticItem {
    pub name: String,
    pub ty: TypeExpr,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let(LetStmt),
    IndexAssign(IndexAssignStmt),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct LetStmt {
    pub mutable: bool,
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct IndexAssignStmt {
    pub target: Expr,
    pub index: Expr,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Path(PathExpr),
    String(StringLiteral),
    Number(String),
    Char(String),
    Array(Vec<Expr>),
    Map(Vec<MapEntry>),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    MacroCall {
        path: PathExpr,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    Field {
        receiver: Box<Expr>,
        name: String,
    },
    Index {
        receiver: Box<Expr>,
        index: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
    Closure {
        param: String,
        body: Box<Expr>,
    },
    Paren(Box<Expr>),
    Return(Box<Expr>),
}

#[derive(Clone, Debug)]
pub struct MapEntry {
    pub key: Expr,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct PathExpr {
    pub segments: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StringLiteral {
    pub raw: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum UnaryOp {
    Ref,
    Not,
    Neg,
}

#[derive(Clone, Debug)]
pub enum BinaryOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    NotEq,
    And,
    Or,
}

#[derive(Clone, Debug)]
pub enum TypeExpr {
    Path(TypePath),
    Reference {
        lifetime: Option<String>,
        mutable: bool,
        inner: Box<TypeExpr>,
    },
}

#[derive(Clone, Debug)]
pub struct TypePath {
    pub segments: Vec<TypeSegment>,
}

#[derive(Clone, Debug)]
pub struct TypeSegment {
    pub name: String,
    pub generics: Vec<TypeExpr>,
}

#[derive(Clone, Debug)]
pub struct Analysis {
    pub module: Module,
}
