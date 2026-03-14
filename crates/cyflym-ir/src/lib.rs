pub mod ir;

use std::collections::HashMap;

use cyflym_parser::ast::{BinOp, Expr, Program, Stmt};
use ir::{Instruction, IrBinOp, IrCmpOp, IrFunction, Module, Reg};

#[derive(Clone, PartialEq, Debug)]
pub enum IrType { Int, Float, Bool, Str, Struct(String), Enum(String), Sender, Receiver, JoinHandle, Mutex, Array(Box<IrType>), JsonValue, HttpResponse, Result(Box<IrType>) }

pub fn ir_type_for_return(ty: &cyflym_typeck::types::Type) -> IrType {
    use cyflym_typeck::types::Type;
    match ty {
        Type::Int => IrType::Int,
        Type::Float => IrType::Float,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Str,
        Type::Struct { name, .. } => IrType::Struct(name.clone()),
        Type::Enum { name, .. } => IrType::Enum(name.clone()),
        Type::Array { inner } => IrType::Array(Box::new(ir_type_for_return(inner))),
        Type::JsonValue => IrType::JsonValue,
        Type::HttpResponse => IrType::HttpResponse,
        Type::Result { inner } => IrType::Result(Box::new(ir_type_for_return(inner))),
        Type::ResultErr => IrType::Result(Box::new(IrType::Int)), // default inner type for err
        _ => IrType::Int, // Fallback
    }
}

#[derive(Clone)]
enum LocalVar {
    /// Immutable variable — direct SSA register
    Value(Reg),
    /// Mutable variable — alloca pointer register (needs Load to read, Store to write)
    Ptr(Reg),
}

/// Lower a parsed `Program` into an IR `Module`.
pub fn lower(program: &Program, module_name: Option<&str>, module_fn_ret_types: &HashMap<(String, String), IrType>) -> Module {
    lower_with_extra_structs(program, module_name, module_fn_ret_types, &HashMap::new())
}

/// Like `lower`, but merges `extra_struct_defs` (from imported modules) into the struct
/// definitions available during lowering. This allows the main module to perform field
/// accesses on structs that are defined in imported modules.
pub fn lower_with_extra_structs(
    program: &Program,
    module_name: Option<&str>,
    module_fn_ret_types: &HashMap<(String, String), IrType>,
    extra_struct_defs: &HashMap<String, Vec<String>>,
) -> Module {
    // Collect struct definitions: name -> field names (ordered)
    let mut struct_defs: HashMap<String, Vec<String>> = extra_struct_defs.clone();
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

    let module_names: Vec<String> = program.imports.iter()
        .map(|imp| imp.module_name.clone())
        .collect();

    // Build local function return type map for Result/opaque type tracking
    let mut local_fn_ret_types: HashMap<String, IrType> = HashMap::new();
    for f in &program.functions {
        let ret_name = &f.return_type.name;
        let ir_type = if ret_name.starts_with("Result<") && ret_name.ends_with('>') {
            let inner_str = &ret_name[7..ret_name.len()-1];
            let inner = match inner_str {
                "Int" => IrType::Int,
                "Float" => IrType::Float,
                "Bool" => IrType::Bool,
                "String" => IrType::Str,
                _ => IrType::Int,
            };
            IrType::Result(Box::new(inner))
        } else if ret_name == "Float" {
            IrType::Float
        } else if ret_name == "JsonValue" {
            IrType::JsonValue
        } else if ret_name == "HttpResponse" {
            IrType::HttpResponse
        } else if struct_defs.contains_key(ret_name) {
            IrType::Struct(ret_name.clone())
        } else if enum_defs.contains_key(ret_name) {
            IrType::Enum(ret_name.clone())
        } else {
            continue; // Int, Bool, String — default IrType::Int is fine
        };
        local_fn_ret_types.insert(f.name.clone(), ir_type);
    }

    let mut functions: Vec<IrFunction> = program.functions.iter()
        .map(|f| {
            let func_name = if let Some(mod_name) = module_name {
                format!("{}__{}", mod_name, f.name)
            } else {
                f.name.clone()
            };
            lower_function_named(f, &func_name, &struct_defs, &enum_defs, &module_names, module_fn_ret_types, &local_fn_ret_types)
        })
        .collect();

    // Lower impl methods as mangled functions
    for imp in &program.impls {
        for method in &imp.methods {
            let mangled = if let Some(mod_name) = module_name {
                format!("{}__{}__{}", mod_name, imp.target_type, method.name)
            } else {
                format!("{}_{}", imp.target_type, method.name)
            };
            functions.push(lower_function_named(method, &mangled, &struct_defs, &enum_defs, &module_names, module_fn_ret_types, &local_fn_ret_types));
        }
    }

    Module { functions }
}

