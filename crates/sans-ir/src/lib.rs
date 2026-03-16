pub mod ir;

use std::collections::{HashMap, HashSet};

use sans_parser::ast::{BinOp, Expr, Program, Stmt};
use ir::{Instruction, IrBinOp, IrCmpOp, IrFunction, Module, Reg};

#[derive(Clone, PartialEq, Debug)]
pub enum IrType { Int, Float, Bool, Str, Struct(String), Enum(String), Sender, Receiver, JoinHandle, Mutex, Array(Box<IrType>), JsonValue, HttpResponse, Result(Box<IrType>), HttpServer, HttpRequest, Tuple(Vec<IrType>), Map }

pub fn ir_type_for_return(ty: &sans_typeck::types::Type) -> IrType {
    use sans_typeck::types::Type;
    match ty {
        Type::Int => IrType::Int,
        Type::Float => IrType::Float,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Str,
        Type::Struct { name, .. } => IrType::Struct(name.clone()),
        Type::Enum { name, .. } => IrType::Enum(name.clone()),
        Type::Array { inner } => IrType::Array(Box::new(ir_type_for_return(inner))),
        Type::JsonValue => IrType::JsonValue,
        Type::Map => IrType::Map,
        Type::HttpResponse => IrType::HttpResponse,
        Type::HttpServer => IrType::HttpServer,
        Type::HttpRequest => IrType::HttpRequest,
        Type::Result { inner } => IrType::Result(Box::new(ir_type_for_return(inner))),
        Type::ResultErr => IrType::Result(Box::new(IrType::Int)), // default inner type for err
        Type::Tuple { elements } => {
            IrType::Tuple(elements.iter().map(ir_type_for_return).collect())
        }
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
    lower_with_extra_structs(program, module_name, module_fn_ret_types, &HashMap::new(), &HashMap::new())
}

/// Like `lower`, but merges `extra_struct_defs` (from imported modules) into the struct
/// definitions available during lowering. This allows the main module to perform field
/// accesses on structs that are defined in imported modules.
pub fn lower_with_extra_structs(
    program: &Program,
    module_name: Option<&str>,
    module_fn_ret_types: &HashMap<(String, String), IrType>,
    extra_struct_defs: &HashMap<String, Vec<String>>,
    extra_globals: &HashMap<String, IrType>,
) -> Module {
    // Collect struct definitions: name -> field names (ordered)
    let mut struct_defs: HashMap<String, Vec<String>> = extra_struct_defs.clone();
    for s in &program.structs {
        let field_names: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
        struct_defs.insert(s.name.clone(), field_names);
    }
    // Collect struct field types: name -> [(field_name, IrType)]
    let mut struct_field_types: HashMap<String, Vec<(String, IrType)>> = HashMap::new();
    for s in &program.structs {
        let fields: Vec<(String, IrType)> = s.fields.iter()
            .map(|f| {
                let ir_type = match f.type_name.name.as_str() {
                    "String" | "S" => IrType::Str,
                    "Float" | "F" => IrType::Float,
                    "Bool" | "B" => IrType::Bool,
                    "Map" | "M" => IrType::Map,
                    "JsonValue" => IrType::JsonValue,
                    "HttpResponse" => IrType::HttpResponse,
                    "HttpRequest" | "HR" => IrType::HttpRequest,
                    "HttpServer" | "HS" => IrType::HttpServer,
                    name if struct_defs.contains_key(name) => IrType::Struct(name.to_string()),
                    name if name.starts_with("Array<") && name.ends_with('>') => {
                        let inner_str = &name[6..name.len()-1];
                        let inner = match inner_str {
                            "Int" | "I" => IrType::Int,
                            "Float" | "F" => IrType::Float,
                            "Bool" | "B" => IrType::Bool,
                            "String" | "S" => IrType::Str,
                            _ => IrType::Int,
                        };
                        IrType::Array(Box::new(inner))
                    }
                    _ => IrType::Int,
                };
                (f.name.clone(), ir_type)
            })
            .collect();
        struct_field_types.insert(s.name.clone(), fields);
    }

    // Collect enum definitions: name -> [(variant_name, tag_index, num_data_fields)]
    let mut enum_defs: HashMap<String, Vec<(String, usize, usize)>> = HashMap::new();
    for e in &program.enums {
        let variants: Vec<(String, usize, usize)> = e.variants.iter().enumerate()
            .map(|(i, v)| (v.name.clone(), i, v.fields.len()))
            .collect();
        enum_defs.insert(e.name.clone(), variants);
    }

    // Collect enum field types: (enum_name, variant_name) -> [IrType] for data fields
    let mut enum_field_types: HashMap<(String, String), Vec<IrType>> = HashMap::new();
    for e in &program.enums {
        for v in &e.variants {
            let field_types: Vec<IrType> = v.fields.iter()
                .map(|tn| {
                    match tn.name.as_str() {
                        "String" | "S" => IrType::Str,
                        "Float" | "F" => IrType::Float,
                        "Bool" | "B" => IrType::Bool,
                        "Map" | "M" => IrType::Map,
                        "JsonValue" => IrType::JsonValue,
                        "HttpResponse" => IrType::HttpResponse,
                        "HttpRequest" | "HR" => IrType::HttpRequest,
                        "HttpServer" | "HS" => IrType::HttpServer,
                        name if struct_defs.contains_key(name) => IrType::Struct(name.to_string()),
                        name if enum_defs.contains_key(name) => IrType::Enum(name.to_string()),
                        name if name.starts_with("Array<") && name.ends_with('>') => {
                            let inner_str = &name[6..name.len()-1];
                            let inner = match inner_str {
                                "Int" | "I" => IrType::Int,
                                "Float" | "F" => IrType::Float,
                                "Bool" | "B" => IrType::Bool,
                                "String" | "S" => IrType::Str,
                                _ => IrType::Int,
                            };
                            IrType::Array(Box::new(inner))
                        }
                        _ => IrType::Int,
                    }
                })
                .collect();
            enum_field_types.insert((e.name.clone(), v.name.clone()), field_types);
        }
    }

    // Process global variable definitions
    let mut globals: Vec<(String, i64)> = Vec::new();
    let mut global_names: HashMap<String, IrType> = extra_globals.clone();
    for gdef in &program.globals {
        let init_value = match &gdef.value {
            Expr::IntLiteral { value, .. } => *value,
            Expr::BoolLiteral { value, .. } => if *value { 1 } else { 0 },
            _ => 0, // default for non-constant init
        };
        let ir_type = match &gdef.value {
            Expr::IntLiteral { .. } => IrType::Int,
            Expr::FloatLiteral { .. } => IrType::Float,
            Expr::BoolLiteral { .. } => IrType::Bool,
            Expr::StringLiteral { .. } => IrType::Str,
            _ => IrType::Int,
        };
        globals.push((gdef.name.clone(), init_value));
        global_names.insert(gdef.name.clone(), ir_type);
    }

    let module_names: Vec<String> = program.imports.iter()
        .map(|imp| imp.module_name.clone())
        .collect();

    // Build local function return type map for Result/opaque type tracking
    let mut local_fn_ret_types: HashMap<String, IrType> = HashMap::new();
    for f in &program.functions {
        let ret_name = &f.return_type.name;
        let ir_type = if (ret_name.starts_with("Result<") || ret_name.starts_with("R<")) && ret_name.ends_with('>') {
            let prefix_len = if ret_name.starts_with("Result<") { 7 } else { 2 };
            let inner_str = &ret_name[prefix_len..ret_name.len()-1];
            let inner = match inner_str {
                "Int" | "I" => IrType::Int,
                "Float" | "F" => IrType::Float,
                "Bool" | "B" => IrType::Bool,
                "String" | "S" => IrType::Str,
                _ => IrType::Int,
            };
            IrType::Result(Box::new(inner))
        } else if ret_name == "Float" || ret_name == "F" {
            IrType::Float
        } else if ret_name == "String" || ret_name == "S" {
            IrType::Str
        } else if ret_name == "Bool" || ret_name == "B" {
            IrType::Bool
        } else if ret_name == "JsonValue" {
            IrType::JsonValue
        } else if ret_name == "Map" || ret_name == "M" {
            IrType::Map
        } else if ret_name == "HttpResponse" {
            IrType::HttpResponse
        } else if ret_name == "HttpRequest" || ret_name == "HR" {
            IrType::HttpRequest
        } else if ret_name == "HttpServer" || ret_name == "HS" {
            IrType::HttpServer
        } else if ret_name.starts_with("Array<") && ret_name.ends_with('>') {
            let inner_str = &ret_name[6..ret_name.len()-1];
            let inner = match inner_str {
                "Int" | "I" => IrType::Int,
                "Float" | "F" => IrType::Float,
                "Bool" | "B" => IrType::Bool,
                "String" | "S" => IrType::Str,
                _ => IrType::Int,
            };
            IrType::Array(Box::new(inner))
        } else if ret_name.starts_with('(') && ret_name.ends_with(')') {
            // Tuple return type like "(I I)" or "(Int String Bool)"
            let inner = &ret_name[1..ret_name.len()-1];
            let types: Vec<IrType> = inner.split_whitespace()
                .map(|t| match t {
                    "Int" | "I" => IrType::Int,
                    "Float" | "F" => IrType::Float,
                    "Bool" | "B" => IrType::Bool,
                    "String" | "S" => IrType::Str,
                    _ => {
                        if struct_defs.contains_key(t) {
                            IrType::Struct(t.to_string())
                        } else if enum_defs.contains_key(t) {
                            IrType::Enum(t.to_string())
                        } else {
                            IrType::Int
                        }
                    }
                })
                .collect();
            IrType::Tuple(types)
        } else if struct_defs.contains_key(ret_name) {
            IrType::Struct(ret_name.clone())
        } else if enum_defs.contains_key(ret_name) {
            IrType::Enum(ret_name.clone())
        } else {
            continue; // Int — default IrType::Int is fine
        };
        local_fn_ret_types.insert(f.name.clone(), ir_type);
    }

    // Collect local function names for intra-module call mangling
    let local_fn_names: Vec<String> = program.functions.iter().map(|f| f.name.clone()).collect();
    let current_module = module_name.map(|s| s.to_string());

    // Build imported function name map: bare_name -> mangled_name
    // for functions imported via `import "module"` that can be called without prefix
    let mut imported_fn_names: HashMap<String, String> = HashMap::new();
    for imp in &program.imports {
        let mod_name = &imp.module_name;
        for ((m, f), _) in module_fn_ret_types.iter() {
            if m == mod_name {
                let mangled = format!("{}__{}", m, f);
                imported_fn_names.insert(f.clone(), mangled);
            }
        }
    }

    let mut functions: Vec<IrFunction> = Vec::new();
    for f in &program.functions {
        let func_name = if let Some(mod_name) = module_name {
            format!("{}__{}", mod_name, f.name)
        } else {
            f.name.clone()
        };
        let (main_fn, lifted) = lower_function_named(&f, &func_name, &struct_defs, &enum_defs, &module_names, module_fn_ret_types, &local_fn_ret_types, &global_names, &struct_field_types, &enum_field_types, &current_module, &local_fn_names, &imported_fn_names);
        functions.push(main_fn);
        functions.extend(lifted);
    }

    // Lower impl methods as mangled functions
    for imp in &program.impls {
        for method in &imp.methods {
            let mangled = if let Some(mod_name) = module_name {
                format!("{}__{}__{}", mod_name, imp.target_type, method.name)
            } else {
                format!("{}_{}", imp.target_type, method.name)
            };
            let (main_fn, lifted) = lower_function_named(method, &mangled, &struct_defs, &enum_defs, &module_names, module_fn_ret_types, &local_fn_ret_types, &global_names, &struct_field_types, &enum_field_types, &current_module, &local_fn_names, &imported_fn_names);
            functions.push(main_fn);
            functions.extend(lifted);
        }
    }

    Module { globals, functions }
}

fn lower_function_named(func: &sans_parser::ast::Function, func_name: &str, struct_defs: &HashMap<String, Vec<String>>, enum_defs: &HashMap<String, Vec<(String, usize, usize)>>, module_names: &[String], module_fn_ret_types: &HashMap<(String, String), IrType>, local_fn_ret_types: &HashMap<String, IrType>, global_names: &HashMap<String, IrType>, struct_field_types: &HashMap<String, Vec<(String, IrType)>>, enum_field_types: &HashMap<(String, String), Vec<IrType>>, current_module: &Option<String>, local_fn_names: &[String], imported_fn_names: &HashMap<String, String>) -> (IrFunction, Vec<IrFunction>) {
    let mut builder = IrBuilder::new(struct_defs.clone(), enum_defs.clone(), module_names.to_vec(), module_fn_ret_types.clone(), local_fn_ret_types.clone(), global_names.clone(), struct_field_types.clone(), enum_field_types.clone());
    builder.current_module = current_module.clone();
    builder.local_fn_names = local_fn_names.to_vec();
    builder.imported_fn_names = imported_fn_names.clone();

    // Map params to arg registers
    let params: Vec<Reg> = func
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| {
            let reg = format!("arg{}", i);
            builder.locals.insert(param.name.clone(), LocalVar::Value(reg.clone()));
            // Set type for all params
            if struct_defs.contains_key(&param.type_name.name) {
                builder.reg_types.insert(reg.clone(), IrType::Struct(param.type_name.name.clone()));
            } else if enum_defs.contains_key(&param.type_name.name) {
                builder.reg_types.insert(reg.clone(), IrType::Enum(param.type_name.name.clone()));
            } else {
                match param.type_name.name.as_str() {
                    "Int" | "I" => { builder.reg_types.insert(reg.clone(), IrType::Int); }
                    "Float" | "F" => { builder.reg_types.insert(reg.clone(), IrType::Float); }
                    "Bool" | "B" => { builder.reg_types.insert(reg.clone(), IrType::Bool); }
                    "String" | "S" => { builder.reg_types.insert(reg.clone(), IrType::Str); }
                    "HttpRequest" | "HR" => { builder.reg_types.insert(reg.clone(), IrType::HttpRequest); }
                    "HttpServer" | "HS" => { builder.reg_types.insert(reg.clone(), IrType::HttpServer); }
                    "HttpResponse" => { builder.reg_types.insert(reg.clone(), IrType::HttpResponse); }
                    "JsonValue" => { builder.reg_types.insert(reg.clone(), IrType::JsonValue); }
                    "Map" | "M" => { builder.reg_types.insert(reg.clone(), IrType::Map); }
                    _ => {
                        let n = &param.type_name.name;
                        // Array parameter type like "Array<Int>" or "[I]"
                        if n.starts_with("Array<") && n.ends_with('>') {
                            let inner_str = &n[6..n.len()-1];
                            let inner = match inner_str {
                                "Int" | "I" => IrType::Int,
                                "Float" | "F" => IrType::Float,
                                "Bool" | "B" => IrType::Bool,
                                "String" | "S" => IrType::Str,
                                _ if struct_defs.contains_key(inner_str) => IrType::Struct(inner_str.to_string()),
                                _ if enum_defs.contains_key(inner_str) => IrType::Enum(inner_str.to_string()),
                                _ => IrType::Int,
                            };
                            builder.reg_types.insert(reg.clone(), IrType::Array(Box::new(inner)));
                        }
                        // Tuple parameter type like "(I I)"
                        else if n.starts_with('(') && n.ends_with(')') {
                            let inner = &n[1..n.len()-1];
                            let types: Vec<IrType> = inner.split_whitespace()
                                .map(|t| match t {
                                    "Int" | "I" => IrType::Int,
                                    "Float" | "F" => IrType::Float,
                                    "Bool" | "B" => IrType::Bool,
                                    "String" | "S" => IrType::Str,
                                    _ => IrType::Int,
                                })
                                .collect();
                            builder.reg_types.insert(reg.clone(), IrType::Tuple(types));
                        }
                        // Result<T> or R<T>
                        else if (n.starts_with("Result<") || n.starts_with("R<")) && n.ends_with('>') {
                            let prefix_len = if n.starts_with("Result<") { 7 } else { 2 };
                            let inner_str = &n[prefix_len..n.len()-1];
                            let inner = match inner_str {
                                "Int" | "I" => IrType::Int,
                                "Float" | "F" => IrType::Float,
                                "Bool" | "B" => IrType::Bool,
                                "String" | "S" => IrType::Str,
                                _ => IrType::Int,
                            };
                            builder.reg_types.insert(reg.clone(), IrType::Result(Box::new(inner)));
                        }
                    }
                }
            }
            reg
        })
        .collect();

    let param_struct_sizes: Vec<usize> = func.params.iter()
        .map(|p| {
            let n = &p.type_name.name;
            if let Some(fields) = struct_defs.get(n) {
                fields.len()
            } else if let Some(variants) = enum_defs.get(n) {
                let max_data = variants.iter().map(|(_, _, n)| *n).max().unwrap_or(0);
                1 + max_data // tag + max data fields
            } else if n.starts_with('(') && n.ends_with(')') {
                // Tuple parameter — count fields
                n[1..n.len()-1].split_whitespace().count()
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

    let lifted = builder.lifted_fns;
    let main_fn = IrFunction {
        name: func_name.to_string(),
        params,
        param_struct_sizes,
        body: builder.instructions,
    };
    (main_fn, lifted)
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
    global_names: HashMap<String, IrType>,
    /// Tracks the name of the current basic block (updated when a Label instruction is emitted)
    current_label: Option<String>,
    /// Lambda functions lifted out of the current function during lowering
    lifted_fns: Vec<IrFunction>,
    /// Stack of (cond_label, end_label) for nested loops (used by break/continue)
    loop_stack: Vec<(String, String)>,
    /// Counter for generating unique lambda names
    lambda_counter: usize,
    /// Tracks closure info: dest_register -> (lifted_fn_name, capture_var_names)
    closure_info: HashMap<Reg, (String, Vec<String>)>,
    /// Maps (struct_name, field_name) to the field's IrType for correct field access typing
    struct_field_types: HashMap<String, Vec<(String, IrType)>>,
    /// Maps (enum_name, variant_name) to the variant's field IrTypes
    enum_field_types: HashMap<(String, String), Vec<IrType>>,
    /// Current module name (if in a module context), used to mangle intra-module calls
    current_module: Option<String>,
    /// Set of function names defined in the current module (bare names)
    local_fn_names: Vec<String>,
    /// Maps bare function name -> mangled name for imported module functions
    imported_fn_names: HashMap<String, String>,
}

impl IrBuilder {
    fn new(struct_defs: HashMap<String, Vec<String>>, enum_defs: HashMap<String, Vec<(String, usize, usize)>>, module_names: Vec<String>, module_fn_ret_types: HashMap<(String, String), IrType>, local_fn_ret_types: HashMap<String, IrType>, global_names: HashMap<String, IrType>, struct_field_types: HashMap<String, Vec<(String, IrType)>>, enum_field_types: HashMap<(String, String), Vec<IrType>>) -> Self {
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
            global_names,
            current_label: None,
            lifted_fns: Vec::new(),
            loop_stack: Vec::new(),
            lambda_counter: 0,
            closure_info: HashMap::new(),
            struct_field_types,
            enum_field_types,
            current_module: None,
            local_fn_names: Vec::new(),
            imported_fn_names: HashMap::new(),
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

    fn emit_label(&mut self, name: String) {
        self.current_label = Some(name.clone());
        self.instructions.push(Instruction::Label { name });
    }

    fn find_captures(&self, body: &[Stmt], params: &[sans_parser::ast::Param]) -> Vec<String> {
        let param_names: HashSet<&str> = params.iter().map(|p| p.name.as_str()).collect();
        let mut captures = Vec::new();
        let mut seen = HashSet::new();
        self.collect_idents_in_stmts(body, &param_names, &mut captures, &mut seen);
        captures
    }

    fn collect_idents_in_stmts(&self, stmts: &[Stmt], param_names: &HashSet<&str>, captures: &mut Vec<String>, seen: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Let { value, .. } => self.collect_idents_in_expr(value, param_names, captures, seen),
                Stmt::Assign { value, .. } => self.collect_idents_in_expr(value, param_names, captures, seen),
                Stmt::Return { value, .. } => self.collect_idents_in_expr(value, param_names, captures, seen),
                Stmt::Expr(expr) => self.collect_idents_in_expr(expr, param_names, captures, seen),
                Stmt::While { condition, body, .. } => {
                    self.collect_idents_in_expr(condition, param_names, captures, seen);
                    self.collect_idents_in_stmts(body, param_names, captures, seen);
                }
                Stmt::If { condition, body, .. } => {
                    self.collect_idents_in_expr(condition, param_names, captures, seen);
                    self.collect_idents_in_stmts(body, param_names, captures, seen);
                }
                Stmt::ForIn { iterable, body, .. } => {
                    self.collect_idents_in_expr(iterable, param_names, captures, seen);
                    self.collect_idents_in_stmts(body, param_names, captures, seen);
                }
                Stmt::LetDestructure { value, .. } => self.collect_idents_in_expr(value, param_names, captures, seen),
                Stmt::Break { .. } | Stmt::Continue { .. } => {}
            }
        }
    }

    fn collect_idents_in_expr(&self, expr: &Expr, param_names: &HashSet<&str>, captures: &mut Vec<String>, seen: &mut HashSet<String>) {
        match expr {
            Expr::Identifier { name, .. } => {
                if self.locals.contains_key(name) && !param_names.contains(name.as_str()) && !seen.contains(name) {
                    seen.insert(name.clone());
                    captures.push(name.clone());
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.collect_idents_in_expr(left, param_names, captures, seen);
                self.collect_idents_in_expr(right, param_names, captures, seen);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.collect_idents_in_expr(arg, param_names, captures, seen);
                }
            }
            Expr::MethodCall { object, args, .. } => {
                self.collect_idents_in_expr(object, param_names, captures, seen);
                for arg in args {
                    self.collect_idents_in_expr(arg, param_names, captures, seen);
                }
            }
            Expr::If { condition, then_body, then_expr, else_body, else_expr, .. } => {
                self.collect_idents_in_expr(condition, param_names, captures, seen);
                self.collect_idents_in_stmts(then_body, param_names, captures, seen);
                self.collect_idents_in_expr(then_expr, param_names, captures, seen);
                self.collect_idents_in_stmts(else_body, param_names, captures, seen);
                self.collect_idents_in_expr(else_expr, param_names, captures, seen);
            }
            Expr::UnaryOp { operand, .. } => {
                self.collect_idents_in_expr(operand, param_names, captures, seen);
            }
            Expr::FieldAccess { object, .. } => {
                self.collect_idents_in_expr(object, param_names, captures, seen);
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, expr) in fields {
                    self.collect_idents_in_expr(expr, param_names, captures, seen);
                }
            }
            Expr::EnumVariant { args, .. } => {
                for arg in args {
                    self.collect_idents_in_expr(arg, param_names, captures, seen);
                }
            }
            Expr::Match { scrutinee, arms, .. } => {
                self.collect_idents_in_expr(scrutinee, param_names, captures, seen);
                for arm in arms {
                    self.collect_idents_in_expr(&arm.body, param_names, captures, seen);
                }
            }
            Expr::ArrayLiteral { elements, .. } | Expr::TupleLiteral { elements, .. } => {
                for elem in elements {
                    self.collect_idents_in_expr(elem, param_names, captures, seen);
                }
            }
            Expr::Spawn { args, .. } => {
                for arg in args {
                    self.collect_idents_in_expr(arg, param_names, captures, seen);
                }
            }
            Expr::MutexCreate { value, .. } => {
                self.collect_idents_in_expr(value, param_names, captures, seen);
            }
            Expr::Lambda { body, .. } => {
                // Don't recurse into nested lambda bodies for captures of the outer lambda
                // (nested lambdas handle their own captures)
                let _ = body;
            }
            // Literals and other leaf nodes — no identifiers to capture
            _ => {}
        }
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
                if let Some(local) = self.locals.get(name).cloned() {
                    match local {
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
                } else if let Some(ir_type) = self.global_names.get(name).cloned() {
                    // Global variable — emit GlobalLoad
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::GlobalLoad { dest: dest.clone(), name: name.clone() });
                    self.reg_types.insert(dest.clone(), ir_type);
                    dest
                } else {
                    // Function reference — emit FnRef instruction with mangling
                    let mangled = if let Some(ref mod_name) = self.current_module {
                        if self.local_fn_names.contains(name) {
                            format!("{}__{}", mod_name, name)
                        } else if let Some(m) = self.imported_fn_names.get(name) {
                            m.clone()
                        } else {
                            name.clone()
                        }
                    } else if let Some(m) = self.imported_fn_names.get(name) {
                        m.clone()
                    } else {
                        name.clone()
                    };
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FnRef { dest: dest.clone(), name: mangled });
                    self.reg_types.insert(dest.clone(), IrType::Int); // fn pointer as i64
                    dest
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

                        self.emit_label(rhs_label.clone());
                        let right_reg = self.lower_expr(right);
                        // After lowering right, the current label may have changed
                        // (e.g. if right contains nested && or || or if-else)
                        let rhs_source_label = self.current_label.clone().unwrap_or_else(|| rhs_label.clone());
                        self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                        self.emit_label(false_label.clone());
                        let false_reg = self.fresh_reg();
                        self.instructions.push(Instruction::BoolConst { dest: false_reg.clone(), value: false });
                        self.reg_types.insert(false_reg.clone(), IrType::Bool);
                        self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                        self.emit_label(merge_label.clone());
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Phi {
                            dest: dest.clone(),
                            a_val: right_reg,
                            a_label: rhs_source_label,
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

                        self.emit_label(true_label.clone());
                        let true_reg = self.fresh_reg();
                        self.instructions.push(Instruction::BoolConst { dest: true_reg.clone(), value: true });
                        self.reg_types.insert(true_reg.clone(), IrType::Bool);
                        self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                        self.emit_label(rhs_label.clone());
                        let right_reg = self.lower_expr(right);
                        // After lowering right, the current label may have changed
                        // (e.g. if right contains nested && or || or if-else)
                        let rhs_source_label = self.current_label.clone().unwrap_or_else(|| rhs_label.clone());
                        self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                        self.emit_label(merge_label.clone());
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Phi {
                            dest: dest.clone(),
                            a_val: true_reg,
                            a_label: true_label,
                            b_val: right_reg,
                            b_label: rhs_source_label,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    _ => {} // fall through to normal handling
                }

                // Non-short-circuit: evaluate both sides
                let left_reg = self.lower_expr(left);
                let right_reg = self.lower_expr(right);

                // Check for String + String → StringConcat (either side being Str triggers concat)
                if matches!(op, BinOp::Add) && (self.reg_types.get(&left_reg) == Some(&IrType::Str) || self.reg_types.get(&right_reg) == Some(&IrType::Str)) {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::StringConcat {
                        dest: dest.clone(),
                        left: left_reg,
                        right: right_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                }

                // Check operand types for dispatch
                let is_float = self.reg_types.get(&left_reg) == Some(&IrType::Float);
                let is_string = self.reg_types.get(&left_reg) == Some(&IrType::Str)
                    || self.reg_types.get(&right_reg) == Some(&IrType::Str);

                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        let dest = self.fresh_reg();
                        let ir_op = match op {
                            BinOp::Add => IrBinOp::Add,
                            BinOp::Sub => IrBinOp::Sub,
                            BinOp::Mul => IrBinOp::Mul,
                            BinOp::Div => IrBinOp::Div,
                            BinOp::Mod => IrBinOp::Mod,
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
                        } else if is_string {
                            self.instructions.push(Instruction::StringCmpOp {
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
                    sans_parser::ast::UnaryOp::Not => {
                        let src_reg = self.lower_expr(operand);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Not { dest: dest.clone(), src: src_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        dest
                    }
                    sans_parser::ast::UnaryOp::Neg => {
                        let src_reg = self.lower_expr(operand);
                        let src_type = self.reg_types.get(&src_reg).cloned().unwrap_or(IrType::Int);
                        let dest = self.fresh_reg();
                        if src_type == IrType::Float {
                            let zero_reg = self.fresh_reg();
                            self.instructions.push(Instruction::FloatConst { dest: zero_reg.clone(), value: 0.0 });
                            self.reg_types.insert(zero_reg.clone(), IrType::Float);
                            self.instructions.push(Instruction::FloatBinOp {
                                dest: dest.clone(),
                                op: IrBinOp::Sub,
                                left: zero_reg,
                                right: src_reg,
                            });
                            self.reg_types.insert(dest.clone(), IrType::Float);
                        } else {
                            self.instructions.push(Instruction::Neg { dest: dest.clone(), src: src_reg });
                            self.reg_types.insert(dest.clone(), IrType::Int);
                        }
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

                // Save locals before branches so each branch gets its own scope
                let saved_locals = self.locals.clone();

                // Then branch
                self.emit_label(then_label.clone());
                for stmt in then_body {
                    self.lower_stmt(stmt);
                }
                let then_reg = self.lower_expr(then_expr);
                // Capture the actual block that contains then_reg (may differ from then_label if nested)
                let then_source_label = self.current_label.clone().unwrap_or_else(|| then_label.clone());
                self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                // Restore locals before else branch so it doesn't see then-branch variables
                self.locals = saved_locals.clone();

                // Else branch
                self.emit_label(else_label.clone());
                for stmt in else_body {
                    self.lower_stmt(stmt);
                }
                let else_reg = self.lower_expr(else_expr);
                // Capture the actual block that contains else_reg (may differ from else_label if nested)
                let else_source_label = self.current_label.clone().unwrap_or_else(|| else_label.clone());
                self.instructions.push(Instruction::Jump { target: merge_label.clone() });

                // Restore locals after both branches (branch-local vars don't leak out)
                self.locals = saved_locals;

                // Merge
                self.emit_label(merge_label.clone());
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
                    a_label: then_source_label,
                    b_val: else_reg,
                    b_label: else_source_label,
                });
                self.reg_types.insert(dest.clone(), phi_type);
                dest
            }
            Expr::Call { function, args, .. } => {
                if function == "print" || function == "p" {
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
                        IrType::Map => panic!("cannot print Map"),
                        IrType::Float => self.instructions.push(Instruction::PrintFloat { value: arg_reg }),
                        IrType::HttpResponse => panic!("cannot print HttpResponse"),
                        IrType::Result(_) => panic!("cannot print Result"),
                        IrType::HttpServer => panic!("cannot print HttpServer"),
                        IrType::HttpRequest => panic!("cannot print HttpRequest"),
                        IrType::Tuple(_) => panic!("cannot print Tuple"),
                    }
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                }

                if function == "int_to_string" || function == "str" || function == "itos" {
                    let val_reg = self.lower_expr(&args[0]);
                    // If the argument is already a string, just pass it through
                    if self.reg_types.get(&val_reg) == Some(&IrType::Str) {
                        return val_reg;
                    }
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::IntToString {
                        dest: dest.clone(),
                        value: val_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                }

                if function == "string_to_int" || function == "stoi" {
                    let str_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::StringToInt {
                        dest: dest.clone(),
                        string: str_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "file_read" || function == "read_file" || function == "fread" || function == "fr" {
                    let path_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FileRead {
                        dest: dest.clone(),
                        path: path_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                } else if function == "file_write" || function == "write_file" || function == "fwrite" || function == "fw" {
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
                } else if function == "file_append" || function == "append_file" || function == "fappend" || function == "fa" {
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
                } else if function == "file_exists" || function == "fexists" || function == "fe" {
                    let path_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FileExists {
                        dest: dest.clone(),
                        path: path_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Bool);
                    return dest;
                } else if function == "args" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Args {
                        dest: dest.clone(),
                    });
                    self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Str)));
                    return dest;
                } else if function == "json_parse" || function == "jparse" || function == "jp" {
                    let source_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonParse { dest: dest.clone(), source: source_reg });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "map" || function == "M" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::MapCreate { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::Map);
                    return dest;
                } else if function == "mget" {
                    let map_reg = self.lower_expr(&args[0]);
                    let key_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::MapGet { dest: dest.clone(), map: map_reg, key: key_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "mset" {
                    let map_reg = self.lower_expr(&args[0]);
                    let key_reg = self.lower_expr(&args[1]);
                    let val_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::MapSet { dest: dest.clone(), map: map_reg, key: key_reg, value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "mhas" {
                    let map_reg = self.lower_expr(&args[0]);
                    let key_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::MapHas { dest: dest.clone(), map: map_reg, key: key_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "json_object" || function == "jobj" || function == "jo" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonObject { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_array" || function == "jarr" || function == "ja" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonArray { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_string" || function == "jstr" || function == "js" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonString { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_int" || function == "ji" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonInt { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_bool" || function == "jb" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonBool { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_null" || function == "jn" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonNull { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::JsonValue);
                    return dest;
                } else if function == "json_stringify" || function == "jstringify" || function == "jfy" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::JsonStringify { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                } else if function == "http_listen" || function == "listen" || function == "hl" {
                    let port_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::HttpListen { dest: dest.clone(), port: port_reg });
                    self.reg_types.insert(dest.clone(), IrType::HttpServer);
                    return dest;
                } else if function == "int_to_float" || function == "itof" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::IntToFloat { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Float);
                    return dest;
                } else if function == "float_to_int" || function == "ftoi" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FloatToInt { dest: dest.clone(), value: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "float_to_string" || function == "ftos" {
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
                } else if function == "log_debug" || function == "ld" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogDebug { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "log_info" || function == "li" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogInfo { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "log_warn" || function == "lw" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogWarn { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "log_error" || function == "le" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogError { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "log_set_level" || function == "ll" {
                    let level_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::LogSetLevel { dest: dest.clone(), level: level_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "print_err" {
                    let msg_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::PrintErr { dest: dest.clone(), message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "fptr" {
                    let dest = self.fresh_reg();
                    if let Expr::StringLiteral { value: name, .. } = &args[0] {
                        self.instructions.push(Instruction::FptrNamed { dest: dest.clone(), func_name: name.clone() });
                    } else {
                        panic!("fptr() requires a string literal argument");
                    }
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "fcall" {
                    let fn_ptr_reg = self.lower_expr(&args[0]);
                    let arg_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Fcall { dest: dest.clone(), fn_ptr: fn_ptr_reg, arg: arg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "fcall2" {
                    let fn_ptr_reg = self.lower_expr(&args[0]);
                    let a1_reg = self.lower_expr(&args[1]);
                    let a2_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Fcall2 { dest: dest.clone(), fn_ptr: fn_ptr_reg, arg1: a1_reg, arg2: a2_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "fcall3" {
                    let fn_ptr_reg = self.lower_expr(&args[0]);
                    let a1_reg = self.lower_expr(&args[1]);
                    let a2_reg = self.lower_expr(&args[2]);
                    let a3_reg = self.lower_expr(&args[3]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Fcall3 { dest: dest.clone(), fn_ptr: fn_ptr_reg, arg1: a1_reg, arg2: a2_reg, arg3: a3_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "wfd" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let msg_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::WriteFd { dest: dest.clone(), fd: fd_reg, message: msg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "char_at" {
                    let str_reg = self.lower_expr(&args[0]);
                    let idx_reg = self.lower_expr(&args[1]);
                    // Desugar to: ptr_as_int = copy(str), addr = ptr_as_int + idx, load8(addr)
                    let ptr_reg = self.fresh_reg();
                    self.instructions.push(Instruction::Copy { dest: ptr_reg.clone(), src: str_reg });
                    self.reg_types.insert(ptr_reg.clone(), IrType::Int);
                    let addr_reg = self.fresh_reg();
                    self.instructions.push(Instruction::BinOp { dest: addr_reg.clone(), op: IrBinOp::Add, left: ptr_reg, right: idx_reg });
                    self.reg_types.insert(addr_reg.clone(), IrType::Int);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Load8 { dest: dest.clone(), ptr: addr_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "ptr" {
                    let arg_reg = self.lower_expr(&args[0]);
                    // Emit a Copy into a fresh register typed as Int so that
                    // arithmetic on the result dispatches to BinOp (not StringConcat)
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Copy { dest: dest.clone(), src: arg_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "arena_begin" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::ArenaBegin { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "arena_alloc" {
                    let size_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::ArenaAlloc { dest: dest.clone(), size: size_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "arena_end" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::ArenaEnd { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "alloc" {
                    let size_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Alloc { dest: dest.clone(), size: size_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "dealloc" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Dealloc { dest: dest.clone(), ptr: ptr_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "ralloc" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let size_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Ralloc { dest: dest.clone(), ptr: ptr_reg, size: size_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "mcpy" {
                    let dst_reg = self.lower_expr(&args[0]);
                    let src_reg = self.lower_expr(&args[1]);
                    let len_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Mcpy { dest: dest.clone(), dst_ptr: dst_reg, src_ptr: src_reg, len: len_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "mzero" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let len_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Mzero { dest: dest.clone(), ptr: ptr_reg, len: len_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "mcmp" {
                    let a_reg = self.lower_expr(&args[0]);
                    let b_reg = self.lower_expr(&args[1]);
                    let len_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Mcmp { dest: dest.clone(), a_ptr: a_reg, b_ptr: b_reg, len: len_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "slen" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Slen { dest: dest.clone(), ptr: ptr_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "load8" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Load8 { dest: dest.clone(), ptr: ptr_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "store8" {
                    // Accept 2 args (ptr, val) or 3 args (addrspace, ptr, val)
                    let (ptr_reg, val_reg) = if args.len() == 3 {
                        let _addrspace = self.lower_expr(&args[0]); // ignored
                        (self.lower_expr(&args[1]), self.lower_expr(&args[2]))
                    } else {
                        (self.lower_expr(&args[0]), self.lower_expr(&args[1]))
                    };
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Store8 { dest: dest.clone(), ptr: ptr_reg, val: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "load16" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Load16 { dest: dest.clone(), ptr: ptr_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "store16" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let val_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Store16 { dest: dest.clone(), ptr: ptr_reg, val: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "load32" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Load32 { dest: dest.clone(), ptr: ptr_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "store32" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let val_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Store32 { dest: dest.clone(), ptr: ptr_reg, val: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "bswap16" {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Bswap16 { dest: dest.clone(), val: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "rbind" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let addr_reg = self.lower_expr(&args[1]);
                    let len_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Rbind { dest: dest.clone(), fd: fd_reg, addr: addr_reg, len: len_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "rsetsockopt" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let level_reg = self.lower_expr(&args[1]);
                    let opt_reg = self.lower_expr(&args[2]);
                    let val_ptr_reg = self.lower_expr(&args[3]);
                    let val_len_reg = self.lower_expr(&args[4]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Rsetsockopt { dest: dest.clone(), fd: fd_reg, level: level_reg, opt: opt_reg, val_ptr: val_ptr_reg, val_len: val_len_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "load64" || function == "deref" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Load64 { dest: dest.clone(), ptr: ptr_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "store64" {
                    let ptr_reg = self.lower_expr(&args[0]);
                    let val_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Store64 { dest: dest.clone(), ptr: ptr_reg, val: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "strstr" {
                    let haystack_reg = self.lower_expr(&args[0]);
                    let needle_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Strstr { dest: dest.clone(), haystack: haystack_reg, needle: needle_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "exit" {
                    let code_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Exit { dest: dest.clone(), code: code_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "system" || function == "sys" {
                    let cmd_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::System { dest: dest.clone(), command: cmd_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "sock" {
                    let domain_reg = self.lower_expr(&args[0]);
                    let type_reg = self.lower_expr(&args[1]);
                    let proto_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Sock { dest: dest.clone(), domain: domain_reg, sock_type: type_reg, proto: proto_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "sbind" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let port_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Sbind { dest: dest.clone(), fd: fd_reg, port: port_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "slisten" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let backlog_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Slisten { dest: dest.clone(), fd: fd_reg, backlog: backlog_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "saccept" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Saccept { dest: dest.clone(), fd: fd_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "srecv" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let buf_reg = self.lower_expr(&args[1]);
                    let len_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Srecv { dest: dest.clone(), fd: fd_reg, buf: buf_reg, len: len_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "ssend" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let buf_reg = self.lower_expr(&args[1]);
                    let len_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Ssend { dest: dest.clone(), fd: fd_reg, buf: buf_reg, len: len_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "sclose" {
                    let fd_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Sclose { dest: dest.clone(), fd: fd_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "ssl_ctx" {
                    let cert = self.lower_expr(&args[0]);
                    let key = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::SslCtx { dest: dest.clone(), cert, key });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "ssl_accept" {
                    let ctx = self.lower_expr(&args[0]);
                    let fd = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::SslAccept { dest: dest.clone(), ctx, fd });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "ssl_read" {
                    let ssl = self.lower_expr(&args[0]);
                    let buf = self.lower_expr(&args[1]);
                    let len = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::SslRead { dest: dest.clone(), ssl, buf, len });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "ssl_write" {
                    let ssl = self.lower_expr(&args[0]);
                    let buf = self.lower_expr(&args[1]);
                    let len = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::SslWrite { dest: dest.clone(), ssl, buf, len });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "ssl_close" {
                    let ssl = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::SslClose { dest: dest.clone(), ssl });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "cinit" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Cinit { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "csets" {
                    let handle_reg = self.lower_expr(&args[0]);
                    let opt_reg = self.lower_expr(&args[1]);
                    let val_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Csets { dest: dest.clone(), handle: handle_reg, opt: opt_reg, val: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "cseti" {
                    let handle_reg = self.lower_expr(&args[0]);
                    let opt_reg = self.lower_expr(&args[1]);
                    let val_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Cseti { dest: dest.clone(), handle: handle_reg, opt: opt_reg, val: val_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "cperf" {
                    let handle_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Cperf { dest: dest.clone(), handle: handle_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "cclean" {
                    let handle_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Cclean { dest: dest.clone(), handle: handle_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "cinfo" {
                    let handle_reg = self.lower_expr(&args[0]);
                    let info_reg = self.lower_expr(&args[1]);
                    let buf_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::Cinfo { dest: dest.clone(), handle: handle_reg, info: info_reg, buf: buf_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "curl_slist_append" {
                    let slist_reg = self.lower_expr(&args[0]);
                    let str_reg = self.lower_expr(&args[1]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::CurlSlistAppend { dest: dest.clone(), slist: slist_reg, str_ptr: str_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "curl_slist_free" {
                    let slist_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::CurlSlistFree { dest: dest.clone(), slist: slist_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "get_log_level" {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::GetLogLevel { dest: dest.clone() });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "set_log_level" {
                    let lvl_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::SetLogLevel { dest: dest.clone(), level: lvl_reg });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                } else if function == "http_get" || function == "hget" || function == "hg" {
                    let url_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::HttpGet { dest: dest.clone(), url: url_reg });
                    self.reg_types.insert(dest.clone(), IrType::HttpResponse);
                    return dest;
                } else if function == "http_post" || function == "hpost" || function == "hp" {
                    let url_reg = self.lower_expr(&args[0]);
                    let body_reg = self.lower_expr(&args[1]);
                    let ct_reg = self.lower_expr(&args[2]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::HttpPost { dest: dest.clone(), url: url_reg, body: body_reg, content_type: ct_reg });
                    self.reg_types.insert(dest.clone(), IrType::HttpResponse);
                    return dest;
                }

                // Check if the function name is a local variable holding a closure or fn ref
                if let Some(local) = self.locals.get(function).cloned() {
                    let local_reg = match &local {
                        LocalVar::Value(r) => r.clone(),
                        LocalVar::Ptr(r) => r.clone(),
                    };
                    // Check if this local holds a capturing closure
                    if let Some((lifted_fn_name, capture_names)) = self.closure_info.get(&local_reg).cloned() {
                        // Emit direct Call to lifted function with captures prepended
                        let mut all_args: Vec<Reg> = Vec::new();
                        for cap_name in &capture_names {
                            let cap_reg = match self.locals.get(cap_name).cloned() {
                                Some(LocalVar::Value(r)) => r,
                                Some(LocalVar::Ptr(ptr)) => {
                                    let load_dest = self.fresh_reg();
                                    self.instructions.push(Instruction::Load { dest: load_dest.clone(), ptr });
                                    load_dest
                                }
                                None => panic!("capture not found: {}", cap_name),
                            };
                            all_args.push(cap_reg);
                        }
                        let explicit_args: Vec<Reg> = args.iter().map(|a| self.lower_expr(a)).collect();
                        all_args.extend(explicit_args);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Call {
                            dest: dest.clone(),
                            function: lifted_fn_name,
                            args: all_args,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    } else {
                        // Non-capturing fn ref — use Fcall for single arg, or Call via FnRef
                        let fn_ptr_reg = match local {
                            LocalVar::Value(r) => r,
                            LocalVar::Ptr(ptr) => {
                                let load_dest = self.fresh_reg();
                                self.instructions.push(Instruction::Load { dest: load_dest.clone(), ptr });
                                load_dest
                            }
                        };
                        if args.len() == 1 {
                            let arg_reg = self.lower_expr(&args[0]);
                            let dest = self.fresh_reg();
                            self.instructions.push(Instruction::Fcall { dest: dest.clone(), fn_ptr: fn_ptr_reg, arg: arg_reg });
                            self.reg_types.insert(dest.clone(), IrType::Int);
                            return dest;
                        }
                        // For multi-arg, fall through to direct Call (works if it's a known fn name)
                    }
                }

                let arg_regs: Vec<Reg> = args.iter().map(|a| self.lower_expr(a)).collect();
                let dest = self.fresh_reg();
                // Mangle calls: intra-module (local) and cross-module (imported) functions
                let call_name = if let Some(ref mod_name) = self.current_module {
                    if self.local_fn_names.contains(function) {
                        format!("{}__{}", mod_name, function)
                    } else if let Some(mangled) = self.imported_fn_names.get(function) {
                        mangled.clone()
                    } else {
                        function.clone()
                    }
                } else if let Some(mangled) = self.imported_fn_names.get(function) {
                    mangled.clone()
                } else {
                    function.clone()
                };
                self.instructions.push(Instruction::Call {
                    dest: dest.clone(),
                    function: call_name,
                    args: arg_regs,
                });
                // Use tracked return type if available (for Result, struct, enum, etc.)
                let ret_type = self.local_fn_ret_types.get(function).cloned()
                    .or_else(|| {
                        // Check module_fn_ret_types for cross-module calls
                        for ((m, f), t) in self.module_fn_ret_types.iter() {
                            if f == function {
                                return Some(t.clone());
                            }
                        }
                        None
                    })
                    .unwrap_or(IrType::Int);
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
            Expr::FieldAccess { object, field, span } => {
                let obj_reg = self.lower_expr(object);
                match self.reg_types.get(&obj_reg) {
                    Some(IrType::Tuple(elements)) => {
                        let num_fields = elements.len();
                        let field_index: usize = field.parse().expect("tuple access must be numeric");
                        let elem_type = elements.get(field_index).cloned().unwrap_or(IrType::Int);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::FieldLoad {
                            dest: dest.clone(),
                            ptr: obj_reg,
                            field_index,
                            num_fields,
                        });
                        self.reg_types.insert(dest.clone(), elem_type);
                        dest
                    }
                    Some(IrType::Struct(name)) => {
                        let struct_name = name.clone();
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
                        // Look up the field's actual IrType from struct_field_types
                        let field_ir_type = self.struct_field_types.get(&struct_name)
                            .and_then(|fields| fields.iter().find(|(n, _)| n == field))
                            .map(|(_, t)| t.clone())
                            .unwrap_or(IrType::Int);
                        self.reg_types.insert(dest.clone(), field_ir_type);
                        dest
                    }
                    _ => {
                        // Non-struct: treat as no-arg method call
                        let synthetic = Expr::MethodCall {
                            object: object.clone(),
                            method: field.clone(),
                            args: vec![],
                            span: span.clone(),
                        };
                        self.lower_expr(&synthetic)
                    }
                }
            }
            Expr::EnumVariant { enum_name, variant_name, args, .. } => {
                let variants = self.enum_defs.get(enum_name)
                    .expect("unknown enum in variant construction").clone();
                let (_, tag, _variant_data_fields) = variants.iter()
                    .find(|(n, _, _)| n == variant_name)
                    .expect("unknown variant");
                let tag = *tag as i64;
                // Use max data fields across ALL variants so all are same size (needed for Phi)
                let num_data_fields = variants.iter().map(|(_, _, n)| *n).max().unwrap_or(0);

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
                    (Some(IrType::Array(ref inner)), "map") => {
                        let fn_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayMap { dest: dest.clone(), array: obj_reg, fn_ptr: fn_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(inner.clone())); // same inner type for now
                        return dest;
                    }
                    (Some(IrType::Array(ref inner)), "filter") => {
                        let fn_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayFilter { dest: dest.clone(), array: obj_reg, fn_ptr: fn_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(inner.clone()));
                        return dest;
                    }
                    (Some(IrType::Array(_)), "any") => {
                        let fn_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayAny { dest: dest.clone(), array: obj_reg, fn_ptr: fn_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Array(ref inner)), "find") => {
                        let fn_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayFind { dest: dest.clone(), array: obj_reg, fn_ptr: fn_reg });
                        self.reg_types.insert(dest.clone(), inner.as_ref().clone());
                        return dest;
                    }
                    (Some(IrType::Array(ref inner)), "enumerate") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayEnumerate { dest: dest.clone(), array: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Tuple(vec![IrType::Int, inner.as_ref().clone()]))));
                        return dest;
                    }
                    (Some(IrType::Array(_)), "zip") => {
                        let other_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayZip { dest: dest.clone(), array: obj_reg, other: other_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Tuple(vec![IrType::Int, IrType::Int]))));
                        return dest;
                    }
                    (Some(IrType::Array(ref inner)), "pop") => {
                        let elem_type = *inner.clone();
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayPop { dest: dest.clone(), array: obj_reg });
                        self.reg_types.insert(dest.clone(), elem_type);
                        return dest;
                    }
                    (Some(IrType::Array(_)), "contains") => {
                        let val_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayContains { dest: dest.clone(), array: obj_reg, value: val_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Array(ref inner)), "remove") => {
                        let elem_type = *inner.clone();
                        let idx_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayRemove { dest: dest.clone(), array: obj_reg, index: idx_reg });
                        self.reg_types.insert(dest.clone(), elem_type);
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
                    (Some(IrType::Str), "trim") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringTrim { dest: dest.clone(), string: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::Str), "starts_with") | (Some(IrType::Str), "sw") => {
                        let prefix_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringStartsWith { dest: dest.clone(), string: obj_reg, prefix: prefix_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Str), "ends_with") | (Some(IrType::Str), "ew") => {
                        let suffix_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringEndsWith { dest: dest.clone(), string: obj_reg, suffix: suffix_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Str), "contains") => {
                        let needle_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringContains { dest: dest.clone(), string: obj_reg, needle: needle_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Str), "split") => {
                        let delim_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringSplit { dest: dest.clone(), string: obj_reg, delimiter: delim_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Str)));
                        return dest;
                    }
                    (Some(IrType::Str), "replace") => {
                        let old_reg = self.lower_expr(&args[0]);
                        let new_reg = self.lower_expr(&args[1]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringReplace { dest: dest.clone(), string: obj_reg, old: old_reg, new_str: new_reg });
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
                    (Some(IrType::Map), "get") => {
                        let key_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapGet { dest: dest.clone(), map: obj_reg, key: key_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Map), "set") => {
                        let key_reg = self.lower_expr(&args[0]);
                        let val_reg = self.lower_expr(&args[1]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapSet { dest: dest.clone(), map: obj_reg, key: key_reg, value: val_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Map), "has") => {
                        let key_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapHas { dest: dest.clone(), map: obj_reg, key: key_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Map), "len") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapLen { dest: dest.clone(), map: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Map), "keys") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapKeys { dest: dest.clone(), map: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Str)));
                        return dest;
                    }
                    (Some(IrType::Map), "vals") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapVals { dest: dest.clone(), map: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Int)));
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
                    (Some(IrType::HttpServer), "accept") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::HttpAccept { dest: dest.clone(), server: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::HttpRequest);
                        return dest;
                    }
                    (Some(IrType::HttpRequest), "path") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::HttpRequestPath { dest: dest.clone(), request: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::HttpRequest), "method") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::HttpRequestMethod { dest: dest.clone(), request: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::HttpRequest), "body") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::HttpRequestBody { dest: dest.clone(), request: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::HttpRequest), "respond") => {
                        let status_reg = self.lower_expr(&args[0]);
                        let body_reg = self.lower_expr(&args[1]);
                        let dest = self.fresh_reg();
                        if args.len() == 3 {
                            let ct_reg = self.lower_expr(&args[2]);
                            self.instructions.push(Instruction::HttpRespondWithContentType {
                                dest: dest.clone(), request: obj_reg, status: status_reg, body: body_reg, content_type: ct_reg
                            });
                        } else {
                            self.instructions.push(Instruction::HttpRespond { dest: dest.clone(), request: obj_reg, status: status_reg, body: body_reg });
                        }
                        self.reg_types.insert(dest.clone(), IrType::Int);
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
                    // Relaxed: when type is Int (pointer), infer the correct dispatch
                    // based on method name for self-hosted compiler compatibility
                    (Some(IrType::Int), _) if method == "get" => {
                        let key_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        // Default to ArrayGet — most .get() calls on Int are array access
                        self.instructions.push(Instruction::ArrayGet { dest: dest.clone(), array: obj_reg, index: key_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "set" => {
                        let key_reg = self.lower_expr(&args[0]);
                        let val_reg = self.lower_expr(&args[1]);
                        let dest = self.fresh_reg();
                        // Default to ArraySet — most .set() calls on Int are array access
                        self.instructions.push(Instruction::ArraySet { array: obj_reg, index: key_reg, value: val_reg });
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "has" => {
                        let key_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapHas { dest: dest.clone(), map: obj_reg, key: key_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "push" => {
                        let val_reg = self.lower_expr(&args[0]);
                        self.instructions.push(Instruction::ArrayPush { array: obj_reg, value: val_reg });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "pop" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayPop { dest: dest.clone(), array: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "len" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayLen { dest: dest.clone(), array: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "keys" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapKeys { dest: dest.clone(), map: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Str)));
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "vals" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MapVals { dest: dest.clone(), map: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Int)));
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "remove" => {
                        let idx_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayRemove { dest: dest.clone(), array: obj_reg, index: idx_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "sw" => {
                        let prefix_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringStartsWith { dest: dest.clone(), string: obj_reg, prefix: prefix_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "ew" => {
                        let suffix_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringEndsWith { dest: dest.clone(), string: obj_reg, suffix: suffix_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "contains" => {
                        let arg_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        let arg_type = self.reg_types.get(&arg_reg).cloned().unwrap_or(IrType::Int);
                        if arg_type == IrType::Str {
                            self.instructions.push(Instruction::StringContains { dest: dest.clone(), string: obj_reg, needle: arg_reg });
                        } else {
                            self.instructions.push(Instruction::ArrayContains { dest: dest.clone(), array: obj_reg, value: arg_reg });
                        }
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "map" => {
                        let fn_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayMap { dest: dest.clone(), array: obj_reg, fn_ptr: fn_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Int)));
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "filter" => {
                        let fn_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayFilter { dest: dest.clone(), array: obj_reg, fn_ptr: fn_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Int)));
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "any" => {
                        let fn_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayAny { dest: dest.clone(), array: obj_reg, fn_ptr: fn_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "find" => {
                        let fn_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayFind { dest: dest.clone(), array: obj_reg, fn_ptr: fn_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "enumerate" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayEnumerate { dest: dest.clone(), array: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Int)));
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "zip" => {
                        let other_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayZip { dest: dest.clone(), array: obj_reg, other: other_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Int)));
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "starts_with" || method == "sw" => {
                        let prefix_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringStartsWith { dest: dest.clone(), string: obj_reg, prefix: prefix_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "ends_with" || method == "ew" => {
                        let suffix_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringEndsWith { dest: dest.clone(), string: obj_reg, suffix: suffix_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "add" => {
                        let other_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringConcat { dest: dest.clone(), left: obj_reg, right: other_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "unwrap" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultUnwrap { dest: dest.clone(), result: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "is_ok" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultIsOk { dest: dest.clone(), result: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "is_err" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultIsErr { dest: dest.clone(), result: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Bool);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "error" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultError { dest: dest.clone(), result: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "unwrap_or" => {
                        let default_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ResultUnwrapOr { dest: dest.clone(), result: obj_reg, default: default_reg });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "stringify" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::JsonStringify { dest: dest.clone(), value: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "split" => {
                        let delim_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringSplit { dest: dest.clone(), string: obj_reg, delimiter: delim_reg });
                        self.reg_types.insert(dest.clone(), IrType::Array(Box::new(IrType::Str)));
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "replace" => {
                        let old_reg = self.lower_expr(&args[0]);
                        let new_reg = self.lower_expr(&args[1]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringReplace { dest: dest.clone(), string: obj_reg, old: old_reg, new_str: new_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "trim" => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringTrim { dest: dest.clone(), string: obj_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    (Some(IrType::Int), _) if method == "substring" => {
                        let start_reg = self.lower_expr(&args[0]);
                        let end_reg = self.lower_expr(&args[1]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringSubstring { dest: dest.clone(), string: obj_reg, start: start_reg, end: end_reg });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
                    _ => {} // fall through to struct/enum handling
                }

                // Existing struct/enum method call handling
                let type_name = match self.reg_types.get(&obj_reg) {
                    Some(IrType::Struct(name)) => name.clone(),
                    Some(IrType::Enum(name)) => name.clone(),
                    Some(IrType::Int) => {
                        // Relaxed: treat Int as a generic call (method as function with obj as first arg)
                        let mangled = format!("{}", method);
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
                        return dest;
                    }
                    _ => panic!("method call on non-struct/enum: {:?} method: {}", self.reg_types.get(&obj_reg), method),
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
                    let sans_parser::ast::Pattern::EnumVariant {
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

                    self.emit_label(arm_label);

                    // Bind data fields with correct types from enum definition
                    let field_types = self.enum_field_types.get(&(enum_name.clone(), variant_name.clone())).cloned();
                    for (i, binding_name) in bindings.iter().enumerate() {
                        let data_reg = self.fresh_reg();
                        self.instructions.push(Instruction::EnumData {
                            dest: data_reg.clone(),
                            ptr: scrutinee_reg.clone(),
                            field_index: i,
                        });
                        let ir_type = field_types.as_ref()
                            .and_then(|types| types.get(i).cloned())
                            .unwrap_or(IrType::Int);
                        self.reg_types.insert(data_reg.clone(), ir_type);
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
                        self.emit_label(next_label);
                    }
                }

                self.emit_label(merge_label);
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
                    "Int" | "I" => IrType::Int,
                    "Bool" | "B" => IrType::Bool,
                    "String" | "S" => IrType::Str,
                    "Float" | "F" => IrType::Float,
                    "Map" | "M" => IrType::Map,
                    other if self.enum_defs.contains_key(other) => IrType::Enum(other.to_string()),
                    other => IrType::Struct(other.to_string()),
                };
                self.reg_types.insert(dest.clone(), IrType::Array(Box::new(inner_ir_type)));
                dest
            }
            Expr::ArrayLiteral { elements, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::ArrayCreate {
                    dest: dest.clone(),
                });
                let mut elem_type = IrType::Int;
                for (i, elem) in elements.iter().enumerate() {
                    let val_reg = self.lower_expr(elem);
                    if i == 0 {
                        elem_type = self.reg_types.get(&val_reg).cloned().unwrap_or(IrType::Int);
                    }
                    self.instructions.push(Instruction::ArrayPush {
                        array: dest.clone(),
                        value: val_reg,
                    });
                }
                self.reg_types.insert(dest.clone(), IrType::Array(Box::new(elem_type)));
                dest
            }

            Expr::TupleLiteral { elements, .. } => {
                let num_fields = elements.len();
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::StructAlloc { dest: dest.clone(), num_fields });

                let mut elem_types = Vec::new();
                for (i, elem_expr) in elements.iter().enumerate() {
                    let val_reg = self.lower_expr(elem_expr);
                    let ty = self.reg_types.get(&val_reg).cloned().unwrap_or(IrType::Int);
                    elem_types.push(ty);
                    self.instructions.push(Instruction::FieldStore {
                        ptr: dest.clone(),
                        field_index: i,
                        value: val_reg,
                    });
                }
                self.reg_types.insert(dest.clone(), IrType::Tuple(elem_types));
                dest
            }

            Expr::Lambda { params, return_type: _, body, .. } => {
                // 1. Find captured variables
                let captures = self.find_captures(body, params);

                // 2. Generate unique lambda name
                let lambda_name = format!("__lambda_{}", self.lambda_counter);
                self.lambda_counter += 1;

                // 3. Build the lifted function with a fresh IrBuilder
                let mut lifted_params: Vec<Reg> = Vec::new();
                let mut lifted_builder = IrBuilder::new(
                    self.struct_defs.clone(),
                    self.enum_defs.clone(),
                    self.module_names.clone(),
                    self.module_fn_ret_types.clone(),
                    self.local_fn_ret_types.clone(),
                    self.global_names.clone(),
                    self.struct_field_types.clone(),
                    self.enum_field_types.clone(),
                );
                // Share lambda counter with nested builder to avoid name collisions
                lifted_builder.lambda_counter = self.lambda_counter;

                // Add capture params first
                for cap_name in &captures {
                    let reg = lifted_builder.fresh_reg();
                    lifted_params.push(reg.clone());
                    // Copy type info from the enclosing scope
                    if let Some(local) = self.locals.get(cap_name) {
                        let src_reg = match local {
                            LocalVar::Value(r) => r.clone(),
                            LocalVar::Ptr(r) => r.clone(),
                        };
                        if let Some(ty) = self.reg_types.get(&src_reg).cloned() {
                            lifted_builder.reg_types.insert(reg.clone(), ty);
                        }
                    }
                    lifted_builder.locals.insert(cap_name.clone(), LocalVar::Value(reg));
                }

                // Add actual lambda params
                for param in params {
                    let reg = lifted_builder.fresh_reg();
                    lifted_params.push(reg.clone());
                    // Set type from param type name
                    match param.type_name.name.as_str() {
                        "Int" | "I" => { lifted_builder.reg_types.insert(reg.clone(), IrType::Int); }
                        "Float" | "F" => { lifted_builder.reg_types.insert(reg.clone(), IrType::Float); }
                        "Bool" | "B" => { lifted_builder.reg_types.insert(reg.clone(), IrType::Bool); }
                        "String" | "S" => { lifted_builder.reg_types.insert(reg.clone(), IrType::Str); }
                        _ => { lifted_builder.reg_types.insert(reg.clone(), IrType::Int); }
                    }
                    lifted_builder.locals.insert(param.name.clone(), LocalVar::Value(reg));
                }

                // Lower body statements
                for (i, stmt) in body.iter().enumerate() {
                    let is_last = i == body.len() - 1;
                    if is_last {
                        match stmt {
                            Stmt::Expr(expr) => {
                                let reg = lifted_builder.lower_expr(expr);
                                lifted_builder.instructions.push(Instruction::Ret { value: reg });
                            }
                            Stmt::Return { value, .. } => {
                                let reg = lifted_builder.lower_expr(value);
                                lifted_builder.instructions.push(Instruction::Ret { value: reg });
                            }
                            other => {
                                lifted_builder.lower_stmt(other);
                            }
                        }
                    } else {
                        lifted_builder.lower_stmt(stmt);
                    }
                }

                // Create IrFunction for the lifted lambda
                let param_struct_sizes: Vec<usize> = lifted_params.iter().map(|_| 0usize).collect();
                let lifted_fn = IrFunction {
                    name: lambda_name.clone(),
                    params: lifted_params,
                    param_struct_sizes,
                    body: lifted_builder.instructions,
                };
                self.lifted_fns.push(lifted_fn);
                // Collect any nested lambdas
                self.lifted_fns.extend(lifted_builder.lifted_fns);
                // Sync lambda counter back from nested builder to avoid name collisions
                self.lambda_counter = lifted_builder.lambda_counter;

                // 4. At the call site, emit either FnRef (no captures) or closure struct
                if captures.is_empty() {
                    // Non-capturing: just emit FnRef
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::FnRef { dest: dest.clone(), name: lambda_name });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    dest
                } else {
                    // Capturing: create a closure struct {fn_ptr, n_captures, cap0, cap1, ...}
                    let num_fields = 2 + captures.len();
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::StructAlloc { dest: dest.clone(), num_fields });

                    // Store fn_ptr at field 0
                    let fn_ref_reg = self.fresh_reg();
                    self.instructions.push(Instruction::FnRef { dest: fn_ref_reg.clone(), name: lambda_name.clone() });
                    self.instructions.push(Instruction::FieldStore { ptr: dest.clone(), field_index: 0, value: fn_ref_reg });

                    // Store num_captures at field 1
                    let ncap_reg = self.fresh_reg();
                    self.instructions.push(Instruction::Const { dest: ncap_reg.clone(), value: captures.len() as i64 });
                    self.instructions.push(Instruction::FieldStore { ptr: dest.clone(), field_index: 1, value: ncap_reg });

                    // Store each captured value
                    for (i, cap_name) in captures.iter().enumerate() {
                        let cap_reg = match self.locals.get(cap_name).cloned() {
                            Some(LocalVar::Value(r)) => r,
                            Some(LocalVar::Ptr(ptr)) => {
                                let load_dest = self.fresh_reg();
                                self.instructions.push(Instruction::Load { dest: load_dest.clone(), ptr });
                                load_dest
                            }
                            None => panic!("capture not found: {}", cap_name),
                        };
                        self.instructions.push(Instruction::FieldStore { ptr: dest.clone(), field_index: 2 + i, value: cap_reg });
                    }

                    // Store closure info so call sites can use direct Call with captures prepended
                    self.closure_info.insert(dest.clone(), (lambda_name, captures.iter().cloned().collect()));

                    self.reg_types.insert(dest.clone(), IrType::Int);
                    dest
                }
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
                if let Some(local) = self.locals.get(name).cloned() {
                    if let LocalVar::Ptr(ptr) = local {
                        self.instructions.push(Instruction::Store { ptr, value: val_reg });
                    } else {
                        // Relaxed: allow reassignment of immutable variables
                        // Just update the value binding (no alloca promotion)
                        self.locals.insert(name.clone(), LocalVar::Value(val_reg));
                    }
                } else if self.global_names.contains_key(name) {
                    // Global variable — emit GlobalStore
                    self.instructions.push(Instruction::GlobalStore { name: name.clone(), value: val_reg });
                } else {
                    // Bare assignment creating a new immutable binding
                    self.locals.insert(name.clone(), LocalVar::Value(val_reg));
                }
            }
            Stmt::While { condition, body, .. } => {
                let cond_label = self.fresh_label("while_cond");
                let body_label = self.fresh_label("while_body");
                let end_label = self.fresh_label("while_end");

                self.loop_stack.push((cond_label.clone(), end_label.clone()));

                self.instructions.push(Instruction::Jump { target: cond_label.clone() });

                self.emit_label(cond_label.clone());
                let cond_reg = self.lower_expr(condition);
                self.instructions.push(Instruction::Branch {
                    cond: cond_reg,
                    then_label: body_label.clone(),
                    else_label: end_label.clone(),
                });

                self.emit_label(body_label.clone());
                for s in body {
                    self.lower_stmt(s);
                }
                self.instructions.push(Instruction::Jump { target: cond_label.clone() });

                self.emit_label(end_label.clone());
                self.loop_stack.pop();
            }
            Stmt::Break { .. } => {
                if let Some((_, end_label)) = self.loop_stack.last() {
                    self.instructions.push(Instruction::Jump { target: end_label.clone() });
                }
            }
            Stmt::Continue { .. } => {
                if let Some((cond_label, _)) = self.loop_stack.last() {
                    self.instructions.push(Instruction::Jump { target: cond_label.clone() });
                }
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

                // Save locals so branch-local variables don't leak out
                let saved_locals = self.locals.clone();
                self.emit_label(then_label);
                for s in body {
                    self.lower_stmt(s);
                }
                self.instructions.push(Instruction::Jump { target: end_label.clone() });
                self.locals = saved_locals;

                self.emit_label(end_label);
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
                let inc_label = self.fresh_label("forin_inc");
                let end_label = self.fresh_label("forin_end");

                // For break/continue: continue jumps to inc (increment then re-check),
                // break jumps to end
                self.loop_stack.push((inc_label.clone(), end_label.clone()));

                self.instructions.push(Instruction::Jump { target: cond_label.clone() });

                self.emit_label(cond_label.clone());
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

                self.emit_label(body_label.clone());
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

                // Jump to increment (so body block has a terminator before inc label)
                self.instructions.push(Instruction::Jump { target: inc_label.clone() });
                // Increment label (continue target)
                self.emit_label(inc_label);

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
                self.emit_label(end_label);
                self.loop_stack.pop();
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
        sans_parser::parse(src).expect("parse failed")
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
        let prog = sans_parser::parse(
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
        let prog = sans_parser::parse(
            "fn main() Int { let (tx, rx) = channel<Int>(10) tx.send(1) rx.recv() }"
        ).unwrap();
        let module = lower(&prog, None, &HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ChannelCreateBounded { .. })),
            "expected ChannelCreateBounded instruction");
    }

    #[test]
    fn lower_unbounded_channel_unchanged() {
        let prog = sans_parser::parse(
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
        let prog = sans_parser::parse(
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
        let prog = sans_parser::parse(
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
        let prog = sans_parser::parse(
            r#"fn main() Int { let s = "a" + "b" 0 }"#
        ).unwrap();
        let module = lower(&prog, None, &HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::StringConcat { .. })),
            "expected StringConcat instruction");
    }

    #[test]
    fn lower_int_to_string_and_string_to_int() {
        let prog = sans_parser::parse(
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
        let program = sans_parser::parse("fn main() Int { let v = json_parse(\"{}\") \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::JsonParse { .. })),
            "expected JsonParse instruction");
    }

    #[test]
    fn lower_json_object() {
        let program = sans_parser::parse("fn main() Int { let v = json_object() \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::JsonObject { .. })),
            "expected JsonObject instruction");
    }

    #[test]
    fn lower_json_stringify() {
        let program = sans_parser::parse("fn main() Int { let v = json_object() \n let s = json_stringify(v) \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::JsonStringify { .. })),
            "expected JsonStringify instruction");
    }

    #[test]
    fn lower_json_get_method() {
        let program = sans_parser::parse("fn main() Int { let v = json_parse(\"{}\") \n let inner = v.get(\"key\") \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::JsonGet { .. })),
            "expected JsonGet instruction");
    }

    #[test]
    fn lower_log_info() {
        let program = sans_parser::parse("fn main() Int { log_info(\"hello\") }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::LogInfo { .. })),
            "expected LogInfo instruction");
    }

    #[test]
    fn lower_log_set_level() {
        let program = sans_parser::parse("fn main() Int { log_set_level(2) }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::LogSetLevel { .. })),
            "expected LogSetLevel instruction");
    }

    #[test]
    fn lower_http_get() {
        let program = sans_parser::parse("fn main() Int { let r = http_get(\"http://example.com\") \n r.status() }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::HttpGet { .. })),
            "expected HttpGet instruction");
    }

    #[test]
    fn lower_http_post() {
        let program = sans_parser::parse("fn main() Int { let r = http_post(\"http://example.com\", \"body\", \"text/plain\") \n r.status() }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::HttpPost { .. })),
            "expected HttpPost instruction");
    }

    #[test]
    fn lower_http_body_method() {
        let program = sans_parser::parse("fn main() Int { let r = http_get(\"http://example.com\") \n let b = r.body() \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::HttpBody { .. })),
            "expected HttpBody instruction");
    }

    #[test]
    fn lower_result_ok() {
        let program = sans_parser::parse("fn main() Int { let r = ok(42) \n r.unwrap() }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ResultOk { .. })),
            "expected ResultOk instruction");
    }

    #[test]
    fn lower_result_err() {
        let program = sans_parser::parse("fn main() Int { let r = err(\"bad\") \n 0 }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ResultErr { .. })),
            "expected ResultErr instruction");
    }

    #[test]
    fn lower_result_unwrap() {
        let program = sans_parser::parse("fn main() Int { let r = ok(42) \n r.unwrap() }").unwrap();
        let module = lower(&program, None, &std::collections::HashMap::new());
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ResultUnwrap { .. })),
            "expected ResultUnwrap instruction");
    }

    #[test]
    fn lower_tuple_literal() {
        let src = "main() I { t = (10 20)\n t.0 + t.1 }";
        let program = parse(src);
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = lower(&program, None, &std::collections::HashMap::new());
        let main_fn = ir.functions.iter().find(|f| f.name == "main").unwrap();
        assert!(main_fn.body.iter().any(|i| matches!(i, Instruction::StructAlloc { num_fields: 2, .. })));
    }

    #[test]
    fn lower_lambda_basic() {
        let src = "main() I { f = |x:I| I { x + 10 }\n 0 }";
        let program = parse(src);
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = lower(&program, None, &std::collections::HashMap::new());
        // Should have a lifted __lambda_0 function
        assert!(ir.functions.iter().any(|f| f.name.starts_with("__lambda")),
            "expected a lifted lambda function, got: {:?}", ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>());
    }

    #[test]
    fn lower_lambda_with_capture() {
        let src = "main() I { offset = 10\n f = |x:I| I { x + offset }\n 0 }";
        let program = parse(src);
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = lower(&program, None, &std::collections::HashMap::new());
        let lambda_fn = ir.functions.iter().find(|f| f.name.starts_with("__lambda")).unwrap();
        // Capturing lambda should have 2 params: capture (offset) + explicit (x)
        assert_eq!(lambda_fn.params.len(), 2,
            "expected 2 params (capture + explicit), got {}: {:?}", lambda_fn.params.len(), lambda_fn.params);
    }

    #[test]
    fn lower_lambda_no_capture_uses_fnref() {
        let src = "main() I { f = |x:I| I { x + 10 }\n 0 }";
        let program = parse(src);
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = lower(&program, None, &std::collections::HashMap::new());
        let main_fn = ir.functions.iter().find(|f| f.name == "main").unwrap();
        // Non-capturing lambda should emit FnRef
        assert!(main_fn.body.iter().any(|i| matches!(i, Instruction::FnRef { .. })),
            "expected FnRef instruction for non-capturing lambda");
    }

    #[test]
    fn lower_lambda_capture_uses_closure_struct() {
        let src = "main() I { offset = 10\n f = |x:I| I { x + offset }\n 0 }";
        let program = parse(src);
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = lower(&program, None, &std::collections::HashMap::new());
        let main_fn = ir.functions.iter().find(|f| f.name == "main").unwrap();
        // Capturing lambda should emit StructAlloc for closure struct
        assert!(main_fn.body.iter().any(|i| matches!(i, Instruction::StructAlloc { num_fields: 3, .. })),
            "expected StructAlloc with 3 fields (fn_ptr, n_captures, cap0) for closure struct");
    }
}
