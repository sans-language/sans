pub mod ir;

use std::collections::HashMap;

use cyflym_parser::ast::{BinOp, Expr, Program, Stmt};
use ir::{Instruction, IrBinOp, IrCmpOp, IrFunction, Module, Reg};

#[derive(Clone)]
enum LocalVar {
    /// Immutable variable — direct SSA register
    Value(Reg),
    /// Mutable variable — alloca pointer register (needs Load to read, Store to write)
    Ptr(Reg),
}

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
            builder.locals.insert(param.name.clone(), LocalVar::Value(reg.clone()));
            reg
        })
        .collect();

    for (i, stmt) in func.body.iter().enumerate() {
        let is_last = i == func.body.len() - 1;

        if is_last {
            match stmt {
                Stmt::Expr(expr) => {
                    let reg = builder.lower_expr(expr);
                    builder.instructions.push(Instruction::Ret { value: reg });
                }
                Stmt::Return { value, .. } => {
                    let reg = builder.lower_expr(value);
                    builder.instructions.push(Instruction::Ret { value: reg });
                }
                other => {
                    builder.lower_stmt(other);
                }
            }
        } else {
            builder.lower_stmt(stmt);
        }
    }

    IrFunction {
        name: func.name.clone(),
        params,
        body: builder.instructions,
    }
}

struct IrBuilder {
    counter: usize,
    label_counter: usize,
    locals: HashMap<String, LocalVar>,
    instructions: Vec<Instruction>,
}

impl IrBuilder {
    fn new() -> Self {
        IrBuilder {
            counter: 0,
            label_counter: 0,
            locals: HashMap::new(),
            instructions: Vec::new(),
        }
    }

