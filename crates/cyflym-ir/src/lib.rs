pub mod ir;

use std::collections::HashMap;

use cyflym_parser::ast::{BinOp, Expr, Program, Stmt};
use ir::{Instruction, IrBinOp, IrCmpOp, IrFunction, Module, Reg};

#[derive(Clone, PartialEq)]
enum IrType { Int, Bool, Str, Struct(String), Enum(String), Sender, Receiver, JoinHandle, Mutex }

#[derive(Clone)]
enum LocalVar {
    /// Immutable variable — direct SSA register
    Value(Reg),
    /// Mutable variable — alloca pointer register (needs Load to read, Store to write)
    Ptr(Reg),
}

/// Lower a parsed `Program` into an IR `Module`.
pub fn lower(program: &Program) -> Module {
    // Collect struct definitions: name -> field names (ordered)
    let mut struct_defs: HashMap<String, Vec<String>> = HashMap::new();
    for s in &program.structs {
        let field_names: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
        struct_defs.insert(s.name.clone(), field_names);
    }
    // Collect enum definitions: name -> [(variant_name, tag_index, num_data_fields)]
    let mut enum_defs: HashMap<String, Vec<(String, usize, usize)>> = HashMap::new();
    for e in &program.enums {
        let variants: Vec<(String, usize, usize)> = e.variants.iter().enumerate()
            .map(|(i, v)| (v.name.clone(), i, v.fields.len()))
            .collect();
        enum_defs.insert(e.name.clone(), variants);
    }
    let mut functions: Vec<IrFunction> = program.functions.iter()
        .map(|f| lower_function(f, &struct_defs, &enum_defs))
        .collect();

    // Lower impl methods as mangled functions
    for imp in &program.impls {
        for method in &imp.methods {
            let mut mangled_method = method.clone();
            mangled_method.name = format!("{}_{}", imp.target_type, method.name);
            functions.push(lower_function(&mangled_method, &struct_defs, &enum_defs));
        }
    }

    Module { functions }
}

fn lower_function(func: &cyflym_parser::ast::Function, struct_defs: &HashMap<String, Vec<String>>, enum_defs: &HashMap<String, Vec<(String, usize, usize)>>) -> IrFunction {
    let mut builder = IrBuilder::new(struct_defs.clone(), enum_defs.clone());

    // Map params to arg registers
    let params: Vec<Reg> = func
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| {
            let reg = format!("arg{}", i);
            builder.locals.insert(param.name.clone(), LocalVar::Value(reg.clone()));
            // Set type for struct/enum params
            if struct_defs.contains_key(&param.type_name.name) {
                builder.reg_types.insert(reg.clone(), IrType::Struct(param.type_name.name.clone()));
            } else if enum_defs.contains_key(&param.type_name.name) {
                builder.reg_types.insert(reg.clone(), IrType::Enum(param.type_name.name.clone()));
            }
            reg
        })
        .collect();

    let param_struct_sizes: Vec<usize> = func.params.iter()
        .map(|p| {
            if let Some(fields) = struct_defs.get(&p.type_name.name) {
                fields.len()
            } else if let Some(variants) = enum_defs.get(&p.type_name.name) {
                let max_data = variants.iter().map(|(_, _, n)| *n).max().unwrap_or(0);
                1 + max_data // tag + max data fields
            } else {
                0
            }
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
        param_struct_sizes,
        body: builder.instructions,
    }
}

struct IrBuilder {
    counter: usize,
    label_counter: usize,
    locals: HashMap<String, LocalVar>,
    instructions: Vec<Instruction>,
    reg_types: HashMap<Reg, IrType>,
    struct_defs: HashMap<String, Vec<String>>,
    enum_defs: HashMap<String, Vec<(String, usize, usize)>>,
}

