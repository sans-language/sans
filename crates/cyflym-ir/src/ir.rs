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
    BinOp { dest: Reg, op: IrBinOp, left: Reg, right: Reg },
    Copy { dest: Reg, src: Reg },
    Call { dest: Reg, function: String, args: Vec<Reg> },
    Ret { value: Reg },
}

#[derive(Debug, Clone, Copy)]
pub enum IrBinOp {
    Add,
    Sub,
    Mul,
    Div,
}