    fn fresh_reg(&mut self) -> Reg {
        let reg = format!("%{}", self.counter);
        self.counter += 1;
        reg
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    fn lower_expr(&mut self, expr: &Expr) -> Reg {
        match expr {
            Expr::IntLiteral { value, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::Const { dest: dest.clone(), value: *value });
                dest
            }
            Expr::BoolLiteral { value, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::BoolConst { dest: dest.clone(), value: *value });
                dest
            }
            Expr::Identifier { name, .. } => {
                match self.locals.get(name).unwrap_or_else(|| panic!("undefined variable: {}", name)).clone() {
                    LocalVar::Value(reg) => reg,
                    LocalVar::Ptr(ptr) => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Load { dest: dest.clone(), ptr });
                        dest
                    }
                }
            }
            Expr::BinaryOp { left, op, right, .. } => {
                // Short-circuit operators: must handle BEFORE evaluating both sides
                match op {
                    BinOp::And => {
                        let left_reg = self.lower_expr(left);
                        let rhs_label = self.fresh_label("and_rhs");
                        let false_label = self.fresh_label("and_false");
                        let merge_label = self.fresh_label("and_merge");

                        self.instructions.push(Instruction::Branch {
                            cond: left_reg,
                            then_label: rhs_label.clone(),
                            else_label: false_label.clone(),
                        });

                        self.instructions.push(Instruction::Label { name: rhs_label.clone() });
                        let right_reg = self.lower_expr(right);
                        self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                        self.instructions.push(Instruction::Label { name: false_label.clone() });
                        let false_reg = self.fresh_reg();
                        self.instructions.push(Instruction::BoolConst { dest: false_reg.clone(), value: false });
                        self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                        self.instructions.push(Instruction::Label { name: merge_label.clone() });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Phi {
                            dest: dest.clone(),
                            a_val: right_reg,
                            a_label: rhs_label,
                            b_val: false_reg,
                            b_label: false_label,
                        });
                        return dest;
                    }
                    BinOp::Or => {
                        let left_reg = self.lower_expr(left);
                        let true_label = self.fresh_label("or_true");
                        let rhs_label = self.fresh_label("or_rhs");
                        let merge_label = self.fresh_label("or_merge");

                        self.instructions.push(Instruction::Branch {
                            cond: left_reg,
                            then_label: true_label.clone(),
                            else_label: rhs_label.clone(),
                        });

                        self.instructions.push(Instruction::Label { name: true_label.clone() });
                        let true_reg = self.fresh_reg();
                        self.instructions.push(Instruction::BoolConst { dest: true_reg.clone(), value: true });
                        self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                        self.instructions.push(Instruction::Label { name: rhs_label.clone() });
                        let right_reg = self.lower_expr(right);
                        self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                        self.instructions.push(Instruction::Label { name: merge_label.clone() });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Phi {
                            dest: dest.clone(),
                            a_val: true_reg,
                            a_label: true_label,
                            b_val: right_reg,
                            b_label: rhs_label,
                        });
                        return dest;
                    }
                    _ => {} // fall through to normal handling
                }

                // Non-short-circuit: evaluate both sides
                let left_reg = self.lower_expr(left);
                let right_reg = self.lower_expr(right);

                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        let dest = self.fresh_reg();
                        let ir_op = match op {
                            BinOp::Add => IrBinOp::Add,
                            BinOp::Sub => IrBinOp::Sub,
                            BinOp::Mul => IrBinOp::Mul,
                            BinOp::Div => IrBinOp::Div,
                            _ => unreachable!(),
                        };
                        self.instructions.push(Instruction::BinOp {
                            dest: dest.clone(),
                            op: ir_op,
                            left: left_reg,
                            right: right_reg,
                        });
                        dest
                    }
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                        let dest = self.fresh_reg();
                        let cmp_op = match op {
                            BinOp::Eq => IrCmpOp::Eq,
                            BinOp::NotEq => IrCmpOp::NotEq,
                            BinOp::Lt => IrCmpOp::Lt,
                            BinOp::Gt => IrCmpOp::Gt,
                            BinOp::LtEq => IrCmpOp::LtEq,
                            BinOp::GtEq => IrCmpOp::GtEq,
                            _ => unreachable!(),
                        };
                        self.instructions.push(Instruction::CmpOp {
                            dest: dest.clone(),
                            op: cmp_op,
                            left: left_reg,
                            right: right_reg,
                        });
                        dest
                    }
                    BinOp::And | BinOp::Or => unreachable!("handled above"),
                }
            }
            Expr::UnaryOp { op, operand, .. } => {
                match op {
                    cyflym_parser::ast::UnaryOp::Not => {
                        let src_reg = self.lower_expr(operand);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Not { dest: dest.clone(), src: src_reg });
                        dest
                    }
                }
            }
            Expr::If { condition, then_body, then_expr, else_body, else_expr, .. } => {
                let cond_reg = self.lower_expr(condition);
                let then_label = self.fresh_label("then");
                let else_label = self.fresh_label("else");
                let merge_label = self.fresh_label("merge");

                self.instructions.push(Instruction::Branch {
                    cond: cond_reg,
                    then_label: then_label.clone(),
                    else_label: else_label.clone(),
                });

                // Then branch
                self.instructions.push(Instruction::Label { name: then_label.clone() });
                for stmt in then_body {
                    self.lower_stmt(stmt);
                }
                let then_reg = self.lower_expr(then_expr);
                self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                // Else branch
                self.instructions.push(Instruction::Label { name: else_label.clone() });
                for stmt in else_body {
                    self.lower_stmt(stmt);
                }
                let else_reg = self.lower_expr(else_expr);
                self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                // Merge
                self.instructions.push(Instruction::Label { name: merge_label.clone() });
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::Phi {
                    dest: dest.clone(),
                    a_val: then_reg,
                    a_label: then_label,
                    b_val: else_reg,
                    b_label: else_label,
                });
                dest
            }
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

    fn lower_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, mutable, value, .. } => {
                let val_reg = self.lower_expr(value);
                if *mutable {
                    let ptr = self.fresh_reg();
                    self.instructions.push(Instruction::Alloca { dest: ptr.clone() });
                    self.instructions.push(Instruction::Store { ptr: ptr.clone(), value: val_reg });
                    self.locals.insert(name.clone(), LocalVar::Ptr(ptr));
                } else {
                    self.locals.insert(name.clone(), LocalVar::Value(val_reg));
                }
            }
            Stmt::Assign { name, value, .. } => {
                let val_reg = self.lower_expr(value);
                if let LocalVar::Ptr(ptr) = self.locals.get(name).unwrap().clone() {
                    self.instructions.push(Instruction::Store { ptr, value: val_reg });
                } else {
                    panic!("cannot assign to immutable variable: {}", name);
                }
            }
            Stmt::While { condition, body, .. } => {
                let cond_label = self.fresh_label("while_cond");
                let body_label = self.fresh_label("while_body");
                let end_label = self.fresh_label("while_end");

                self.instructions.push(Instruction::Jump { target: cond_label.clone() });

                self.instructions.push(Instruction::Label { name: cond_label.clone() });
                let cond_reg = self.lower_expr(condition);
                self.instructions.push(Instruction::Branch {
                    cond: cond_reg,
                    then_label: body_label.clone(),
                    else_label: end_label.clone(),
                });

                self.instructions.push(Instruction::Label { name: body_label.clone() });
                for s in body {
                    self.lower_stmt(s);
                }
                self.instructions.push(Instruction::Jump { target: cond_label.clone() });

                self.instructions.push(Instruction::Label { name: end_label.clone() });
            }
            Stmt::Return { value, .. } => {
                let reg = self.lower_expr(value);
                self.instructions.push(Instruction::Ret { value: reg });
            }
            Stmt::Expr(expr) => {
                self.lower_expr(expr);
            }
            Stmt::If { condition, body, .. } => {
                let cond_reg = self.lower_expr(condition);
                let then_label = self.fresh_label("if_then");
                let end_label = self.fresh_label("if_end");

                self.instructions.push(Instruction::Branch {
                    cond: cond_reg,
                    then_label: then_label.clone(),
                    else_label: end_label.clone(),
                });

                self.instructions.push(Instruction::Label { name: then_label });
                for s in body {
                    self.lower_stmt(s);
                }
                self.instructions.push(Instruction::Jump { target: end_label.clone() });

                self.instructions.push(Instruction::Label { name: end_label });
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

    #[test]
    fn lower_bool_literal() {
        let program = parse("fn main() Bool { true }");
        let module = lower(&program);
        let func = &module.functions[0];
        let has_bool_const = func.body.iter().any(|i| matches!(i, Instruction::BoolConst { value: true, .. }));
        assert!(has_bool_const, "expected BoolConst(true)");
    }

    #[test]
    fn lower_comparison() {
        let program = parse("fn main() Bool { 1 == 2 }");
        let module = lower(&program);
        let func = &module.functions[0];
        let has_cmp = func.body.iter().any(|i| matches!(i, Instruction::CmpOp { .. }));
        assert!(has_cmp, "expected CmpOp instruction");
    }

    #[test]
    fn lower_if_else() {
        let program = parse("fn main() Int { if true { 1 } else { 2 } }");
        let module = lower(&program);
        let func = &module.functions[0];
        let has_branch = func.body.iter().any(|i| matches!(i, Instruction::Branch { .. }));
        let has_phi = func.body.iter().any(|i| matches!(i, Instruction::Phi { .. }));
        assert!(has_branch, "expected Branch instruction");
        assert!(has_phi, "expected Phi instruction");
    }

    #[test]
    fn lower_not() {
        let program = parse("fn main() Bool { !true }");
        let module = lower(&program);
        let func = &module.functions[0];
        let has_not = func.body.iter().any(|i| matches!(i, Instruction::Not { .. }));
        assert!(has_not, "expected Not instruction");
    }

    #[test]
    fn lower_while_loop() {
        let program = parse("fn main() Int { let mut x Int = 0 while x < 10 { x = x + 1 } x }");
        let module = lower(&program);
        let func = &module.functions[0];

        let has_alloca = func.body.iter().any(|i| matches!(i, Instruction::Alloca { .. }));
        let has_store = func.body.iter().any(|i| matches!(i, Instruction::Store { .. }));
        let has_load = func.body.iter().any(|i| matches!(i, Instruction::Load { .. }));
        assert!(has_alloca, "expected Alloca for mutable variable");
        assert!(has_store, "expected Store for assignment");
        assert!(has_load, "expected Load for variable read");

        let label_count = func.body.iter().filter(|i| matches!(i, Instruction::Label { .. })).count();
        assert!(label_count >= 3, "expected at least 3 labels for while loop (cond, body, end)");
    }

    #[test]
    fn lower_return() {
        let program = parse("fn main() Int { return 42 }");
        let module = lower(&program);
        let func = &module.functions[0];
        let has_ret = func.body.iter().any(|i| matches!(i, Instruction::Ret { .. }));
        assert!(has_ret, "expected Ret instruction");
    }

    #[test]
    fn lower_mutable_variable() {
        let program = parse("fn main() Int { let mut x Int = 1 x = 2 x }");
        let module = lower(&program);
        let func = &module.functions[0];
        let has_alloca = func.body.iter().any(|i| matches!(i, Instruction::Alloca { .. }));
        let store_count = func.body.iter().filter(|i| matches!(i, Instruction::Store { .. })).count();
        assert!(has_alloca, "expected Alloca");
        assert!(store_count >= 2, "expected at least 2 Store instructions (init + reassign)");
    }
}