impl IrBuilder {
    fn new(struct_defs: HashMap<String, Vec<String>>, enum_defs: HashMap<String, Vec<(String, usize, usize)>>) -> Self {
        IrBuilder {
            counter: 0,
            label_counter: 0,
            locals: HashMap::new(),
            instructions: Vec::new(),
            reg_types: HashMap::new(),
            struct_defs,
            enum_defs,
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
                self.reg_types.insert(dest.clone(), IrType::Int);
                dest
            }
            Expr::BoolLiteral { value, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::BoolConst { dest: dest.clone(), value: *value });
                self.reg_types.insert(dest.clone(), IrType::Bool);
                dest
            }
            Expr::StringLiteral { value, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::StringConst { dest: dest.clone(), value: value.clone() });
                self.reg_types.insert(dest.clone(), IrType::Str);
                dest
            }
            Expr::Identifier { name, .. } => {
                match self.locals.get(name).unwrap_or_else(|| panic!("undefined variable: {}", name)).clone() {
                    LocalVar::Value(reg) => reg,
                    LocalVar::Ptr(ptr) => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Load { dest: dest.clone(), ptr: ptr.clone() });
                        if let Some(ty) = self.reg_types.get(&ptr).cloned() {
                            self.reg_types.insert(dest.clone(), ty);
                        }
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
                        self.reg_types.insert(false_reg.clone(), IrType::Bool);
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
                        self.reg_types.insert(dest.clone(), IrType::Bool);
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
                        self.reg_types.insert(true_reg.clone(), IrType::Bool);
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
                        self.reg_types.insert(dest.clone(), IrType::Bool);
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
                        self.reg_types.insert(dest.clone(), IrType::Int);
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
                        self.reg_types.insert(dest.clone(), IrType::Bool);
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
                        self.reg_types.insert(dest.clone(), IrType::Bool);
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
                let phi_type = self.reg_types.get(&then_reg).cloned().unwrap_or(IrType::Int);
                self.instructions.push(Instruction::Phi {
                    dest: dest.clone(),
                    a_val: then_reg,
                    a_label: then_label,
                    b_val: else_reg,
                    b_label: else_label,
                });
                self.reg_types.insert(dest.clone(), phi_type);
                dest
            }
            Expr::Call { function, args, .. } => {
                if function == "print" {
                    let arg_reg = self.lower_expr(&args[0]);
                    let ty = self.reg_types.get(&arg_reg).cloned().unwrap_or(IrType::Int);
                    match ty {
                        IrType::Str => self.instructions.push(Instruction::PrintString { value: arg_reg }),
                        IrType::Bool => self.instructions.push(Instruction::PrintBool { value: arg_reg }),
                        IrType::Int => self.instructions.push(Instruction::PrintInt { value: arg_reg }),
                        IrType::Struct(_) => panic!("cannot print struct"),
                        IrType::Enum(_) => panic!("cannot print enum"),
                        IrType::Sender | IrType::Receiver | IrType::JoinHandle | IrType::Mutex => panic!("cannot print concurrency type"),
                    }
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                }

                let arg_regs: Vec<Reg> = args.iter().map(|a| self.lower_expr(a)).collect();
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::Call {
                    dest: dest.clone(),
                    function: function.clone(),
                    args: arg_regs,
                });
                self.reg_types.insert(dest.clone(), IrType::Int);
                dest
            }
            Expr::StructLiteral { name, fields, .. } => {
                let struct_fields = self.struct_defs.get(name).cloned().unwrap_or_default();
                let num_fields = struct_fields.len();
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::StructAlloc { dest: dest.clone(), num_fields });
                self.reg_types.insert(dest.clone(), IrType::Struct(name.clone()));

                for (field_name, field_expr) in fields {
                    let val_reg = self.lower_expr(field_expr);
                    let field_index = struct_fields.iter().position(|n| n == field_name)
                        .expect("unknown field in struct literal");
                    self.instructions.push(Instruction::FieldStore {
                        ptr: dest.clone(),
                        field_index,
                        value: val_reg,
                    });
                }
                dest
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj_reg = self.lower_expr(object);
                let struct_name = match self.reg_types.get(&obj_reg) {
                    Some(IrType::Struct(name)) => name.clone(),
                    _ => panic!("field access on non-struct register"),
                };
                let struct_fields = self.struct_defs.get(&struct_name)
                    .expect("unknown struct in field access");
                let field_index = struct_fields.iter().position(|n| n == field)
                    .expect("unknown field in field access");
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::FieldLoad {
                    dest: dest.clone(),
                    ptr: obj_reg,
                    field_index,
                });
                // For now, all struct fields are Int
                self.reg_types.insert(dest.clone(), IrType::Int);
                dest
            }
            Expr::EnumVariant { enum_name, variant_name, args, .. } => {
                let variants = self.enum_defs.get(enum_name)
                    .expect("unknown enum in variant construction").clone();
                let (_, tag, num_data_fields) = variants.iter()
                    .find(|(n, _, _)| n == variant_name)
                    .expect("unknown variant");
                let tag = *tag as i64;
                let num_data_fields = *num_data_fields;

                let dest = self.fresh_reg();
                self.instructions.push(Instruction::EnumAlloc {
                    dest: dest.clone(),
                    tag,
                    num_data_fields,
                });
                self.reg_types.insert(dest.clone(), IrType::Enum(enum_name.clone()));

                for (i, arg) in args.iter().enumerate() {
                    let val_reg = self.lower_expr(arg);
                    self.instructions.push(Instruction::FieldStore {
                        ptr: dest.clone(),
                        field_index: i + 1, // +1 because tag is at index 0
                        value: val_reg,
                    });
                }
                dest
            }
            Expr::MethodCall { object, method, args, .. } => {
                let obj_reg = self.lower_expr(object);

                // Handle concurrency built-in methods FIRST
                match (self.reg_types.get(&obj_reg).cloned(), method.as_str()) {
                    (Some(IrType::Sender), "send") => {
                        let val_reg = self.lower_expr(&args[0]);
                        self.instructions.push(Instruction::ChannelSend {
                            tx: obj_reg,
                            value: val_reg,
                        });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Receiver), "recv") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ChannelRecv {
                            dest: dest.clone(),
                            rx: obj_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::JoinHandle), "join") => {
                        self.instructions.push(Instruction::ThreadJoin {
                            handle: obj_reg,
                        });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Mutex), "lock") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MutexLock {
                            dest: dest.clone(),
                            mutex: obj_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Mutex), "unlock") => {
                        let val_reg = self.lower_expr(&args[0]);
                        self.instructions.push(Instruction::MutexUnlock {
                            mutex: obj_reg,
                            value: val_reg,
                        });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    _ => {} // fall through to struct/enum handling
                }

                // Existing struct/enum method call handling
                let type_name = match self.reg_types.get(&obj_reg) {
                    Some(IrType::Struct(name)) => name.clone(),
                    Some(IrType::Enum(name)) => name.clone(),
                    _ => panic!("method call on non-struct/enum"),
                };
                let mangled = format!("{}_{}", type_name, method);
                let mut arg_regs = vec![obj_reg];
                for arg in args {
                    arg_regs.push(self.lower_expr(arg));
                }
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::Call {
                    dest: dest.clone(),
                    function: mangled,
                    args: arg_regs,
                });
                self.reg_types.insert(dest.clone(), IrType::Int);
                dest
            }
            Expr::Match { scrutinee, arms, .. } => {
                let scrutinee_reg = self.lower_expr(scrutinee);
                let enum_name = match self.reg_types.get(&scrutinee_reg) {
                    Some(IrType::Enum(name)) => name.clone(),
                    _ => panic!("match on non-enum register"),
                };
                let variants = self.enum_defs.get(&enum_name)
                    .expect("unknown enum in match").clone();

                // Get the tag
                let tag_reg = self.fresh_reg();
                self.instructions.push(Instruction::EnumTag {
                    dest: tag_reg.clone(),
                    ptr: scrutinee_reg.clone(),
                });
                self.reg_types.insert(tag_reg.clone(), IrType::Int);

                // Alloca for the result
                let result_ptr = self.fresh_reg();
                self.instructions.push(Instruction::Alloca { dest: result_ptr.clone() });

                let merge_label = self.fresh_label("match_merge");

                for (arm_index, arm) in arms.iter().enumerate() {
                    let cyflym_parser::ast::Pattern::EnumVariant {
                        variant_name,
                        bindings,
                        ..
                    } = &arm.pattern;

                    let (_, tag, _num_data) = variants.iter()
                        .find(|(n, _, _)| n == variant_name)
                        .expect("unknown variant in match arm");
                    let tag_val = *tag as i64;

                    let arm_label = self.fresh_label(&format!("match_arm{}", arm_index));
                    let next_label = if arm_index < arms.len() - 1 {
                        self.fresh_label(&format!("match_check{}", arm_index + 1))
                    } else {
                        // Last arm: fall through to arm (always taken)
                        arm_label.clone()
                    };

                    if arm_index < arms.len() - 1 {
                        // Compare tag
                        let tag_const = self.fresh_reg();
                        self.instructions.push(Instruction::Const {
                            dest: tag_const.clone(),
                            value: tag_val,
                        });
                        self.reg_types.insert(tag_const.clone(), IrType::Int);

                        let cmp_reg = self.fresh_reg();
                        self.instructions.push(Instruction::CmpOp {
                            dest: cmp_reg.clone(),
                            op: IrCmpOp::Eq,
                            left: tag_reg.clone(),
                            right: tag_const,
                        });
                        self.reg_types.insert(cmp_reg.clone(), IrType::Bool);

                        self.instructions.push(Instruction::Branch {
                            cond: cmp_reg,
                            then_label: arm_label.clone(),
                            else_label: next_label.clone(),
                        });
                    } else {
                        // Last arm: just jump to it
                        self.instructions.push(Instruction::Jump {
                            target: arm_label.clone(),
                        });
                    }

                    self.instructions.push(Instruction::Label { name: arm_label });

                    // Bind data fields
                    for (i, binding_name) in bindings.iter().enumerate() {
                        let data_reg = self.fresh_reg();
                        self.instructions.push(Instruction::EnumData {
                            dest: data_reg.clone(),
                            ptr: scrutinee_reg.clone(),
                            field_index: i,
                        });
                        self.reg_types.insert(data_reg.clone(), IrType::Int);
                        self.locals.insert(binding_name.clone(), LocalVar::Value(data_reg));
                    }

                    let body_reg = self.lower_expr(&arm.body);
                    self.instructions.push(Instruction::Store {
                        ptr: result_ptr.clone(),
                        value: body_reg,
                    });
                    self.instructions.push(Instruction::Jump {
                        target: merge_label.clone(),
                    });

                    // If not the last arm, emit the next check label
                    if arm_index < arms.len() - 1 {
                        self.instructions.push(Instruction::Label { name: next_label });
                    }
                }

                self.instructions.push(Instruction::Label { name: merge_label });
                let result_reg = self.fresh_reg();
                self.instructions.push(Instruction::Load {
                    dest: result_reg.clone(),
                    ptr: result_ptr,
                });
                self.reg_types.insert(result_reg.clone(), IrType::Int);
                result_reg
            }
            Expr::Spawn { function, args, .. } => {
                let arg_regs: Vec<Reg> = args.iter().map(|a| self.lower_expr(a)).collect();
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::ThreadSpawn {
                    dest: dest.clone(),
                    function: function.clone(),
                    args: arg_regs,
                });
                self.reg_types.insert(dest.clone(), IrType::JoinHandle);
                dest
            }
            Expr::ChannelCreate { .. } => {
                // Should only appear inside LetDestructure
                panic!("ChannelCreate should only appear inside LetDestructure")
            }
            Expr::MutexCreate { value, .. } => {
                let val_reg = self.lower_expr(value);
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::MutexCreate {
                    dest: dest.clone(),
                    value: val_reg,
                });
                self.reg_types.insert(dest.clone(), IrType::Mutex);
                dest
            }
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, mutable, value, .. } => {
                let val_reg = self.lower_expr(value);
                if *mutable {
                    let val_type = self.reg_types.get(&val_reg).cloned().unwrap_or(IrType::Int);
                    let ptr = self.fresh_reg();
                    self.instructions.push(Instruction::Alloca { dest: ptr.clone() });
                    self.instructions.push(Instruction::Store { ptr: ptr.clone(), value: val_reg });
                    self.locals.insert(name.clone(), LocalVar::Ptr(ptr.clone()));
                    self.reg_types.insert(ptr, val_type);
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
            Stmt::LetDestructure { names, value, .. } => {
                match value {
                    Expr::ChannelCreate { capacity, .. } => {
                        let tx_reg = self.fresh_reg();
                        let rx_reg = self.fresh_reg();
                        if let Some(cap_expr) = capacity {
                            let cap_reg = self.lower_expr(cap_expr);
                            self.instructions.push(Instruction::ChannelCreateBounded {
                                tx_dest: tx_reg.clone(),
                                rx_dest: rx_reg.clone(),
                                capacity: cap_reg,
                            });
                        } else {
                            self.instructions.push(Instruction::ChannelCreate {
                                tx_dest: tx_reg.clone(),
                                rx_dest: rx_reg.clone(),
                            });
                        }
                        self.reg_types.insert(tx_reg.clone(), IrType::Sender);
                        self.reg_types.insert(rx_reg.clone(), IrType::Receiver);
                        self.locals.insert(names[0].clone(), LocalVar::Value(tx_reg));
                        self.locals.insert(names[1].clone(), LocalVar::Value(rx_reg));
                    }
                    _ => panic!("LetDestructure only supports ChannelCreate"),
                }
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
    fn lower_print_string() {
        let program = parse(r#"fn main() Int { print("hello") }"#);
        let module = lower(&program);
        let func = &module.functions[0];
        let has_print = func.body.iter().any(|i| matches!(i, Instruction::PrintString { .. }));
        let has_str = func.body.iter().any(|i| matches!(i, Instruction::StringConst { .. }));
        assert!(has_print, "expected PrintString");
        assert!(has_str, "expected StringConst");
    }

    #[test]
    fn lower_print_int() {
        let program = parse("fn main() Int { print(42) }");
        let module = lower(&program);
        let func = &module.functions[0];
        let has_print = func.body.iter().any(|i| matches!(i, Instruction::PrintInt { .. }));
        assert!(has_print, "expected PrintInt");
    }

    #[test]
    fn lower_struct_literal() {
        let program = parse("struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 1, y: 2 } 0 }");
        let module = lower(&program);
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_alloc = func.body.iter().any(|i| matches!(i, Instruction::StructAlloc { num_fields: 2, .. }));
        let store_count = func.body.iter().filter(|i| matches!(i, Instruction::FieldStore { .. })).count();
        assert!(has_alloc, "expected StructAlloc with 2 fields");
        assert_eq!(store_count, 2, "expected 2 FieldStore instructions");
    }

    #[test]
    fn lower_field_access() {
        let program = parse("struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 1, y: 2 } p.x }");
        let module = lower(&program);
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_load = func.body.iter().any(|i| matches!(i, Instruction::FieldLoad { field_index: 0, .. }));
        assert!(has_load, "expected FieldLoad with field_index 0");
    }

    #[test]
    fn lower_enum_variant() {
        let program = parse("enum Color { Red, Green, Blue, } fn main() Int { let c = Color::Green 0 }");
        let module = lower(&program);
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_alloc = func.body.iter().any(|i| matches!(i, Instruction::EnumAlloc { tag: 1, num_data_fields: 0, .. }));
        assert!(has_alloc, "expected EnumAlloc with tag=1, num_data_fields=0");
    }

    #[test]
    fn lower_match_expr() {
        let program = parse(
            "enum Color { Red, Green, Blue, } fn main() Int { let c = Color::Green match c { Color::Red => 1, Color::Green => 2, Color::Blue => 3, } }"
        );
        let module = lower(&program);
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_tag = func.body.iter().any(|i| matches!(i, Instruction::EnumTag { .. }));
        let has_branch = func.body.iter().any(|i| matches!(i, Instruction::Branch { .. }));
        let label_count = func.body.iter().filter(|i| matches!(i, Instruction::Label { .. })).count();
        assert!(has_tag, "expected EnumTag instruction");
        assert!(has_branch, "expected Branch instruction");
        assert!(label_count >= 3, "expected at least 3 labels, got {}", label_count);
    }

    #[test]
    fn lower_method_call() {
        let program = parse("struct Point { x Int, y Int, } impl Point { fn sum(self) Int { self.x + self.y } } fn main() Int { let p = Point { x: 3, y: 4 } p.sum() }");
        let module = lower(&program);
        // Should have 2 functions: main and Point_sum
        assert_eq!(module.functions.len(), 2);
        let point_sum = module.functions.iter().find(|f| f.name == "Point_sum").expect("no Point_sum");
        assert_eq!(point_sum.params.len(), 1); // self
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_call = main_func.body.iter().any(|i| matches!(i, Instruction::Call { function, .. } if function == "Point_sum"));
        assert!(has_call, "expected Call(Point_sum)");
    }

    #[test]
    fn lower_method_with_args() {
        let program = parse("struct Point { x Int, y Int, } impl Point { fn add(self, n Int) Int { self.x + self.y + n } } fn main() Int { let p = Point { x: 1, y: 2 } p.add(10) }");
        let module = lower(&program);
        let point_add = module.functions.iter().find(|f| f.name == "Point_add").expect("no Point_add");
        assert_eq!(point_add.params.len(), 2); // self + n
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

    #[test]
    fn lower_generic_function() {
        let program = parse("fn identity<T>(x T) T { x } fn main() Int { identity(42) }");
        let module = lower(&program);
        assert_eq!(module.functions.len(), 2);
        let identity = module.functions.iter().find(|f| f.name == "identity").expect("no identity");
        assert_eq!(identity.params.len(), 1);
    }

    #[test]
    fn lower_spawn() {
        let program = parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() 0 }");
        let module = lower(&program);
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_spawn = main_func.body.iter().any(|i| matches!(i, Instruction::ThreadSpawn { .. }));
        assert!(has_spawn, "expected ThreadSpawn instruction");
    }

    #[test]
    fn lower_channel_create() {
        let program = parse("fn main() Int { let (tx, rx) = channel<Int>() 0 }");
        let module = lower(&program);
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_create = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelCreate { .. }));
        assert!(has_create, "expected ChannelCreate instruction");
    }

    #[test]
    fn lower_send_recv() {
        let program = parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }");
        let module = lower(&program);
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_send = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelSend { .. }));
        let has_recv = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelRecv { .. }));
        assert!(has_send, "expected ChannelSend instruction");
        assert!(has_recv, "expected ChannelRecv instruction");
    }

    #[test]
    fn lower_join() {
        let program = parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() h.join() }");
        let module = lower(&program);
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_join = main_func.body.iter().any(|i| matches!(i, Instruction::ThreadJoin { .. }));
        assert!(has_join, "expected ThreadJoin instruction");
    }

    #[test]
    fn lower_mutex_create_lock_unlock() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let m = mutex(5) let v = m.lock() m.unlock(v) 0 }"
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::MutexCreate { .. })),
            "expected MutexCreate instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::MutexLock { .. })),
            "expected MutexLock instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::MutexUnlock { .. })),
            "expected MutexUnlock instruction");
    }

    #[test]
    fn lower_bounded_channel() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let (tx, rx) = channel<Int>(10) tx.send(1) rx.recv() }"
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ChannelCreateBounded { .. })),
            "expected ChannelCreateBounded instruction");
    }

    #[test]
    fn lower_unbounded_channel_unchanged() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let (tx, rx) = channel<Int>() tx.send(1) rx.recv() }"
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ChannelCreate { .. })),
            "expected ChannelCreate (not bounded) instruction");
        assert!(!instrs.iter().any(|i| matches!(i, Instruction::ChannelCreateBounded { .. })),
            "should NOT have ChannelCreateBounded");
    }
}