fn lower_function_named(func: &cyflym_parser::ast::Function, func_name: &str, struct_defs: &HashMap<String, Vec<String>>, enum_defs: &HashMap<String, Vec<(String, usize, usize)>>, module_names: &[String], module_fn_ret_types: &HashMap<(String, String), IrType>, local_fn_ret_types: &HashMap<String, IrType>) -> IrFunction {
    let mut builder = IrBuilder::new(struct_defs.clone(), enum_defs.clone(), module_names.to_vec(), module_fn_ret_types.clone(), local_fn_ret_types.clone());

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
        name: func_name.to_string(),
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
    module_names: Vec<String>,
    module_fn_ret_types: HashMap<(String, String), IrType>,
    local_fn_ret_types: HashMap<String, IrType>,
}

impl IrBuilder {
    fn new(struct_defs: HashMap<String, Vec<String>>, enum_defs: HashMap<String, Vec<(String, usize, usize)>>, module_names: Vec<String>, module_fn_ret_types: HashMap<(String, String), IrType>, local_fn_ret_types: HashMap<String, IrType>) -> Self {
        IrBuilder {
            counter: 0,
            label_counter: 0,
            locals: HashMap::new(),
            instructions: Vec::new(),
            reg_types: HashMap::new(),
            struct_defs,
            enum_defs,
            module_names,
            module_fn_ret_types,
            local_fn_ret_types,
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
            Expr::FloatLiteral { value, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::FloatConst { dest: dest.clone(), value: *value });
                self.reg_types.insert(dest.clone(), IrType::Float);
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

                // Check for String + String → StringConcat
                if matches!(op, BinOp::Add) && self.reg_types.get(&left_reg) == Some(&IrType::Str) {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::StringConcat {
                        dest: dest.clone(),
                        left: left_reg,
                        right: right_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                }

                // Check if this is a float operation
                let is_float = self.reg_types.get(&left_reg) == Some(&IrType::Float);

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
                        if is_float {
                            self.instructions.push(Instruction::FloatBinOp {
                                dest: dest.clone(),
                                op: ir_op,
                                left: left_reg,
                                right: right_reg,
                            });
                            self.reg_types.insert(dest.clone(), IrType::Float);
                        } else {
                            self.instructions.push(Instruction::BinOp {
                                dest: dest.clone(),
                                op: ir_op,
                                left: left_reg,
                                right: right_reg,
                            });
                            self.reg_types.insert(dest.clone(), IrType::Int);
                        }
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
                        if is_float {
                            self.instructions.push(Instruction::FloatCmpOp {
                                dest: dest.clone(),
                                op: cmp_op,
                                left: left_reg,
                                right: right_reg,
                            });
                        } else {
                            self.instructions.push(Instruction::CmpOp {
                                dest: dest.clone(),
                                op: cmp_op,
                                left: left_reg,
                                right: right_reg,
                            });
                        }
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
                let a_type = self.reg_types.get(&then_reg).cloned().unwrap_or(IrType::Int);
                let b_type = self.reg_types.get(&else_reg).cloned().unwrap_or(IrType::Int);
                // For Result types: err() produces Result(Int) as default.
                // Prefer the branch with the real inner type (from ok()).
                let phi_type = match (&a_type, &b_type) {
                    (IrType::Result(a_inner), IrType::Result(b_inner))
                        if **a_inner == IrType::Int && **b_inner != IrType::Int => b_type,
                    _ => a_type,
                };
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
                        IrType::Array(_) => panic!("cannot print array"),
                        IrType::JsonValue => panic!("cannot print JsonValue"),
                        IrType::Float => self.instructions.push(Instruction::PrintFloat { value: arg_reg }),
                        IrType::HttpResponse => panic!("cannot print HttpResponse"),
                        IrType::Result(_) => panic!("cannot print Result"),
                    }
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                }

                if function == "int_to_string" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::IntToString {
                        dest: dest.clone(),
                        value: val_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                }

                if function == "string_to_int" {
                    let str_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::StringToInt {
                        dest: dest.clone(),
                        string: str_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "file_read" {
                    let path_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FileRead {
                        dest: dest.clone(),
                        path: path_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                } else if function == "file_write" {
                    let path_reg = self.lower_expr(&args[0]);
                    let content_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FileWrite {
                        dest: dest.clone(),
                        path: path_reg,
                        content: content_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "file_append" {
                    let path_reg = self.lower_expr(&args[0]);
                    let content_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FileAppend {
                        dest: dest.clone(),
                        path: path_reg,
                        content: content_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "file_exists" {
                    let path_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FileExists {
                        dest: dest.clone(),
                        path: path_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Bool);
                    return dest;
                } else if function == "json_parse" {
                    let source_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonParse { dest: dest.clone(), source: source_reg });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_object" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonObject { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_array" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonArray { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_string" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonString { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_int" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonInt { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_bool" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonBool { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_null" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonNull { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_stringify" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonStringify { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                } else if function == "int_to_float" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::IntToFloat { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Float);
                    return dest;
                } else if function == "float_to_int" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FloatToInt { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "float_to_string" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FloatToString { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                } else if function == "ok" {
                    let val_reg = self.lower_expr(&args[0]);
                    let val_type = self.reg_types.get(&val_reg).cloned().unwrap_or(IrType::Int);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::ResultOk { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Result(Box::new(val_type)));
                    return dest;
                } else if function == "err" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::ResultErr { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Result(Box::new(IrType::Int))); // default inner
                    return dest;
                } else if function == "log_debug" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogDebug { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "log_info" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogInfo { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "log_warn" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogWarn { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "log_error" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogError { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "log_set_level" {
                    let level_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogSetLevel { dest: dest.clone(), level: level_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "http_get" {
                    let url_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::HttpGet { dest: dest.clone(), url: url_reg });
                    self.reg_types.insert(dest.clone(), IrType::HttpResponse);
                    return dest;
                } else if function == "http_post" {
                    let url_reg = self.lower_expr(&args[0]);
                    let body_reg = self.lower_expr(&args[1]);
                    let ct_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::HttpPost { dest: dest.clone(), url: url_reg, body: body_reg, content_type: ct_reg });
                    self.reg_types.insert(dest.clone(), IrType::HttpResponse);
                    return dest;
                }

                let arg_regs: Vec<Reg> = args.iter().map(|a| self.lower_expr(a)).collect();
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::Call {
                    dest: dest.clone(),
                    function: function.clone(),
                    args: arg_regs,
                });
                // Use tracked return type if available (for Result, struct, enum, etc.)
                let ret_type = self.local_fn_ret_types.get(function).cloned().unwrap_or(IrType::Int);
                self.reg_types.insert(dest.clone(), ret_type);
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
                let num_fields = struct_fields.len();
                let field_index = struct_fields.iter().position(|n| n == field)
                    .expect("unknown field in field access");
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::FieldLoad {
                    dest: dest.clone(),
                    ptr: obj_reg,
                    field_index,
                    num_fields,
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
                // Check for cross-module function call
                if let Expr::Identifier { name, .. } = object.as_ref() {
                    if self.module_names.contains(name) {
                        let mangled_name = format!("{}__{}", name, method);
                        let arg_regs: Vec<Reg> = args.iter()
                            .map(|a| self.lower_expr(a))
                            .collect();
                        let dest = self.fresh_reg();
                        let ret_type = self.module_fn_ret_types
                            .get(&(name.clone(), method.clone()))
                            .cloned()
                            .unwrap_or(IrType::Int);
                        self.reg_types.insert(dest.clone(), ret_type);
                        self.instructions.push(Instruction::Call {
                            dest: dest.clone(),
                            function: mangled_name,
                            args: arg_regs,
                        });
                        return dest;
                    }
                }

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
                    (Some(IrType::Array(_)), "push") => {
                        let val_reg = self.lower_expr(&args[0]);
                        self.instructions.push(Instruction::ArrayPush {
                            array: obj_reg,
                            value: val_reg,
                        });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Array(inner)), "get") => {
                        let elem_type = *inner;
                        let idx_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayGet {
                            dest: dest.clone(),
                            array: obj_reg,
                            index: idx_reg,
                        });
                        self.reg_types.insert(dest.clone(), elem_type);
                        return dest;
                    }
                    (Some(IrType::Array(_)), "set") => {
                        let idx_reg = self.lower_expr(&args[0]);
                        let val_reg = self.lower_expr(&args[1]);
                        self.instructions.push(Instruction::ArraySet {
                            array: obj_reg,
                            index: idx_reg,
                            value: val_reg,
                        });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Array(_)), "len") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayLen {
                            dest: dest.clone(),
                            array: obj_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Str), "len") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringLen {
                            dest: dest.clone(),
                            string: obj_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Str), "substring") => {
                        let start_reg = self.lower_expr(&args[0]);
                        let end_reg = self.lower_expr(&args[1]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringSubstring {
                            dest: dest.clone(),
                            string: obj_reg,
                            start: start_reg,
                            end: end_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "get") => {
                        let key_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::JsonGet { dest: dest.clone(), object: obj_reg, key: key_reg });
                        self.reg_types.insert(dest.clone(), IrType::JsonValue);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "get_index") => {
                        let idx_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::JsonGetIndex { dest: dest.clone(), array: obj_reg, index: idx_reg });
                        self.reg_types.insert(dest.clone(), IrType::JsonValue);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "get_string") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::JsonGetString { dest: dest.clone(), value: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "get_int") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::JsonGetInt { dest: dest.clone(), value: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "get_bool") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::JsonGetBool { dest: dest.clone(), value: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "len") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::JsonLen { dest: dest.clone(), value: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "type_of") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::JsonTypeOf { dest: dest.clone(), value: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "set") => {
                        let key_reg = self.lower_expr(&args[0]);
                        let val_reg = self.lower_expr(&args[1]);
                        self.instructions.push(Instruction::JsonSet { object: obj_reg, key: key_reg, value: val_reg });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::JsonValue), "push") => {
                        let val_reg = self.lower_expr(&args[0]);
                        self.instructions.push(Instruction::JsonPush { array: obj_reg, value: val_reg });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::HttpResponse), "status") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::HttpStatus { dest: dest.clone(), response: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::HttpResponse), "body") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::HttpBody { dest: dest.clone(), response: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::HttpResponse), "header") => {
                        let name_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::HttpHeader { dest: dest.clone(), response: obj_reg, name: name_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::HttpResponse), "ok") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::HttpOk { dest: dest.clone(), response: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Result(ref inner)), "is_ok") => {
                        let _ = inner;
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultIsOk { dest: dest.clone(), result: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Result(ref inner)), "is_err") => {
                        let _ = inner;
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultIsErr { dest: dest.clone(), result: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Result(ref inner)), "unwrap") => {
                        let unwrap_type = *inner.clone();
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultUnwrap { dest: dest.clone(), result: obj_reg });
                        self.reg_types.insert(dest.clone(), unwrap_type);
                        return dest;
                    }
                    (Some(IrType::Result(ref inner)), "unwrap_or") => {
                        let unwrap_type = *inner.clone();
                        let default_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultUnwrapOr { dest: dest.clone(), result: obj_reg, default: default_reg });
                        self.reg_types.insert(dest.clone(), unwrap_type);
                        return dest;
                    }
                    (Some(IrType::Result(ref inner)), "error") => {
                        let _ = inner;
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultError { dest: dest.clone(), result: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
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
            Expr::ArrayCreate { element_type, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::ArrayCreate {
                    dest: dest.clone(),
                });
                let inner_ir_type = match element_type.name.as_str() {
                    "Int" => IrType::Int,
                    "Bool" => IrType::Bool,
                    "String" => IrType::Str,
                    other => IrType::Struct(other.to_string()),
                };
                self.reg_types.insert(dest.clone(), IrType::Array(Box::new(inner_ir_type)));
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
            Stmt::ForIn { var, iterable, body, .. } => {
                let arr_reg = self.lower_expr(iterable);
                // len = ArrayLen(arr)
                let len_reg = self.fresh_reg();
                self.instructions.push(Instruction::ArrayLen {
                    dest: len_reg.clone(),
                    array: arr_reg.clone(),
                });
                self.reg_types.insert(len_reg.clone(), IrType::Int);
                // idx = 0 (use Alloca+Store for mutable counter, same as mut vars)
                let idx_ptr = self.fresh_reg();
                self.instructions.push(Instruction::Alloca { dest: idx_ptr.clone() });
                let zero_reg = self.fresh_reg();
                self.instructions.push(Instruction::Const { dest: zero_reg.clone(), value: 0 });
                self.instructions.push(Instruction::Store { ptr: idx_ptr.clone(), value: zero_reg });
                // Determine element IrType from array's IrType
                let elem_ir_type = match self.reg_types.get(&arr_reg) {
                    Some(IrType::Array(inner)) => inner.as_ref().clone(),
                    _ => IrType::Int, // fallback
                };
                // Loop structure — follows the exact While lowering pattern
                let cond_label = self.fresh_label("forin_cond");
                let body_label = self.fresh_label("forin_body");
                let end_label = self.fresh_label("forin_end");

                self.instructions.push(Instruction::Jump { target: cond_label.clone() });

                self.instructions.push(Instruction::Label { name: cond_label.clone() });
                // Load idx, compare idx < len
                let idx_reg = self.fresh_reg();
                self.instructions.push(Instruction::Load { dest: idx_reg.clone(), ptr: idx_ptr.clone() });
                let cmp_reg = self.fresh_reg();
                self.instructions.push(Instruction::CmpOp {
                    dest: cmp_reg.clone(),
                    op: IrCmpOp::Lt,
                    left: idx_reg.clone(),
                    right: len_reg.clone(),
                });
                self.instructions.push(Instruction::Branch {
                    cond: cmp_reg,
                    then_label: body_label.clone(),
                    else_label: end_label.clone(),
                });

                self.instructions.push(Instruction::Label { name: body_label.clone() });
                // x = ArrayGet(arr, idx)
                let elem_reg = self.fresh_reg();
                self.instructions.push(Instruction::ArrayGet {
                    dest: elem_reg.clone(),
                    array: arr_reg.clone(),
                    index: idx_reg.clone(),
                });
                self.reg_types.insert(elem_reg.clone(), elem_ir_type);
                self.locals.insert(var.clone(), LocalVar::Value(elem_reg));

                // Lower body
                for stmt in body {
                    self.lower_stmt(stmt);
                }

                // idx = idx + 1
                let cur_idx = self.fresh_reg();
                self.instructions.push(Instruction::Load { dest: cur_idx.clone(), ptr: idx_ptr.clone() });
                let one_reg = self.fresh_reg();
                self.instructions.push(Instruction::Const { dest: one_reg.clone(), value: 1 });
                let next_idx = self.fresh_reg();
                self.instructions.push(Instruction::BinOp {
                    dest: next_idx.clone(),
                    op: IrBinOp::Add,
                    left: cur_idx,
                    right: one_reg,
                });
                self.instructions.push(Instruction::Store { ptr: idx_ptr, value: next_idx });

                self.instructions.push(Instruction::Jump { target: cond_label });
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
        let module = lower(&program, None, &HashMap::new());

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
        let module = lower(&program, None, &HashMap::new());

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
        let module = lower(&program, None, &HashMap::new());

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
        let module = lower(&program, None, &HashMap::new());
        let func = &module.functions[0];
        let has_bool_const = func.body.iter().any(|i| matches!(i, Instruction::BoolConst { value: true, .. }));
        assert!(has_bool_const, "expected BoolConst(true)");
    }

    #[test]
    fn lower_comparison() {
        let program = parse("fn main() Bool { 1 == 2 }");
        let module = lower(&program, None, &HashMap::new());
        let func = &module.functions[0];
        let has_cmp = func.body.iter().any(|i| matches!(i, Instruction::CmpOp { .. }));
        assert!(has_cmp, "expected CmpOp instruction");
    }

    #[test]
    fn lower_if_else() {
        let program = parse("fn main() Int { if true { 1 } else { 2 } }");
        let module = lower(&program, None, &HashMap::new());
        let func = &module.functions[0];
        let has_branch = func.body.iter().any(|i| matches!(i, Instruction::Branch { .. }));
        let has_phi = func.body.iter().any(|i| matches!(i, Instruction::Phi { .. }));
        assert!(has_branch, "expected Branch instruction");
        assert!(has_phi, "expected Phi instruction");
    }

    #[test]
    fn lower_not() {
        let program = parse("fn main() Bool { !true }");
        let module = lower(&program, None, &HashMap::new());
        let func = &module.functions[0];
        let has_not = func.body.iter().any(|i| matches!(i, Instruction::Not { .. }));
        assert!(has_not, "expected Not instruction");
    }

    #[test]
    fn lower_while_loop() {
        let program = parse("fn main() Int { let mut x Int = 0 while x < 10 { x = x + 1 } x }");
        let module = lower(&program, None, &HashMap::new());
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
        let module = lower(&program, None, &HashMap::new());
        let func = &module.functions[0];
        let has_ret = func.body.iter().any(|i| matches!(i, Instruction::Ret { .. }));
        assert!(has_ret, "expected Ret instruction");
    }

    #[test]
    fn lower_print_string() {
        let program = parse(r#"fn main() Int { print("hello") }"#);
        let module = lower(&program, None, &HashMap::new());
        let func = &module.functions[0];
        let has_print = func.body.iter().any(|i| matches!(i, Instruction::PrintString { .. }));
        let has_str = func.body.iter().any(|i| matches!(i, Instruction::StringConst { .. }));
        assert!(has_print, "expected PrintString");
        assert!(has_str, "expected StringConst");
    }

    #[test]
    fn lower_print_int() {
        let program = parse("fn main() Int { print(42) }");
        let module = lower(&program, None, &HashMap::new());
        let func = &module.functions[0];
        let has_print = func.body.iter().any(|i| matches!(i, Instruction::PrintInt { .. }));
        assert!(has_print, "expected PrintInt");
    }

    #[test]
    fn lower_struct_literal() {
        let program = parse("struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 1, y: 2 } 0 }");
        let module = lower(&program, None, &HashMap::new());
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_alloc = func.body.iter().any(|i| matches!(i, Instruction::StructAlloc { num_fields: 2, .. }));
        let store_count = func.body.iter().filter(|i| matches!(i, Instruction::FieldStore { .. })).count();
        assert!(has_alloc, "expected StructAlloc with 2 fields");
        assert_eq!(store_count, 2, "expected 2 FieldStore instructions");
    }

    #[test]
    fn lower_field_access() {
        let program = parse("struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 1, y: 2 } p.x }");
        let module = lower(&program, None, &HashMap::new());
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_load = func.body.iter().any(|i| matches!(i, Instruction::FieldLoad { field_index: 0, .. }));
        assert!(has_load, "expected FieldLoad with field_index 0");
    }

    #[test]
    fn lower_enum_variant() {
        let program = parse("enum Color { Red, Green, Blue, } fn main() Int { let c = Color::Green 0 }");
        let module = lower(&program, None, &HashMap::new());
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_alloc = func.body.iter().any(|i| matches!(i, Instruction::EnumAlloc { tag: 1, num_data_fields: 0, .. }));
        assert!(has_alloc, "expected EnumAlloc with tag=1, num_data_fields=0");
    }

    #[test]
    fn lower_match_expr() {
        let program = parse(
            "enum Color { Red, Green, Blue, } fn main() Int { let c = Color::Green match c { Color::Red => 1, Color::Green => 2, Color::Blue => 3, } }"
        );
        let module = lower(&program, None, &HashMap::new());
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
        let module = lower(&program, None, &HashMap::new());
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
        let module = lower(&program, None, &HashMap::new());
        let point_add = module.functions.iter().find(|f| f.name == "Point_add").expect("no Point_add");
        assert_eq!(point_add.params.len(), 2); // self + n
    }

    #[test]
    fn lower_mutable_variable() {
        let program = parse("fn main() Int { let mut x Int = 1 x = 2 x }");
        let module = lower(&program, None, &HashMap::new());
        let func = &module.functions[0];
        let has_alloca = func.body.iter().any(|i| matches!(i, Instruction::Alloca { .. }));
        let store_count = func.body.iter().filter(|i| matches!(i, Instruction::Store { .. })).count();
        assert!(has_alloca, "expected Alloca");
        assert!(store_count >= 2, "expected at least 2 Store instructions (init + reassign)");
    }

    #[test]
    fn lower_generic_function() {
        let program = parse("fn identity<T>(x T) T { x } fn main() Int { identity(42) }");
        let module = lower(&program, None, &HashMap::new());
        assert_eq!(module.functions.len(), 2);
        let identity = module.functions.iter().find(|f| f.name == "identity").expect("no identity");
        assert_eq!(identity.params.len(), 1);
    }

    #[test]
    fn lower_spawn() {
        let program = parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() 0 }");
        let module = lower(&program, None, &HashMap::new());
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_spawn = main_func.body.iter().any(|i| matches!(i, Instruction::ThreadSpawn { .. }));
        assert!(has_spawn, "expected ThreadSpawn instruction");
    }

    #[test]
    fn lower_channel_create() {
        let program = parse("fn main() Int { let (tx, rx) = channel<Int>() 0 }");
        let module = lower(&program, None, &HashMap::new());
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_create = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelCreate { .. }));
        assert!(has_create, "expected ChannelCreate instruction");
    }

    #[test]
    fn lower_send_recv() {
        let program = parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }");
        let module = lower(&program, None, &HashMap::new());
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_send = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelSend { .. }));
        let has_recv = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelRecv { .. }));
        assert!(has_send, "expected ChannelSend instruction");
        assert!(has_recv, "expected ChannelRecv instruction");
    }

    #[test]
    fn lower_join() {
        let program = parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() h.join() }");
        let module = lower(&program, None, &HashMap::new());
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_join = main_func.body.iter().any(|i| matches!(i, Instruction::ThreadJoin { .. }));
        assert!(has_join, "expected ThreadJoin instruction");
    }

    #[test]
    fn lower_mutex_create_lock_unlock() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let m = mutex(5) let v = m.lock() m.unlock(v) 0 }"
        ).unwrap();
        let module = lower(&prog, None, &HashMap::new());
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
        let module = lower(&prog, None, &HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ChannelCreateBounded { .. })),
            "expected ChannelCreateBounded instruction");
    }

    #[test]
    fn lower_unbounded_channel_unchanged() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let (tx, rx) = channel<Int>() tx.send(1) rx.recv() }"
        ).unwrap();
        let module = lower(&prog, None, &HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ChannelCreate { .. })),
            "expected ChannelCreate (not bounded) instruction");
        assert!(!instrs.iter().any(|i| matches!(i, Instruction::ChannelCreateBounded { .. })),
            "should NOT have ChannelCreateBounded");
    }

    #[test]
    fn lower_array_create_push_get_len() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let a = array<Int>() a.push(5) a.get(0) }"
        ).unwrap();
        let module = lower(&prog, None, &HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayCreate { .. })),
            "expected ArrayCreate instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayPush { .. })),
            "expected ArrayPush instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayGet { .. })),
            "expected ArrayGet instruction");
    }

    #[test]
    fn lower_for_in_to_counted_loop() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let a = array<Int>() a.push(1) for x in a { print(x) } 0 }"
        ).unwrap();
        let module = lower(&prog, None, &HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayLen { .. })),
            "expected ArrayLen for for-in loop");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayGet { .. })),
            "expected ArrayGet for for-in loop");
    }

    #[test]
    fn lower_string_concat() {
        let prog = cyflym_parser::parse(
            r#"fn main() Int { let s = "a" + "b" 0 }"#
        ).unwrap();
        let module = lower(&prog, None, &HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::StringConcat { .. })),
            "expected StringConcat instruction");
    }

    #[test]
    fn lower_int_to_string_and_string_to_int() {
        let prog = cyflym_parser::parse(
            r#"fn main() Int { let s = int_to_string(42) string_to_int(s) }"#
        ).unwrap();
        let module = lower(&prog, None, &HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::IntToString { .. })),
            "expected IntToString instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::StringToInt { .. })),
            "expected StringToInt instruction");
    }

    #[test]
    fn lower_with_module_name_mangles_functions() {
        let program = parse("fn add(a Int, b Int) Int { a + b }");
        let module = lower(&program, Some("utils"), &HashMap::new());
        let func = module.functions.iter().find(|f| f.name == "utils__add");
        assert!(func.is_some(), "expected mangled function name 'utils__add'");
    }

    #[test]
    fn lower_cross_module_call_uses_mangled_name() {
        let program = parse("import \"utils\"\nfn main() Int { utils.add(1, 2) }");
        let module = lower(&program, None, &HashMap::new());
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_mangled_call = func.body.iter().any(|i| {
            matches!(i, Instruction::Call { function, .. } if function == "utils__add")
        });
        assert!(has_mangled_call, "expected call to 'utils__add', got: {:?}",
            func.body.iter().filter(|i| matches!(i, Instruction::Call { .. })).collect::<Vec<_>>());
    }

    #[test]
    fn lower_file_read_instruction() {
        let program = parse("fn main() Int { let s = file_read(\"test.txt\") 0 }");
        let module = lower(&program, None, &HashMap::new());
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_file_read = func.body.iter().any(|i| {
            matches!(i, Instruction::FileRead { .. })
        });
        assert!(has_file_read, "expected FileRead instruction, got: {:?}", func.body);
    }

    #[test]
    fn lower_file_write_instruction() {
        let program = parse("fn main() Int { file_write(\"test.txt\", \"hello\") }");
        let module = lower(&program, None, &HashMap::new());
        let func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_file_write = func.body.iter().any(|i| {
            matches!(i, Instruction::FileWrite { .. })
        });
        assert!(has_file_write, "expected FileWrite instruction, got: {:?}", func.body);
    }

    #[test]
    fn lower_json_parse() {
        let program = cyflym_parser::parse("fn main() Int { let v = json_parse(\"{}\") \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::JsonParse { .. })),
            "expected JsonParse instruction");
    }

    #[test]
    fn lower_json_object() {
        let program = cyflym_parser::parse("fn main() Int { let v = json_object() \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::JsonObject { .. })),
            "expected JsonObject instruction");
    }

    #[test]
    fn lower_json_stringify() {
        let program = cyflym_parser::parse("fn main() Int { let v = json_object() \n let s = json_stringify(v) \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::JsonStringify { .. })),
            "expected JsonStringify instruction");
    }

    #[test]
    fn lower_json_get_method() {
        let program = cyflym_parser::parse("fn main() Int { let v = json_parse(\"{}\") \n let inner = v.get(\"key\") \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::JsonGet { .. })),
            "expected JsonGet instruction");
    }

    #[test]
    fn lower_log_info() {
        let program = cyflym_parser::parse("fn main() Int { log_info(\"hello\") }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::LogInfo { .. })),
            "expected LogInfo instruction");
    }

    #[test]
    fn lower_log_set_level() {
        let program = cyflym_parser::parse("fn main() Int { log_set_level(2) }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::LogSetLevel { .. })),
            "expected LogSetLevel instruction");
    }

    #[test]
    fn lower_http_get() {
        let program = cyflym_parser::parse("fn main() Int { let r = http_get(\"http://example.com\") \n r.status() }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::HttpGet { .. })),
            "expected HttpGet instruction");
    }

    #[test]
    fn lower_http_post() {
        let program = cyflym_parser::parse("fn main() Int { let r = http_post(\"http://example.com\", \"body\", \"text/plain\") \n r.status() }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::HttpPost { .. })),
            "expected HttpPost instruction");
    }

    #[test]
    fn lower_http_body_method() {
        let program = cyflym_parser::parse("fn main() Int { let r = http_get(\"http://example.com\") \n let b = r.body() \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::HttpBody { .. })),
            "expected HttpBody instruction");
    }

    #[test]
    fn lower_result_ok() {
        let program = cyflym_parser::parse("fn main() Int { let r = ok(42) \n r.unwrap() }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ResultOk { .. })),
            "expected ResultOk instruction");
    }

    #[test]
    fn lower_result_err() {
        let program = cyflym_parser::parse("fn main() Int { let r = err(\"bad\") \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ResultErr { .. })),
            "expected ResultErr instruction");
    }

    #[test]
    fn lower_result_unwrap() {
        let program = cyflym_parser::parse("fn main() Int { let r = ok(42) \n r.unwrap() }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ResultUnwrap { .. })),
            "expected ResultUnwrap instruction");
    }
}
