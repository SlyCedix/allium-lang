#[derive(Debug)]
pub enum BinaryOperation {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
}

#[derive(Debug)]
pub enum Expr {
    Number(f64),
    Variable(String),
    Binary {
        op: BinaryOperation,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug)]
pub struct Prototype {
    name: String,
    args: Vec<String>,
}

#[derive(Debug)]
pub struct Function {
    prototype: Prototype,
    body: Expr,
}

pub fn get_tree() -> Expr {
    let lhs = Expr::Variable("x".into());
    let rhs = Expr::Variable("y".into());
    let expr = Expr::Binary {
        op: BinaryOperation::Add,
        lhs: lhs.into(),
        rhs: rhs.into(),
    };

    return expr;
}
