#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<IrFunction>,
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<String>, // param register names like "arg0", "arg1"
    pub body: Vec<Instruction>,
}

pub type Reg = String;

#[derive(Debug, Clone)]
pub enum Instruction {
    Const { dest: Reg, value: i64 },
    BoolConst { dest: Reg, value: bool },
    StringConst { dest: Reg, value: String },
    PrintInt { value: Reg },
    PrintString { value: Reg },
    PrintBool { value: Reg },
    BinOp { dest: Reg, op: IrBinOp, left: Reg, right: Reg },
    CmpOp { dest: Reg, op: IrCmpOp, left: Reg, right: Reg },
    Not { dest: Reg, src: Reg },
    Copy { dest: Reg, src: Reg },
    Call { dest: Reg, function: String, args: Vec<Reg> },
    Ret { value: Reg },
    // Control flow
    Label { name: String },
    Branch { cond: Reg, then_label: String, else_label: String },
    Jump { target: String },
    Phi { dest: Reg, a_val: Reg, a_label: String, b_val: Reg, b_label: String },
    // Memory operations for mutable variables
    Alloca { dest: Reg },
    Store { ptr: Reg, value: Reg },
    Load { dest: Reg, ptr: Reg },
}

#[derive(Debug, Clone, Copy)]
pub enum IrBinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy)]
pub enum IrCmpOp {
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}
