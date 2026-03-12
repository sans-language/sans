pub mod ir;

use std::collections::HashMap;

use cyflym_parser::ast::{BinOp, Expr, Program, Stmt};
use ir::{Instruction, IrBinOp, IrFunction, Module, Reg};

/// Lower a parsed `Program` into an IR `Module`.
pub fn lower(program: &Program) -> Module {
    let functions = program.functions.iter().map(lower_function).collect();
    Module { functions }
}

fn lower_function(func: &cyflym_parser::ast::Function) -> IrFunction {
    let mut builder = IrBuilder::new();

    // Map params to arg registers
    let params: Vec<Reg> = func
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| {
            let reg = format!("arg{}", i);
            builder.locals.insert(param.name.clone(), reg.clone());
            reg
        })
        .collect();

    let mut last_reg: Option<Reg> = None;
    for stmt in &func.body {
        match stmt {
            Stmt::Let { name, value, .. } => {
                let reg = builder.lower_expr(value);
                builder.locals.insert(name.clone(), reg);
            }
            Stmt::Expr(expr) => {
                last_reg = Some(builder.lower_expr(expr));
            }
        }
    }

    // Emit Ret with the last expression's register
    if let Some(ret_reg) = last_reg {
        builder.instructions.push(Instruction::Ret { value: ret_reg });
    }

    IrFunction {
        name: func.name.clone(),
        params,
        body: builder.instructions,
    }
}

struct IrBuilder {
    counter: usize,
    locals: HashMap<String, Reg>,
    instructions: Vec<Instruction>,
}

impl IrBuilder {
    fn new() -> Self {
        IrBuilder {
            counter: 0,
            locals: HashMap::new(),
            instructions: Vec::new(),
        }
    }

    fn fresh_reg(&mut self) -> Reg {
        let reg = format!("%{}", self.counter);
        self.counter += 1;
        reg
    }

    fn lower_expr(&mut self, expr: &Expr) -> Reg {
        match expr {
            Expr::IntLiteral { value, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::Const { dest: dest.clone(), value: *value });
                dest
            }
            Expr::Identifier { name, .. } => {
                self.locals
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| panic!("undefined variable: {}", name))
            }
            Expr::BinaryOp { left, op, right, .. } => {
                let left_reg = self.lower_expr(left);
                let right_reg = self.lower_expr(right);
                let dest = self.fresh_reg();
                let ir_op = match op {
                    BinOp::Add => IrBinOp::Add,
                    BinOp::Sub => IrBinOp::Sub,
                    BinOp::Mul => IrBinOp::Mul,
                    BinOp::Div => IrBinOp::Div,
                    // TODO(Task 5): handle comparison and boolean operators
                    _ => todo!("IR lowering for {:?} not yet implemented", op),
                };
                self.instructions.push(Instruction::BinOp {
                    dest: dest.clone(),
                    op: ir_op,
                    left: left_reg,
                    right: right_reg,
                });
                dest
            }
            // TODO(Task 5): handle bool literals, if/else, and unary ops in IR lowering
            Expr::BoolLiteral { .. } => todo!("IR lowering for BoolLiteral not yet implemented"),
            Expr::If { .. } => todo!("IR lowering for If not yet implemented"),
            Expr::UnaryOp { .. } => todo!("IR lowering for UnaryOp not yet implemented"),

            Expr::Call { function, args, .. } => {
                let arg_regs: Vec<Reg> = args.iter().map(|a| self.lower_expr(a)).collect();
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::Call {
                    dest: dest.clone(),
                    function: function.clone(),
                    args: arg_regs,
                });
                dest
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::Instruction;

    fn parse(src: &str) -> Program {
        cyflym_parser::parse(src).expect("parse failed")
    }

    #[test]
    fn lower_minimal() {
        let program = parse("fn main() Int { 42 }");
        let module = lower(&program);

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.name, "main");

        // Should have Const(42) and Ret
        let has_const = func.body.iter().any(|i| matches!(i, Instruction::Const { value: 42, .. }));
        let has_ret = func.body.iter().any(|i| matches!(i, Instruction::Ret { .. }));
        assert!(has_const, "expected Const(42) instruction");
        assert!(has_ret, "expected Ret instruction");
    }

    #[test]
    fn lower_let_and_arithmetic() {
        let program = parse("fn main() Int { let x Int = 1 + 2 x }");
        let module = lower(&program);

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Should have a BinOp instruction
        let has_binop = func.body.iter().any(|i| matches!(i, Instruction::BinOp { .. }));
        assert!(has_binop, "expected BinOp instruction");

        // Should also have Ret
        let has_ret = func.body.iter().any(|i| matches!(i, Instruction::Ret { .. }));
        assert!(has_ret, "expected Ret instruction");
    }

    #[test]
    fn lower_function_call() {
        let program = parse(
            "fn add(a Int, b Int) Int { a + b } fn main() Int { add(1, 2) }",
        );
        let module = lower(&program);

        assert_eq!(module.functions.len(), 2, "expected 2 functions");

        // Find main function
        let main_func = module.functions.iter().find(|f| f.name == "main").expect("no main");

        // main should have a Call instruction
        let has_call = main_func.body.iter().any(|i| {
            matches!(i, Instruction::Call { function, .. } if function == "add")
        });
        assert!(has_call, "expected Call(add) instruction in main");
    }
}
