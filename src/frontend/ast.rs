pub type CompUnit = Vec<GlobalDef>;

#[derive(Debug)]
pub enum GlobalDef {
  Func(FuncDef),
}

#[derive(Debug)]
pub struct FuncDef {
  pub func_type: FuncType,
  pub param_list: ParamList,
  pub ident: String,
  pub block: Block,
}


#[derive(Debug)]
pub enum FuncType {
  Void,
  Int,
}

pub type ParamList = Vec<Param>;

#[derive(Debug)]
pub enum Param {
  Ident(String),
}

pub type Block = Vec<BlockItem>;

#[derive(Debug)]
pub enum BlockItem {
  Decl(Decl),
  Stmt(Stmt),
}

#[derive(Debug)]
pub enum Stmt {
  Assign(LVal, Box<Exp>),
  Exp(Option<Box<Exp>>),
  Block(Box<Block>),
  If(Box<Exp>, Box<Stmt>, Option<Box<Stmt>>),
  While(Box<Exp>, Box<Stmt>),
  Break,
  Continue,
  Return(Box<Exp>),
}

pub type Exp = LOrExp;

#[derive(Debug)]
pub enum LOrExp {
  And(Box<LAndExp>),
  Or(Box<LOrExp>, Box<LAndExp>),
}

#[derive(Debug)]
pub enum LAndExp {
  Eq(Box<EqExp>),
  And(Box<LAndExp>, Box<EqExp>),
}

#[derive(Debug)]
pub enum EqExp {
  Rel(Box<RelExp>),
  Eq(Box<EqExp>, EqOp, Box<RelExp>),
}

#[derive(Debug)]
pub enum EqOp {
  Equal,
  NotEqual,
}

#[derive(Debug)]
pub enum RelExp {
  Add(Box<AddExp>),
  Rel(Box<RelExp>, RelOp, Box<AddExp>),
}

#[derive(Debug)]
pub enum RelOp {
  Less,
  LessEqual,
  Greater,
  GreaterEqual,
}

#[derive(Debug)]
pub enum AddExp {
  Mul(Box<MulExp>),
  Add(Box<AddExp>, AddOp, Box<MulExp>),
}

#[derive(Debug)]
pub enum AddOp {
  Plus,
  Minus,
}

#[derive(Debug)]
pub enum MulExp {
  Unary(Box<UnaryExp>),
  Mul(Box<MulExp>, MulOp, Box<UnaryExp>),
}

#[derive(Debug)]
pub enum MulOp {
  Multiply,
  Divide,
  Modulo,
}

#[derive(Debug)]
pub enum UnaryExp {
  Primary(PrimaryExp),
  Call(String, Vec<Box<Exp>>),
  Op(UnaryOp, Box<UnaryExp>),
}

#[derive(Debug)]
pub enum UnaryOp {
  Positive,
  Negative,
  Not,
}

#[derive(Debug)]
pub enum PrimaryExp {
  Num(i32),
  LVal(LVal),
  Paren(Box<Exp>),
}

#[derive(Debug)]
pub enum Decl {
  Const(ConstDecl),
  Var(VarDecl),
}

pub type ConstDecl = Vec<ConstDef>;
pub type VarDecl = Vec<VarDef>;

#[derive(Debug)]
pub struct ConstDef {
  pub ident: String,
  pub init_val: Box<Exp>,
}

#[derive(Debug)]
pub struct VarDef {
  pub ident: String,
  pub init_val: Option<InitVal>,
}

#[derive(Debug)]
pub enum InitVal {
  Simple(Box<Exp>),
  // Array(Vec<Box<Exp>>),
}

#[derive(Debug)]
pub enum LVal {
  Ident(String),
}
