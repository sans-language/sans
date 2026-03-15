pub mod types;

use std::collections::HashMap;
use sans_parser::ast::{Expr, Program, Stmt};
use types::Type;

/// Information about a generic function, used for type inference at call sites.
pub struct GenericFnInfo {
    pub type_params: Vec<(String, Option<String>)>, // (name, optional trait bound)
    pub param_types: Vec<String>,                     // raw type name strings (may be type params)
    pub return_type_name: String,                     // raw return type name (may be type param)
}

/// Exported items from a module, available for cross-module resolution.
#[derive(Debug)]
pub struct ModuleExports {
    pub functions: HashMap<String, FunctionSignature>,
    pub structs: HashMap<String, Vec<(String, Type)>>,
    pub enums: HashMap<String, Vec<(String, Vec<Type>)>>,
}

/// Signature of an exported function.
#[derive(Debug)]
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Type,
}

/// An error produced during type checking.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub message: String,
}

impl TypeError {
    fn new(message: impl Into<String>) -> Self {
        TypeError { message: message.into() }
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "type error: {}", self.message)
    }
}

/// Check if an actual type is compatible with an expected type.
/// ResultErr is compatible with any Result<T>.
fn types_compatible(actual: &Type, expected: &Type) -> bool {
    actual == expected
        || (matches!(actual, Type::ResultErr) && matches!(expected, Type::Result { .. }))
        // Allow Fn types to be passed as Int (function pointers are i64)
        || (*expected == Type::Int && matches!(actual, Type::Fn { .. }))
        // Allow String where Int is expected (strings are pointers = i64)
        || (*expected == Type::Int && *actual == Type::String)
        || (*expected == Type::String && *actual == Type::Int)
        // Allow Map/Array where Int is expected (heap pointers = i64)
        || (*expected == Type::Int && *actual == Type::Map)
        || (*expected == Type::Map && *actual == Type::Int)
        || (*expected == Type::Int && matches!(actual, Type::Array { .. }))
}

/// Resolve an AST type name string to a `Type`.
fn resolve_type(
    name: &str,
    structs: &HashMap<String, Vec<(String, Type)>>,
    enums: &HashMap<String, Vec<(String, Vec<Type>)>>,
    module_exports: &HashMap<String, ModuleExports>,
) -> Result<Type, TypeError> {
    if name.starts_with('(') && name.ends_with(')') {
        let inner = &name[1..name.len()-1];
        let parts: Vec<&str> = inner.split_whitespace().collect();
        let element_types: Vec<Type> = parts.iter()
            .map(|p| resolve_type(p, structs, enums, module_exports))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Type::Tuple { elements: element_types });
    }

    match name {
        "Int" | "I" => Ok(Type::Int),
        "Float" | "F" => Ok(Type::Float),
        "Bool" | "B" => Ok(Type::Bool),
        "String" | "S" => Ok(Type::String),
        "HttpRequest" | "HR" => Ok(Type::HttpRequest),
        "HttpServer" | "HS" => Ok(Type::HttpServer),
        "Map" | "M" => Ok(Type::Map),
        _ if name.starts_with("Array<") && name.ends_with('>') => {
            let inner_str = &name[6..name.len()-1];
            let inner = resolve_type(inner_str, structs, enums, module_exports)?;
            Ok(Type::Array { inner: Box::new(inner) })
        }
        _ if name.starts_with("R<") && name.ends_with('>') => {
            let inner_str = &name[2..name.len()-1];
            let inner = resolve_type(inner_str, structs, enums, module_exports)?;
            Ok(Type::Result { inner: Box::new(inner) })
        }
        _ if name.starts_with("Result<") && name.ends_with('>') => {
            let inner_str = &name[7..name.len()-1];
            let inner = resolve_type(inner_str, structs, enums, module_exports)?;
            Ok(Type::Result { inner: Box::new(inner) })
        }
        other => {
            if let Some(fields) = structs.get(other) {
                Ok(Type::Struct { name: other.to_string(), fields: fields.clone() })
            } else if let Some(variants) = enums.get(other) {
                Ok(Type::Enum { name: other.to_string(), variants: variants.clone() })
            } else {
                // Search module exports for the type.
                // Note: if two modules export the same type name, the first found wins.
                // Duplicate last-segment detection is deferred (see spec).
                for (_mod_name, exports) in module_exports {
                    if let Some(fields) = exports.structs.get(other) {
                        return Ok(Type::Struct { name: other.to_string(), fields: fields.clone() });
                    }
                    if let Some(variants) = exports.enums.get(other) {
                        return Ok(Type::Enum { name: other.to_string(), variants: variants.clone() });
                    }
                }
                Err(TypeError::new(format!("unknown type '{}'", other)))
            }
        }
    }
}

/// Type-check a list of statements.
fn check_stmts(
    stmts: &[Stmt],
    locals: &mut HashMap<String, (Type, bool)>,
    fn_env: &HashMap<String, (Vec<Type>, Type)>,
    ret_type: &Type,
    structs: &HashMap<String, Vec<(String, Type)>>,
    enums: &HashMap<String, Vec<(String, Vec<Type>)>>,
    methods: &HashMap<(String, String), (Vec<Type>, Type)>,
    generic_fns: &HashMap<String, GenericFnInfo>,
    traits: &HashMap<String, Vec<(String, Vec<Type>, Type)>>,
    module_exports: &HashMap<String, ModuleExports>,
) -> Result<(), TypeError> {
    for stmt in stmts {
        check_stmt(stmt, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
    }
    Ok(())
}

fn check_stmt(
    stmt: &Stmt,
    locals: &mut HashMap<String, (Type, bool)>,
    fn_env: &HashMap<String, (Vec<Type>, Type)>,
    ret_type: &Type,
    structs: &HashMap<String, Vec<(String, Type)>>,
    enums: &HashMap<String, Vec<(String, Vec<Type>)>>,
    methods: &HashMap<(String, String), (Vec<Type>, Type)>,
    generic_fns: &HashMap<String, GenericFnInfo>,
    traits: &HashMap<String, Vec<(String, Vec<Type>, Type)>>,
    module_exports: &HashMap<String, ModuleExports>,
) -> Result<(), TypeError> {
    match stmt {
        Stmt::Let { name, mutable, type_name, value, .. } => {
            let actual = check_expr(value, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            let ty = if let Some(tn) = type_name {
                let declared = resolve_type(&tn.name, structs, enums, module_exports)?;
                if declared != actual {
                    return Err(TypeError::new(format!(
                        "type mismatch in let '{}': declared {} but expression has type {}",
                        name, declared, actual
                    )));
                }
                declared
            } else {
                actual
            };
            locals.insert(name.clone(), (ty, *mutable));
            Ok(())
        }
        Stmt::Expr(expr) => {
            check_expr(expr, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            Ok(())
        }
        Stmt::Break { .. } => Ok(()),
        Stmt::Continue { .. } => Ok(()),
        Stmt::While { condition, body, .. } => {
            let cond_ty = check_expr(condition, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "while condition must be Bool, got {}", cond_ty
                )));
            }
            check_stmts(body, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            Ok(())
        }
        Stmt::Return { value, .. } => {
            let ty = check_expr(value, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            if !types_compatible(&ty, ret_type) {
                return Err(TypeError::new(format!(
                    "return type mismatch: expected {} but got {}",
                    ret_type, ty
                )));
            }
            Ok(())
        }
        Stmt::Assign { name, value, .. } => {
            if let Some((expected_ty, is_mutable)) = locals.get(name).cloned() {
                // Existing variable — assignment
                if !is_mutable {
                    return Err(TypeError::new(format!(
                        "cannot assign to immutable variable '{}'", name
                    )));
                }
                let actual = check_expr(value, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if actual != expected_ty {
                    return Err(TypeError::new(format!(
                        "type mismatch in assignment to '{}': expected {} but got {}",
                        name, expected_ty, actual
                    )));
                }
            } else {
                // New variable — bare assignment creates immutable binding
                let val_type = check_expr(value, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                locals.insert(name.clone(), (val_type, false));
            }
            Ok(())
        }
        Stmt::If { condition, body, .. } => {
            let cond_ty = check_expr(condition, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "if condition must be Bool, got {}", cond_ty
                )));
            }
            check_stmts(body, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            Ok(())
        }
        Stmt::ForIn { var, iterable, body, .. } => {
            let iter_ty = check_expr(iterable, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            match iter_ty {
                Type::Array { inner } => {
                    let mut loop_locals = locals.clone();
                    loop_locals.insert(var.clone(), (*inner, false));
                    for stmt in body {
                        check_stmt(stmt, &mut loop_locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    }
                    Ok(())
                }
                _ => Err(TypeError::new(format!("for-in requires Array, got {}", iter_ty))),
            }
        }
        Stmt::LetDestructure { names, value, .. } => {
            match value {
                Expr::ChannelCreate { element_type, capacity, .. } => {
                    if let Some(cap_expr) = capacity {
                        let cap_ty = check_expr(cap_expr, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                        if cap_ty != Type::Int {
                            return Err(TypeError::new(format!(
                                "channel capacity must be Int, got {}", cap_ty
                            )));
                        }
                    }
                    let inner = resolve_type(&element_type.name, structs, enums, module_exports)?;
                    if names.len() != 2 {
                        return Err(TypeError::new("channel destructuring requires exactly 2 names"));
                    }
                    locals.insert(names[0].clone(), (Type::Sender { inner: Box::new(inner.clone()) }, false));
                    locals.insert(names[1].clone(), (Type::Receiver { inner: Box::new(inner) }, false));
                    Ok(())
                }
                _ => Err(TypeError::new("destructuring let is only supported for channel<T>()")),
            }
        }
    }
}

/// Type-check the given `Program`. Returns `Ok(ModuleExports)` if the program is
/// well-typed, or a `TypeError` describing the first problem found.
pub fn check(program: &Program, module_exports: &HashMap<String, ModuleExports>) -> Result<ModuleExports, TypeError> {
    check_inner(program, module_exports, true)
}

fn check_inner(program: &Program, module_exports: &HashMap<String, ModuleExports>, require_main: bool) -> Result<ModuleExports, TypeError> {
    // Pass 0a: collect struct definitions.
    let mut struct_registry: HashMap<String, Vec<(String, Type)>> = HashMap::new();
    let enum_registry: HashMap<String, Vec<(String, Vec<Type>)>> = HashMap::new();
    for s in &program.structs {
        let mut fields = Vec::new();
        for f in &s.fields {
            let ty = resolve_type(&f.type_name.name, &struct_registry, &enum_registry, module_exports)?;
            fields.push((f.name.clone(), ty));
        }
        struct_registry.insert(s.name.clone(), fields);
    }

    // Pass 0b: collect enum definitions.
    let mut enum_registry: HashMap<String, Vec<(String, Vec<Type>)>> = HashMap::new();
    for e in &program.enums {
        let mut variants = Vec::new();
        for v in &e.variants {
            let mut field_types = Vec::new();
            for f in &v.fields {
                let ty = resolve_type(&f.name, &struct_registry, &enum_registry, module_exports)?;
                field_types.push(ty);
            }
            variants.push((v.name.clone(), field_types));
        }
        enum_registry.insert(e.name.clone(), variants);
    }

    // Pass 0c: collect trait definitions
    let mut trait_registry: HashMap<String, Vec<(String, Vec<Type>, Type)>> = HashMap::new();
    for t in &program.traits {
        let mut trait_methods = Vec::new();
        for m in &t.methods {
            let param_types: Vec<Type> = m.params.iter()
                .map(|p| resolve_type(&p.type_name.name, &struct_registry, &enum_registry, module_exports))
                .collect::<Result<_, _>>()?;
            let ret_type = resolve_type(&m.return_type.name, &struct_registry, &enum_registry, module_exports)?;
            trait_methods.push((m.name.clone(), param_types, ret_type));
        }
        trait_registry.insert(t.name.clone(), trait_methods);
    }

    // Pass 0d: collect impl blocks — build method registry
    // method_registry: (type_name, method_name) -> (param_types_without_self, return_type)
    let mut method_registry: HashMap<(String, String), (Vec<Type>, Type)> = HashMap::new();
    for imp in &program.impls {
        // Verify trait conformance if trait impl
        if let Some(trait_name) = &imp.trait_name {
            let trait_methods = trait_registry.get(trait_name)
                .ok_or_else(|| TypeError::new(format!("undefined trait '{}'", trait_name)))?;
            for (method_name, expected_params, expected_ret) in trait_methods {
                let method = imp.methods.iter().find(|m| m.name == *method_name)
                    .ok_or_else(|| TypeError::new(format!(
                        "impl {} for {} is missing method '{}'",
                        trait_name, imp.target_type, method_name
                    )))?;
                // Check params (skip self which is first)
                let actual_params: Vec<Type> = method.params[1..].iter()
                    .map(|p| resolve_type(&p.type_name.name, &struct_registry, &enum_registry, module_exports))
                    .collect::<Result<_, _>>()?;
                if actual_params != *expected_params {
                    return Err(TypeError::new(format!(
                        "method '{}' params don't match trait '{}' signature",
                        method_name, trait_name
                    )));
                }
                let actual_ret = resolve_type(&method.return_type.name, &struct_registry, &enum_registry, module_exports)?;
                if actual_ret != *expected_ret {
                    return Err(TypeError::new(format!(
                        "method '{}' return type doesn't match trait '{}' signature",
                        method_name, trait_name
                    )));
                }
            }
        }

        for method in &imp.methods {
            let param_types: Vec<Type> = method.params[1..].iter() // skip self
                .map(|p| resolve_type(&p.type_name.name, &struct_registry, &enum_registry, module_exports))
                .collect::<Result<_, _>>()?;
            let ret_type = resolve_type(&method.return_type.name, &struct_registry, &enum_registry, module_exports)?;
            method_registry.insert(
                (imp.target_type.clone(), method.name.clone()),
                (param_types, ret_type),
            );
        }
    }

    // Pass 1: collect all function signatures into an environment.
    let mut fn_env: HashMap<String, (Vec<Type>, Type)> = HashMap::new();
    let mut generic_fn_env: HashMap<String, GenericFnInfo> = HashMap::new();

    for func in &program.functions {
        if func.type_params.is_empty() {
            let mut param_types = Vec::new();
            for param in &func.params {
                param_types.push(resolve_type(&param.type_name.name, &struct_registry, &enum_registry, module_exports)?);
            }
            let ret_type = resolve_type(&func.return_type.name, &struct_registry, &enum_registry, module_exports)?;
            fn_env.insert(func.name.clone(), (param_types, ret_type));
        } else {
            generic_fn_env.insert(func.name.clone(), GenericFnInfo {
                type_params: func.type_params.iter().map(|tp| (tp.name.clone(), tp.bound.clone())).collect(),
                param_types: func.params.iter().map(|p| p.type_name.name.clone()).collect(),
                return_type_name: func.return_type.name.clone(),
            });
        }
    }

    // Add mangled method signatures to fn_env for IR/codegen
    for imp in &program.impls {
        for method in &imp.methods {
            let mangled = format!("{}_{}", imp.target_type, method.name);
            let mut param_types = Vec::new();
            for p in &method.params {
                param_types.push(resolve_type(&p.type_name.name, &struct_registry, &enum_registry, module_exports)?);
            }
            let ret_type = resolve_type(&method.return_type.name, &struct_registry, &enum_registry, module_exports)?;
            fn_env.insert(mangled, (param_types, ret_type));
        }
    }

    // Pass 1b: type-check global variable definitions.
    let mut global_types: HashMap<String, Type> = HashMap::new();
    {
        let mut global_locals: HashMap<String, (Type, bool)> = HashMap::new();
        for gdef in &program.globals {
            let ty = check_expr(&gdef.value, &global_locals, &fn_env, &Type::Int, &struct_registry, &enum_registry, &method_registry, &generic_fn_env, &trait_registry, module_exports)?;
            global_types.insert(gdef.name.clone(), ty.clone());
            global_locals.insert(gdef.name.clone(), (ty, true));
        }
    }

    // Require a `main` function (unless checking a library module).
    if require_main && !fn_env.contains_key("main") {
        return Err(TypeError::new("missing 'main' function"));
    }

    // Pass 2: type-check each function body.
    for func in &program.functions {
        // Skip type-checking generic function bodies — they are validated at each call site
        if !func.type_params.is_empty() {
            continue;
        }

        let (_, ret_type) = fn_env.get(&func.name).unwrap();
        let ret_type = ret_type.clone();

        let mut locals: HashMap<String, (Type, bool)> = HashMap::new();
        // Inject globals as mutable locals
        for (gname, gty) in &global_types {
            locals.insert(gname.clone(), (gty.clone(), true));
        }
        for param in &func.params {
            let ty = resolve_type(&param.type_name.name, &struct_registry, &enum_registry, module_exports)?;
            locals.insert(param.name.clone(), (ty, false));
        }

        if func.body.is_empty() {
            return Err(TypeError::new(format!(
                "function '{}': missing return expression", func.name
            )));
        }

        // Check all statements
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == func.body.len() - 1;
            check_stmt(stmt, &mut locals, &fn_env, &ret_type, &struct_registry, &enum_registry, &method_registry, &generic_fn_env, &trait_registry, module_exports)?;

            if is_last {
                match stmt {
                    Stmt::Expr(expr) => {
                        let ty = check_expr(expr, &locals, &fn_env, &ret_type, &struct_registry, &enum_registry, &method_registry, &generic_fn_env, &trait_registry, module_exports)?;
                        if !types_compatible(&ty, &ret_type) {
                            return Err(TypeError::new(format!(
                                "function '{}': return type mismatch: expected {} but got {}",
                                func.name, ret_type, ty
                            )));
                        }
                    }
                    Stmt::Return { .. } => {
                        // Already type-checked in check_stmt
                    }
                    Stmt::Let { .. } | Stmt::While { .. } | Stmt::Assign { .. } | Stmt::If { .. } | Stmt::LetDestructure { .. } | Stmt::ForIn { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {
                        return Err(TypeError::new(format!(
                            "function '{}': missing return expression", func.name
                        )));
                    }
                }
            }
        }
    }

    // Pass 3: type-check method bodies from impl blocks.
    for imp in &program.impls {
        for method in &imp.methods {
            let ret_type = resolve_type(&method.return_type.name, &struct_registry, &enum_registry, module_exports)?;

            let mut locals: HashMap<String, (Type, bool)> = HashMap::new();
            // Inject globals as mutable locals
            for (gname, gty) in &global_types {
                locals.insert(gname.clone(), (gty.clone(), true));
            }
            for param in &method.params {
                let ty = resolve_type(&param.type_name.name, &struct_registry, &enum_registry, module_exports)?;
                locals.insert(param.name.clone(), (ty, false));
            }

            if method.body.is_empty() {
                return Err(TypeError::new(format!(
                    "method '{}': missing return expression", method.name
                )));
            }

            for (i, stmt) in method.body.iter().enumerate() {
                let is_last = i == method.body.len() - 1;
                check_stmt(stmt, &mut locals, &fn_env, &ret_type, &struct_registry, &enum_registry, &method_registry, &generic_fn_env, &trait_registry, module_exports)?;

                if is_last {
                    match stmt {
                        Stmt::Expr(expr) => {
                            let ty = check_expr(expr, &locals, &fn_env, &ret_type, &struct_registry, &enum_registry, &method_registry, &generic_fn_env, &trait_registry, module_exports)?;
                            if !types_compatible(&ty, &ret_type) {
                                return Err(TypeError::new(format!(
                                    "method '{}': return type mismatch: expected {} but got {}",
                                    method.name, ret_type, ty
                                )));
                            }
                        }
                        Stmt::Return { .. } => {}
                        _ => {
                            return Err(TypeError::new(format!(
                                "method '{}': missing return expression", method.name
                            )));
                        }
                    }
                }
            }
        }
    }

    let mut fn_exports = HashMap::new();
    for func in &program.functions {
        if func.type_params.is_empty() {
            let (param_types, ret_type) = fn_env.get(&func.name).unwrap();
            fn_exports.insert(func.name.clone(), FunctionSignature {
                params: param_types.clone(),
                return_type: ret_type.clone(),
            });
        }
    }

    Ok(ModuleExports {
        functions: fn_exports,
        structs: struct_registry,
        enums: enum_registry,
    })
}

/// Type-check a library module (no `main` function required).
/// Otherwise identical to `check`.
pub fn check_module(program: &Program, module_exports: &HashMap<String, ModuleExports>) -> Result<ModuleExports, TypeError> {
    check_inner(program, module_exports, false)
}

/// Type-check a single expression and return its type.
fn check_expr(
    expr: &Expr,
    locals: &HashMap<String, (Type, bool)>,
    fn_env: &HashMap<String, (Vec<Type>, Type)>,
    ret_type: &Type,
    structs: &HashMap<String, Vec<(String, Type)>>,
    enums: &HashMap<String, Vec<(String, Vec<Type>)>>,
    methods: &HashMap<(String, String), (Vec<Type>, Type)>,
    generic_fns: &HashMap<String, GenericFnInfo>,
    traits: &HashMap<String, Vec<(String, Vec<Type>, Type)>>,
    module_exports: &HashMap<String, ModuleExports>,
) -> Result<Type, TypeError> {
    match expr {
        Expr::IntLiteral { .. } => Ok(Type::Int),
        Expr::FloatLiteral { .. } => Ok(Type::Float),
        Expr::StringLiteral { .. } => Ok(Type::String),

        Expr::Identifier { name, .. } => {
            if let Some((ty, _)) = locals.get(name) {
                Ok(ty.clone())
            } else if let Some((param_types, ret_type)) = fn_env.get(name) {
                // Function reference — treat as a first-class function value
                Ok(Type::Fn { params: param_types.clone(), ret: Box::new(ret_type.clone()) })
            } else {
                Err(TypeError::new(format!("undefined variable '{}'", name)))
            }
        }

        Expr::BinaryOp { left, op, right, .. } => {
            use sans_parser::ast::BinOp;
            let lt = check_expr(left, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            let rt = check_expr(right, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;

            match op {
                // Arithmetic: Int x Int -> Int, Float x Float -> Float, String + String -> String
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                    if op == &BinOp::Add && lt == Type::String && rt == Type::String {
                        return Ok(Type::String);
                    }
                    if lt == Type::Float && rt == Type::Float {
                        return Ok(Type::Float);
                    }
                    if lt != Type::Int {
                        return Err(TypeError::new(format!(
                            "arithmetic operator requires Int operands, left operand is {}", lt
                        )));
                    }
                    if rt != Type::Int {
                        return Err(TypeError::new(format!(
                            "arithmetic operator requires Int operands, right operand is {}", rt
                        )));
                    }
                    Ok(Type::Int)
                }
                // Comparison: same-type operands -> Bool
                BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                    if lt == Type::Float && rt == Type::Float {
                        return Ok(Type::Bool);
                    }
                    if lt == Type::String && rt == Type::String {
                        match op {
                            BinOp::Eq | BinOp::NotEq => return Ok(Type::Bool),
                            _ => return Err(TypeError::new("String only supports == and != comparison")),
                        }
                    }
                    if lt == Type::Bool && rt == Type::Bool {
                        match op {
                            BinOp::Eq | BinOp::NotEq => return Ok(Type::Bool),
                            _ => return Err(TypeError::new("Bool only supports == and != comparison")),
                        }
                    }
                    if lt != Type::Int {
                        return Err(TypeError::new(format!(
                            "comparison operator requires matching operands, left operand is {}", lt
                        )));
                    }
                    if rt != Type::Int {
                        return Err(TypeError::new(format!(
                            "comparison operator requires matching operands, right operand is {}", rt
                        )));
                    }
                    Ok(Type::Bool)
                }
                // Boolean: Bool x Bool -> Bool
                BinOp::And | BinOp::Or => {
                    if lt != Type::Bool {
                        return Err(TypeError::new(format!(
                            "boolean operator requires Bool operands, left operand is {}", lt
                        )));
                    }
                    if rt != Type::Bool {
                        return Err(TypeError::new(format!(
                            "boolean operator requires Bool operands, right operand is {}", rt
                        )));
                    }
                    Ok(Type::Bool)
                }
            }
        }

        Expr::BoolLiteral { .. } => Ok(Type::Bool),

        Expr::UnaryOp { op, operand, .. } => {
            match op {
                sans_parser::ast::UnaryOp::Not => {
                    let ty = check_expr(operand, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if ty != Type::Bool {
                        return Err(TypeError::new(format!(
                            "'!' operator requires Bool operand, got {}",
                            ty
                        )));
                    }
                    Ok(Type::Bool)
                }
                sans_parser::ast::UnaryOp::Neg => {
                    let ty = check_expr(operand, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if ty != Type::Int && ty != Type::Float {
                        return Err(TypeError::new(format!(
                            "'-' operator requires Int or Float operand, got {}",
                            ty
                        )));
                    }
                    Ok(ty)
                }
            }
        }

        Expr::If { condition, then_body, then_expr, else_body, else_expr, .. } => {
            let cond_ty = check_expr(condition, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "if condition must be Bool, got {}",
                    cond_ty
                )));
            }

            // Type-check then branch (body stmts + final expr)
            let mut then_locals = locals.clone();
            check_stmts(then_body, &mut then_locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            let then_ty = check_expr(then_expr, &then_locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;

            // Type-check else branch
            let mut else_locals = locals.clone();
            check_stmts(else_body, &mut else_locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            let else_ty = check_expr(else_expr, &else_locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;

            // Both branches must have the same type (ResultErr is compatible with any Result<T>)
            let merged_ty = match (&then_ty, &else_ty) {
                _ if then_ty == else_ty => then_ty,
                (Type::ResultErr, Type::Result { .. }) => else_ty,
                (Type::Result { .. }, Type::ResultErr) => then_ty,
                (Type::ResultErr, Type::ResultErr) => then_ty,
                _ => {
                    return Err(TypeError::new(format!(
                        "if/else branch type mismatch: then branch is {} but else branch is {}",
                        then_ty, else_ty
                    )));
                }
            };

            Ok(merged_ty)
        }

        Expr::Call { function, args, .. } => {
            if function == "print" || function == "p" {
                if args.len() != 1 {
                    return Err(TypeError::new(format!(
                        "print() takes exactly 1 argument, got {}", args.len()
                    )));
                }
                let _arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                // Accept any type — non-primitives print raw i64 (pointer value)
                return Ok(Type::Int);
            } else if function == "int_to_string" || function == "str" || function == "itos" {
                if args.len() != 1 {
                    return Err(TypeError::new("int_to_string() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int && arg_ty != Type::String && arg_ty != Type::Float && arg_ty != Type::Bool {
                    return Err(TypeError::new(format!("str() requires Int, String, Float, or Bool argument, got {}", arg_ty)));
                }
                return Ok(Type::String);
            } else if function == "string_to_int" || function == "stoi" {
                if args.len() != 1 {
                    return Err(TypeError::new("string_to_int() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("string_to_int() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "int_to_float" || function == "itof" {
                if args.len() != 1 {
                    return Err(TypeError::new("int_to_float() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("int_to_float() requires Int argument, got {}", arg_ty)));
                }
                return Ok(Type::Float);
            } else if function == "float_to_int" || function == "ftoi" {
                if args.len() != 1 {
                    return Err(TypeError::new("float_to_int() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Float {
                    return Err(TypeError::new(format!("float_to_int() requires Float argument, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "float_to_string" || function == "ftos" {
                if args.len() != 1 {
                    return Err(TypeError::new("float_to_string() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Float {
                    return Err(TypeError::new(format!("float_to_string() requires Float argument, got {}", arg_ty)));
                }
                return Ok(Type::String);
            } else if function == "file_read" || function == "read_file" || function == "fread" || function == "fr" {
                if args.len() != 1 {
                    return Err(TypeError::new("file_read() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("file_read() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::String);
            } else if function == "file_write" || function == "write_file" || function == "fwrite" || function == "fw" || function == "file_append" || function == "append_file" || function == "fappend" || function == "fa" {
                if args.len() != 2 {
                    return Err(TypeError::new(format!("{}() takes exactly 2 arguments", function)));
                }
                let path_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if path_ty != Type::String {
                    return Err(TypeError::new(format!("{}() requires String as first argument, got {}", function, path_ty)));
                }
                let content_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if content_ty != Type::String {
                    return Err(TypeError::new(format!("{}() requires String as second argument, got {}", function, content_ty)));
                }
                return Ok(Type::Int);
            } else if function == "file_exists" || function == "fexists" || function == "fe" {
                if args.len() != 1 {
                    return Err(TypeError::new("file_exists() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("file_exists() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::Bool);
            } else if function == "args" {
                if !args.is_empty() {
                    return Err(TypeError::new("args() takes 0 arguments"));
                }
                return Ok(Type::Array { inner: Box::new(Type::String) });
            } else if function == "json_parse" || function == "jparse" || function == "jp" {
                if args.len() != 1 {
                    return Err(TypeError::new("json_parse() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("json_parse() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::JsonValue);
            } else if function == "map" || function == "M" {
                if !args.is_empty() {
                    return Err(TypeError::new("map() takes 0 arguments"));
                }
                return Ok(Type::Map);
            } else if function == "json_object" || function == "jobj" || function == "jo" {
                if !args.is_empty() {
                    return Err(TypeError::new("json_object() takes 0 arguments"));
                }
                return Ok(Type::JsonValue);
            } else if function == "json_array" || function == "jarr" || function == "ja" {
                if !args.is_empty() {
                    return Err(TypeError::new("json_array() takes 0 arguments"));
                }
                return Ok(Type::JsonValue);
            } else if function == "json_string" || function == "jstr" || function == "js" {
                if args.len() != 1 {
                    return Err(TypeError::new("json_string() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("json_string() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::JsonValue);
            } else if function == "json_int" || function == "ji" {
                if args.len() != 1 {
                    return Err(TypeError::new("json_int() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("json_int() requires Int argument, got {}", arg_ty)));
                }
                return Ok(Type::JsonValue);
            } else if function == "json_bool" || function == "jb" {
                if args.len() != 1 {
                    return Err(TypeError::new("json_bool() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Bool {
                    return Err(TypeError::new(format!("json_bool() requires Bool argument, got {}", arg_ty)));
                }
                return Ok(Type::JsonValue);
            } else if function == "json_null" || function == "jn" {
                if !args.is_empty() {
                    return Err(TypeError::new("json_null() takes 0 arguments"));
                }
                return Ok(Type::JsonValue);
            } else if function == "json_stringify" || function == "jstringify" || function == "jfy" {
                if args.len() != 1 {
                    return Err(TypeError::new("json_stringify() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::JsonValue {
                    return Err(TypeError::new(format!("json_stringify() requires JsonValue argument, got {}", arg_ty)));
                }
                return Ok(Type::String);
            } else if function == "http_get" || function == "hget" || function == "hg" {
                if args.len() != 1 {
                    return Err(TypeError::new("http_get() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("http_get() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::HttpResponse);
            } else if function == "http_post" || function == "hpost" || function == "hp" {
                if args.len() != 3 {
                    return Err(TypeError::new("http_post() takes exactly 3 arguments (url, body, content_type)"));
                }
                let url_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if url_ty != Type::String {
                    return Err(TypeError::new(format!("http_post() url must be String, got {}", url_ty)));
                }
                let body_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if body_ty != Type::String {
                    return Err(TypeError::new(format!("http_post() body must be String, got {}", body_ty)));
                }
                let ct_ty = check_expr(&args[2], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ct_ty != Type::String {
                    return Err(TypeError::new(format!("http_post() content_type must be String, got {}", ct_ty)));
                }
                return Ok(Type::HttpResponse);
            } else if function == "log_debug" || function == "ld" || function == "log_info" || function == "li" || function == "log_warn" || function == "lw" || function == "log_error" || function == "le" {
                if args.len() != 1 {
                    return Err(TypeError::new(format!("{}() takes exactly 1 argument", function)));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("{}() requires String argument, got {}", function, arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "log_set_level" || function == "ll" {
                if args.len() != 1 {
                    return Err(TypeError::new("log_set_level() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("log_set_level() requires Int argument, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "print_err" {
                if args.len() != 1 {
                    return Err(TypeError::new("print_err() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("print_err() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "fptr" {
                if args.len() != 1 {
                    return Err(TypeError::new("fptr() takes exactly 1 argument (function name)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("fptr() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "fcall" {
                if args.len() != 2 {
                    return Err(TypeError::new("fcall() takes exactly 2 arguments (fn_ptr, arg)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall() fn_ptr must be Int, got {}", ptr_ty)));
                }
                let arg_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall() arg must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "fcall2" {
                if args.len() != 3 {
                    return Err(TypeError::new("fcall2() takes exactly 3 arguments (fn_ptr, arg1, arg2)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall2() fn_ptr must be Int, got {}", ptr_ty)));
                }
                let a1_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if a1_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall2() arg1 must be Int, got {}", a1_ty)));
                }
                let a2_ty = check_expr(&args[2], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if a2_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall2() arg2 must be Int, got {}", a2_ty)));
                }
                return Ok(Type::Int);
            } else if function == "fcall3" {
                if args.len() != 4 {
                    return Err(TypeError::new("fcall3() takes exactly 4 arguments (fn_ptr, arg1, arg2, arg3)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall3() fn_ptr must be Int, got {}", ptr_ty)));
                }
                let a1_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if a1_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall3() arg1 must be Int, got {}", a1_ty)));
                }
                let a2_ty = check_expr(&args[2], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if a2_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall3() arg2 must be Int, got {}", a2_ty)));
                }
                let a3_ty = check_expr(&args[3], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if a3_ty != Type::Int {
                    return Err(TypeError::new(format!("fcall3() arg3 must be Int, got {}", a3_ty)));
                }
                return Ok(Type::Int);
            } else if function == "wfd" {
                if args.len() != 2 {
                    return Err(TypeError::new("wfd() takes exactly 2 arguments (fd, msg)"));
                }
                let fd_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if fd_ty != Type::Int {
                    return Err(TypeError::new(format!("wfd() fd must be Int, got {}", fd_ty)));
                }
                let msg_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if msg_ty != Type::String {
                    return Err(TypeError::new(format!("wfd() msg must be String, got {}", msg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "ptr" {
                if args.len() != 1 {
                    return Err(TypeError::new("ptr() takes exactly 1 argument"));
                }
                check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                return Ok(Type::Int);
            } else if function == "alloc" {
                if args.len() != 1 {
                    return Err(TypeError::new("alloc() takes exactly 1 argument (size)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("alloc() size must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "dealloc" {
                if args.len() != 1 {
                    return Err(TypeError::new("dealloc() takes exactly 1 argument (ptr)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("dealloc() ptr must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "ralloc" {
                if args.len() != 2 {
                    return Err(TypeError::new("ralloc() takes exactly 2 arguments (ptr, size)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("ralloc() ptr must be Int, got {}", ptr_ty)));
                }
                let size_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if size_ty != Type::Int {
                    return Err(TypeError::new(format!("ralloc() size must be Int, got {}", size_ty)));
                }
                return Ok(Type::Int);
            } else if function == "mcpy" {
                if args.len() != 3 {
                    return Err(TypeError::new("mcpy() takes exactly 3 arguments (dst, src, n)"));
                }
                let dst_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if dst_ty != Type::Int {
                    return Err(TypeError::new(format!("mcpy() dst must be Int, got {}", dst_ty)));
                }
                let src_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if src_ty != Type::Int {
                    return Err(TypeError::new(format!("mcpy() src must be Int, got {}", src_ty)));
                }
                let n_ty = check_expr(&args[2], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if n_ty != Type::Int {
                    return Err(TypeError::new(format!("mcpy() n must be Int, got {}", n_ty)));
                }
                return Ok(Type::Int);
            } else if function == "mzero" {
                if args.len() != 2 {
                    return Err(TypeError::new("mzero() takes exactly 2 arguments (ptr, n)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("mzero() ptr must be Int, got {}", ptr_ty)));
                }
                let n_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if n_ty != Type::Int {
                    return Err(TypeError::new(format!("mzero() n must be Int, got {}", n_ty)));
                }
                return Ok(Type::Int);
            } else if function == "mcmp" {
                if args.len() != 3 {
                    return Err(TypeError::new("mcmp() takes exactly 3 arguments (a, b, n)"));
                }
                let a_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if a_ty != Type::Int {
                    return Err(TypeError::new(format!("mcmp() a must be Int, got {}", a_ty)));
                }
                let b_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if b_ty != Type::Int {
                    return Err(TypeError::new(format!("mcmp() b must be Int, got {}", b_ty)));
                }
                let n_ty = check_expr(&args[2], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if n_ty != Type::Int {
                    return Err(TypeError::new(format!("mcmp() n must be Int, got {}", n_ty)));
                }
                return Ok(Type::Int);
            } else if function == "slen" {
                if args.len() != 1 {
                    return Err(TypeError::new("slen() takes exactly 1 argument (ptr)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("slen() ptr must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "char_at" {
                if args.len() != 2 {
                    return Err(TypeError::new("char_at() takes exactly 2 arguments (string, index)"));
                }
                let s_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if s_ty != Type::String {
                    return Err(TypeError::new(format!("char_at() first arg must be String, got {}", s_ty)));
                }
                let i_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if i_ty != Type::Int {
                    return Err(TypeError::new(format!("char_at() second arg must be Int, got {}", i_ty)));
                }
                return Ok(Type::Int);
            } else if function == "load8" {
                if args.len() != 1 {
                    return Err(TypeError::new("load8() takes exactly 1 argument (ptr)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("load8() ptr must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "store8" {
                if args.len() != 2 {
                    return Err(TypeError::new("store8() takes exactly 2 arguments (ptr, val)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("store8() ptr must be Int, got {}", ptr_ty)));
                }
                let val_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if val_ty != Type::Int {
                    return Err(TypeError::new(format!("store8() val must be Int, got {}", val_ty)));
                }
                return Ok(Type::Int);
            } else if function == "load16" {
                if args.len() != 1 {
                    return Err(TypeError::new("load16() takes exactly 1 argument (ptr)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("load16() ptr must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "store16" {
                if args.len() != 2 {
                    return Err(TypeError::new("store16() takes exactly 2 arguments (ptr, val)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("store16() ptr must be Int, got {}", ptr_ty)));
                }
                let val_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if val_ty != Type::Int {
                    return Err(TypeError::new(format!("store16() val must be Int, got {}", val_ty)));
                }
                return Ok(Type::Int);
            } else if function == "load32" {
                if args.len() != 1 {
                    return Err(TypeError::new("load32() takes exactly 1 argument (ptr)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("load32() ptr must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "store32" {
                if args.len() != 2 {
                    return Err(TypeError::new("store32() takes exactly 2 arguments (ptr, val)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("store32() ptr must be Int, got {}", ptr_ty)));
                }
                let val_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if val_ty != Type::Int {
                    return Err(TypeError::new(format!("store32() val must be Int, got {}", val_ty)));
                }
                return Ok(Type::Int);
            } else if function == "bswap16" {
                if args.len() != 1 {
                    return Err(TypeError::new("bswap16() takes exactly 1 argument (val)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("bswap16() val must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "rbind" {
                if args.len() != 3 {
                    return Err(TypeError::new("rbind() takes exactly 3 arguments (fd, addr, len)"));
                }
                for (i, label) in ["fd", "addr", "len"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("rbind() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "rsetsockopt" {
                if args.len() != 5 {
                    return Err(TypeError::new("rsetsockopt() takes exactly 5 arguments (fd, level, opt, val_ptr, val_len)"));
                }
                for (i, label) in ["fd", "level", "opt", "val_ptr", "val_len"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("rsetsockopt() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "load64" {
                if args.len() != 1 {
                    return Err(TypeError::new("load64() takes exactly 1 argument (ptr)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("load64() ptr must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "store64" {
                if args.len() != 2 {
                    return Err(TypeError::new("store64() takes exactly 2 arguments (ptr, val)"));
                }
                let ptr_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if ptr_ty != Type::Int {
                    return Err(TypeError::new(format!("store64() ptr must be Int, got {}", ptr_ty)));
                }
                let val_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if val_ty != Type::Int {
                    return Err(TypeError::new(format!("store64() val must be Int, got {}", val_ty)));
                }
                return Ok(Type::Int);
            } else if function == "strstr" {
                if args.len() != 2 {
                    return Err(TypeError::new("strstr() takes exactly 2 arguments (haystack, needle)"));
                }
                let haystack_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if haystack_ty != Type::Int {
                    return Err(TypeError::new(format!("strstr() haystack must be Int, got {}", haystack_ty)));
                }
                let needle_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if needle_ty != Type::Int {
                    return Err(TypeError::new(format!("strstr() needle must be Int, got {}", needle_ty)));
                }
                return Ok(Type::Int);
            } else if function == "exit" {
                if args.len() != 1 {
                    return Err(TypeError::new("exit() takes exactly 1 argument (code)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("exit() code must be Int, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "sock" {
                if args.len() != 3 {
                    return Err(TypeError::new("sock() takes exactly 3 arguments (domain, type, proto)"));
                }
                for (i, label) in ["domain", "type", "proto"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("sock() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "sbind" {
                if args.len() != 2 {
                    return Err(TypeError::new("sbind() takes exactly 2 arguments (fd, port)"));
                }
                for (i, label) in ["fd", "port"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("sbind() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "slisten" {
                if args.len() != 2 {
                    return Err(TypeError::new("slisten() takes exactly 2 arguments (fd, backlog)"));
                }
                for (i, label) in ["fd", "backlog"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("slisten() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "saccept" {
                if args.len() != 1 {
                    return Err(TypeError::new("saccept() takes exactly 1 argument (fd)"));
                }
                let t = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if t != Type::Int {
                    return Err(TypeError::new(format!("saccept() fd must be Int, got {}", t)));
                }
                return Ok(Type::Int);
            } else if function == "srecv" {
                if args.len() != 3 {
                    return Err(TypeError::new("srecv() takes exactly 3 arguments (fd, buf, len)"));
                }
                for (i, label) in ["fd", "buf", "len"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("srecv() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "ssend" {
                if args.len() != 3 {
                    return Err(TypeError::new("ssend() takes exactly 3 arguments (fd, buf, len)"));
                }
                for (i, label) in ["fd", "buf", "len"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("ssend() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "sclose" {
                if args.len() != 1 {
                    return Err(TypeError::new("sclose() takes exactly 1 argument (fd)"));
                }
                let t = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if t != Type::Int {
                    return Err(TypeError::new(format!("sclose() fd must be Int, got {}", t)));
                }
                return Ok(Type::Int);
            } else if function == "cinit" {
                if !args.is_empty() {
                    return Err(TypeError::new("cinit() takes no arguments"));
                }
                return Ok(Type::Int);
            } else if function == "csets" {
                if args.len() != 3 {
                    return Err(TypeError::new("csets() takes exactly 3 arguments (handle, opt, val)"));
                }
                let handle_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if handle_ty != Type::Int {
                    return Err(TypeError::new(format!("csets() handle must be Int, got {}", handle_ty)));
                }
                let opt_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if opt_ty != Type::Int {
                    return Err(TypeError::new(format!("csets() opt must be Int, got {}", opt_ty)));
                }
                let val_ty = check_expr(&args[2], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if val_ty != Type::String {
                    return Err(TypeError::new(format!("csets() val must be String, got {}", val_ty)));
                }
                return Ok(Type::Int);
            } else if function == "cseti" {
                if args.len() != 3 {
                    return Err(TypeError::new("cseti() takes exactly 3 arguments (handle, opt, val)"));
                }
                for (i, label) in ["handle", "opt", "val"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("cseti() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "cperf" {
                if args.len() != 1 {
                    return Err(TypeError::new("cperf() takes exactly 1 argument (handle)"));
                }
                let t = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if t != Type::Int {
                    return Err(TypeError::new(format!("cperf() handle must be Int, got {}", t)));
                }
                return Ok(Type::Int);
            } else if function == "cclean" {
                if args.len() != 1 {
                    return Err(TypeError::new("cclean() takes exactly 1 argument (handle)"));
                }
                let t = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if t != Type::Int {
                    return Err(TypeError::new(format!("cclean() handle must be Int, got {}", t)));
                }
                return Ok(Type::Int);
            } else if function == "cinfo" {
                if args.len() != 3 {
                    return Err(TypeError::new("cinfo() takes exactly 3 arguments (handle, info, buf)"));
                }
                for (i, label) in ["handle", "info", "buf"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("cinfo() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "curl_slist_append" {
                if args.len() != 2 {
                    return Err(TypeError::new("curl_slist_append() takes exactly 2 arguments (slist, str)"));
                }
                for (i, label) in ["slist", "str"].iter().enumerate() {
                    let t = check_expr(&args[i], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if t != Type::Int {
                        return Err(TypeError::new(format!("curl_slist_append() {} must be Int, got {}", label, t)));
                    }
                }
                return Ok(Type::Int);
            } else if function == "curl_slist_free" {
                if args.len() != 1 {
                    return Err(TypeError::new("curl_slist_free() takes exactly 1 argument (slist)"));
                }
                let t = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if t != Type::Int {
                    return Err(TypeError::new(format!("curl_slist_free() slist must be Int, got {}", t)));
                }
                return Ok(Type::Int);
            } else if function == "get_log_level" {
                if !args.is_empty() {
                    return Err(TypeError::new("get_log_level() takes no arguments"));
                }
                return Ok(Type::Int);
            } else if function == "set_log_level" {
                if args.len() != 1 {
                    return Err(TypeError::new("set_log_level() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("set_log_level() requires Int argument, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            } else if function == "http_listen" || function == "listen" || function == "hl" {
                if args.len() != 1 {
                    return Err(TypeError::new("http_listen() takes exactly 1 argument (port)"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("http_listen() requires Int port, got {}", arg_ty)));
                }
                return Ok(Type::HttpServer);
            } else if function == "ok" {
                if args.len() != 1 {
                    return Err(TypeError::new("ok() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                return Ok(Type::Result { inner: Box::new(arg_ty) });
            } else if function == "err" {
                if args.len() != 1 {
                    return Err(TypeError::new("err() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("err() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::ResultErr);
            }

            // Check regular functions first
            if let Some((param_types, call_ret_type)) = fn_env.get(function) {
                if args.len() != param_types.len() {
                    return Err(TypeError::new(format!(
                        "wrong argument count calling '{}': expected {} argument(s) but got {}",
                        function,
                        param_types.len(),
                        args.len()
                    )));
                }

                for (i, (arg, expected)) in args.iter().zip(param_types.iter()).enumerate() {
                    let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if !types_compatible(&actual, expected) {
                        return Err(TypeError::new(format!(
                            "argument {} to '{}': expected {} but got {}",
                            i + 1,
                            function,
                            expected,
                            actual
                        )));
                    }
                }

                return Ok(call_ret_type.clone());
            }

            // Check generic functions
            if let Some(generic_info) = generic_fns.get(function) {
                if args.len() != generic_info.param_types.len() {
                    return Err(TypeError::new(format!(
                        "wrong argument count calling '{}': expected {} argument(s) but got {}",
                        function, generic_info.param_types.len(), args.len()
                    )));
                }

                // Type-check args and infer type params
                let mut type_map: HashMap<String, Type> = HashMap::new();
                for (i, (arg, param_type_name)) in args.iter().zip(generic_info.param_types.iter()).enumerate() {
                    let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;

                    let is_type_param = generic_info.type_params.iter().any(|(n, _)| n == param_type_name);
                    if is_type_param {
                        if let Some(existing) = type_map.get(param_type_name) {
                            if *existing != actual {
                                return Err(TypeError::new(format!(
                                    "type parameter '{}' inferred as {} but argument {} has type {}",
                                    param_type_name, existing, i + 1, actual
                                )));
                            }
                        } else {
                            type_map.insert(param_type_name.clone(), actual.clone());
                        }
                    } else {
                        let expected = resolve_type(param_type_name, structs, enums, module_exports)?;
                        if actual != expected {
                            return Err(TypeError::new(format!(
                                "argument {} to '{}': expected {} but got {}",
                                i + 1, function, expected, actual
                            )));
                        }
                    }
                }

                // Verify trait bounds
                for (tp_name, bound) in &generic_info.type_params {
                    if let Some(trait_name) = bound {
                        let concrete_type = type_map.get(tp_name)
                            .ok_or_else(|| TypeError::new(format!(
                                "could not infer type parameter '{}'", tp_name
                            )))?;
                        let type_name = match concrete_type {
                            Type::Struct { name, .. } => name.clone(),
                            Type::Enum { name, .. } => name.clone(),
                            other => return Err(TypeError::new(format!(
                                "type {} cannot satisfy trait bound '{}'", other, trait_name
                            ))),
                        };
                        let trait_methods = traits.get(trait_name)
                            .ok_or_else(|| TypeError::new(format!("undefined trait '{}'", trait_name)))?;
                        for (method_name, _, _) in trait_methods {
                            if !methods.contains_key(&(type_name.clone(), method_name.clone())) {
                                return Err(TypeError::new(format!(
                                    "type '{}' does not implement trait '{}' (missing method '{}')",
                                    type_name, trait_name, method_name
                                )));
                            }
                        }
                    }
                }

                // Resolve return type
                let is_return_type_param = generic_info.type_params.iter().any(|(n, _)| n == &generic_info.return_type_name);
                let result_type = if is_return_type_param {
                    type_map.get(&generic_info.return_type_name)
                        .ok_or_else(|| TypeError::new(format!(
                            "could not infer return type parameter '{}'", generic_info.return_type_name
                        )))?
                        .clone()
                } else {
                    resolve_type(&generic_info.return_type_name, structs, enums, module_exports)?
                };

                return Ok(result_type);
            }

            // Check if the function name is a local variable holding a lambda/function
            if let Some((Type::Fn { params: fn_params, ret }, _)) = locals.get(function) {
                if args.len() != fn_params.len() {
                    return Err(TypeError::new(format!(
                        "wrong argument count calling '{}': expected {} argument(s) but got {}",
                        function, fn_params.len(), args.len()
                    )));
                }
                let fn_params = fn_params.clone();
                let ret = *ret.clone();
                for (i, (arg, expected)) in args.iter().zip(fn_params.iter()).enumerate() {
                    let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if actual != *expected {
                        return Err(TypeError::new(format!(
                            "argument {} to '{}': expected {} but got {}",
                            i + 1, function, expected, actual
                        )));
                    }
                }
                return Ok(ret);
            }

            Err(TypeError::new(format!("undefined function '{}'", function)))
        }

        Expr::StructLiteral { name, fields, .. } => {
            let expected_fields = structs
                .get(name)
                .ok_or_else(|| TypeError::new(format!("undefined struct '{}'", name)))?
                .clone();

            // Check for missing fields
            for (expected_name, _) in &expected_fields {
                if !fields.iter().any(|(f, _)| f == expected_name) {
                    return Err(TypeError::new(format!(
                        "missing field '{}' in struct '{}'", expected_name, name
                    )));
                }
            }

            // Check for unknown fields and type mismatches
            for (field_name, field_expr) in fields {
                let expected_type = expected_fields
                    .iter()
                    .find(|(n, _)| n == field_name)
                    .map(|(_, t)| t)
                    .ok_or_else(|| TypeError::new(format!(
                        "unknown field '{}' on struct '{}'", field_name, name
                    )))?;
                let actual_type = check_expr(field_expr, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if actual_type != *expected_type {
                    return Err(TypeError::new(format!(
                        "type mismatch for field '{}' of struct '{}': expected {} but got {}",
                        field_name, name, expected_type, actual_type
                    )));
                }
            }

            Ok(Type::Struct { name: name.clone(), fields: expected_fields })
        }

        Expr::FieldAccess { object, field, span } => {
            if let Expr::Identifier { name, .. } = object.as_ref() {
                if module_exports.contains_key(name) {
                    return Err(TypeError::new(format!(
                        "cannot access field on module '{}' — did you mean to call a function?", name
                    )));
                }
            }
            let obj_ty = check_expr(object, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            match &obj_ty {
                Type::Tuple { elements } => {
                    let index: usize = field.parse().map_err(|_| {
                        TypeError::new(format!("tuple access must use numeric index, got '{}'", field))
                    })?;
                    if index >= elements.len() {
                        return Err(TypeError::new(format!(
                            "tuple index {} out of bounds, tuple has {} elements", index, elements.len()
                        )));
                    }
                    Ok(elements[index].clone())
                }
                Type::Struct { name, fields } => {
                    fields
                        .iter()
                        .find(|(n, _)| n == field)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| TypeError::new(format!(
                            "no field '{}' on struct '{}'", field, name
                        )))
                }
                _ => {
                    // Try as no-arg method call on non-struct types
                    let synthetic = Expr::MethodCall {
                        object: object.clone(),
                        method: field.clone(),
                        args: vec![],
                        span: span.clone(),
                    };
                    check_expr(&synthetic, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)
                }
            }
        }

        Expr::EnumVariant { enum_name, variant_name, args, .. } => {
            let variants = enums
                .get(enum_name)
                .ok_or_else(|| TypeError::new(format!("undefined enum '{}'", enum_name)))?;
            let (_, expected_fields) = variants
                .iter()
                .find(|(n, _)| n == variant_name)
                .ok_or_else(|| TypeError::new(format!(
                    "unknown variant '{}' on enum '{}'", variant_name, enum_name
                )))?;
            if args.len() != expected_fields.len() {
                return Err(TypeError::new(format!(
                    "variant '{}::{}' expects {} argument(s) but got {}",
                    enum_name, variant_name, expected_fields.len(), args.len()
                )));
            }
            for (i, (arg, expected_ty)) in args.iter().zip(expected_fields.iter()).enumerate() {
                let actual_ty = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if actual_ty != *expected_ty {
                    return Err(TypeError::new(format!(
                        "argument {} to '{}::{}': expected {} but got {}",
                        i + 1, enum_name, variant_name, expected_ty, actual_ty
                    )));
                }
            }
            Ok(Type::Enum { name: enum_name.clone(), variants: variants.clone() })
        }

        Expr::Match { scrutinee, arms, .. } => {
            let scrutinee_ty = check_expr(scrutinee, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            let (enum_name, variants) = match &scrutinee_ty {
                Type::Enum { name, variants } => (name.clone(), variants.clone()),
                other => return Err(TypeError::new(format!(
                    "match scrutinee must be an enum type, got {}", other
                ))),
            };

            let mut result_type: Option<Type> = None;
            for arm in arms {
                let sans_parser::ast::Pattern::EnumVariant {
                    enum_name: pat_enum,
                    variant_name: pat_variant,
                    bindings,
                    ..
                } = &arm.pattern;

                if *pat_enum != enum_name {
                    return Err(TypeError::new(format!(
                        "pattern enum '{}' does not match scrutinee enum '{}'",
                        pat_enum, enum_name
                    )));
                }

                let (_, variant_fields) = variants
                    .iter()
                    .find(|(n, _)| n == pat_variant)
                    .ok_or_else(|| TypeError::new(format!(
                        "unknown variant '{}' on enum '{}'", pat_variant, enum_name
                    )))?;

                if bindings.len() != variant_fields.len() {
                    return Err(TypeError::new(format!(
                        "pattern '{}::{}' expects {} binding(s) but got {}",
                        enum_name, pat_variant, variant_fields.len(), bindings.len()
                    )));
                }

                // Create a local scope with bindings
                let mut arm_locals = locals.clone();
                for (binding_name, binding_ty) in bindings.iter().zip(variant_fields.iter()) {
                    arm_locals.insert(binding_name.clone(), (binding_ty.clone(), false));
                }

                let arm_ty = check_expr(&arm.body, &arm_locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;

                if let Some(ref expected) = result_type {
                    if arm_ty != *expected {
                        return Err(TypeError::new(format!(
                            "match arm type mismatch: expected {} but got {}",
                            expected, arm_ty
                        )));
                    }
                } else {
                    result_type = Some(arm_ty);
                }
            }

            result_type.ok_or_else(|| TypeError::new("match expression has no arms"))
        }

        Expr::Spawn { function, args, .. } => {
            if let Some((param_types, _)) = fn_env.get(function) {
                if args.len() != param_types.len() {
                    return Err(TypeError::new(format!(
                        "wrong argument count calling '{}': expected {} but got {}",
                        function, param_types.len(), args.len()
                    )));
                }
                for (i, (arg, expected)) in args.iter().zip(param_types.iter()).enumerate() {
                    let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    let compatible = actual == *expected
                        || (*expected == Type::Int && matches!(actual, Type::Sender { .. } | Type::Receiver { .. } | Type::JoinHandle | Type::Mutex { .. }));
                    if !compatible {
                        return Err(TypeError::new(format!(
                            "argument {} to '{}': expected {} but got {}",
                            i + 1, function, expected, actual
                        )));
                    }
                }
                Ok(Type::JoinHandle)
            } else {
                Err(TypeError::new(format!("undefined function '{}' in spawn", function)))
            }
        }

        Expr::ChannelCreate { element_type, capacity, .. } => {
            if let Some(cap_expr) = capacity {
                let cap_ty = check_expr(cap_expr, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if cap_ty != Type::Int {
                    return Err(TypeError::new(format!(
                        "channel capacity must be Int, got {}", cap_ty
                    )));
                }
            }
            let inner = resolve_type(&element_type.name, structs, enums, module_exports)?;
            Ok(Type::Sender { inner: Box::new(inner) })
        }

        Expr::MutexCreate { value, .. } => {
            let inner = check_expr(value, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            Ok(Type::Mutex { inner: Box::new(inner) })
        }

        Expr::ArrayCreate { element_type, .. } => {
            let inner = resolve_type(&element_type.name, structs, enums, module_exports)?;
            Ok(Type::Array { inner: Box::new(inner) })
        }

        Expr::ArrayLiteral { elements, .. } => {
            let first_ty = check_expr(&elements[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
            for (i, elem) in elements.iter().enumerate().skip(1) {
                let elem_ty = check_expr(elem, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if elem_ty != first_ty {
                    return Err(TypeError::new(format!(
                        "array literal element {} has type {} but expected {} (from first element)",
                        i, elem_ty, first_ty
                    )));
                }
            }
            Ok(Type::Array { inner: Box::new(first_ty) })
        }

        Expr::TupleLiteral { elements, .. } => {
            let mut elem_types = Vec::new();
            for elem in elements {
                elem_types.push(check_expr(elem, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?);
            }
            Ok(Type::Tuple { elements: elem_types })
        }

        Expr::MethodCall { object, method, args, .. } => {
            // Check if this is a cross-module function call: mod.func(args)
            if let Expr::Identifier { name, .. } = object.as_ref() {
                if let Some(mod_exports) = module_exports.get(name) {
                    let sig = mod_exports.functions.get(method)
                        .ok_or_else(|| TypeError::new(format!(
                            "function '{}' not found in module '{}'", method, name
                        )))?;
                    if args.len() != sig.params.len() {
                        return Err(TypeError::new(format!(
                            "function '{}' in module '{}' expects {} arguments, got {}",
                            method, name, sig.params.len(), args.len()
                        )));
                    }
                    for (i, (arg, expected)) in args.iter().zip(sig.params.iter()).enumerate() {
                        let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                        if actual != *expected {
                            return Err(TypeError::new(format!(
                                "argument {} to '{}.{}': expected {} but got {}",
                                i + 1, name, method, expected, actual
                            )));
                        }
                    }
                    return Ok(sig.return_type.clone());
                }
            }
            let obj_ty = check_expr(object, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;

            // Handle concurrency built-in methods
            match (&obj_ty, method.as_str()) {
                (Type::Sender { inner }, "send") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("send() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != **inner {
                        return Err(TypeError::new(format!(
                            "send() type mismatch: channel holds {} but got {}", inner, arg_ty
                        )));
                    }
                    return Ok(Type::Int);
                }
                (Type::Receiver { inner }, "recv") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("recv() takes no arguments"));
                    }
                    return Ok(*inner.clone());
                }
                (Type::JoinHandle, "join") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("join() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
                (Type::Mutex { inner }, "lock") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("lock() takes no arguments"));
                    }
                    return Ok(*inner.clone());
                }
                (Type::Mutex { inner }, "unlock") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("unlock() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != **inner {
                        return Err(TypeError::new(format!(
                            "unlock() type mismatch: mutex holds {} but got {}", inner, arg_ty
                        )));
                    }
                    return Ok(Type::Int);
                }
                (Type::Array { inner }, "push") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("push() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != **inner {
                        return Err(TypeError::new(format!(
                            "push() type mismatch: array holds {} but got {}", inner, arg_ty
                        )));
                    }
                    return Ok(Type::Int);
                }
                (Type::Array { inner }, "get") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("get() takes exactly 1 argument"));
                    }
                    let idx_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if idx_ty != Type::Int {
                        return Err(TypeError::new(format!("get() index must be Int, got {}", idx_ty)));
                    }
                    return Ok(*inner.clone());
                }
                (Type::Array { inner }, "set") => {
                    if args.len() != 2 {
                        return Err(TypeError::new("set() takes exactly 2 arguments (index, value)"));
                    }
                    let idx_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if idx_ty != Type::Int {
                        return Err(TypeError::new(format!("set() index must be Int, got {}", idx_ty)));
                    }
                    let val_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if val_ty != **inner {
                        return Err(TypeError::new(format!(
                            "set() type mismatch: array holds {} but got {}", inner, val_ty
                        )));
                    }
                    return Ok(Type::Int);
                }
                (Type::Array { .. }, "len") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("len() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
                (Type::Array { inner }, "pop") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("pop() takes no arguments"));
                    }
                    return Ok(*inner.clone());
                }
                (Type::Array { inner }, "remove") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("remove() takes exactly 1 argument (index)"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::Int {
                        return Err(TypeError::new(format!("remove() index must be Int, got {}", arg_ty)));
                    }
                    return Ok(*inner.clone());
                }
                (Type::Array { inner }, "map") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("map() takes exactly 1 argument (function)"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    match &arg_ty {
                        Type::Fn { params, ret } if params.len() == 1 && params[0] == **inner => {
                            return Ok(Type::Array { inner: Box::new(*ret.clone()) });
                        }
                        _ => {
                            return Err(TypeError::new(format!("map() requires a function ({}) -> T, got {}", inner, arg_ty)));
                        }
                    }
                }
                (Type::Array { inner }, "filter") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("filter() takes exactly 1 argument (function)"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    match &arg_ty {
                        Type::Fn { params, ret } if params.len() == 1 && params[0] == **inner && **ret == Type::Bool => {
                            return Ok(Type::Array { inner: inner.clone() });
                        }
                        _ => {
                            return Err(TypeError::new(format!("filter() requires a function ({}) -> Bool, got {}", inner, arg_ty)));
                        }
                    }
                }
                (Type::Array { inner }, "any") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("any() takes exactly 1 argument (function)"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    match &arg_ty {
                        Type::Fn { params, ret } if params.len() == 1 && params[0] == **inner && **ret == Type::Bool => {
                            return Ok(Type::Bool);
                        }
                        _ => {
                            return Err(TypeError::new(format!("any() requires a function ({}) -> Bool, got {}", inner, arg_ty)));
                        }
                    }
                }
                (Type::Array { inner }, "find") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("find() takes exactly 1 argument (function)"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    match &arg_ty {
                        Type::Fn { params, ret } if params.len() == 1 && params[0] == **inner && **ret == Type::Bool => {
                            return Ok(*inner.clone());
                        }
                        _ => {
                            return Err(TypeError::new(format!("find() requires a function ({}) -> Bool, got {}", inner, arg_ty)));
                        }
                    }
                }
                (Type::Array { inner }, "enumerate") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("enumerate() takes no arguments"));
                    }
                    return Ok(Type::Array {
                        inner: Box::new(Type::Tuple { elements: vec![Type::Int, *inner.clone()] }),
                    });
                }
                (Type::Array { inner: inner_a }, "zip") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("zip() takes exactly 1 argument (array)"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    match &arg_ty {
                        Type::Array { inner: inner_b } => {
                            return Ok(Type::Array {
                                inner: Box::new(Type::Tuple { elements: vec![*inner_a.clone(), *inner_b.clone()] }),
                            });
                        }
                        _ => {
                            return Err(TypeError::new(format!("zip() requires an array argument, got {}", arg_ty)));
                        }
                    }
                }
                (Type::Array { inner }, "contains") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("contains() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != **inner {
                        return Err(TypeError::new(format!("contains() argument must be {}, got {}", inner, arg_ty)));
                    }
                    return Ok(Type::Bool);
                }
                (Type::String, "len") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("len() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
                (Type::String, "substring") => {
                    if args.len() != 2 {
                        return Err(TypeError::new("substring() takes exactly 2 arguments (start, end)"));
                    }
                    let start_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if start_ty != Type::Int {
                        return Err(TypeError::new(format!("substring() start must be Int, got {}", start_ty)));
                    }
                    let end_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if end_ty != Type::Int {
                        return Err(TypeError::new(format!("substring() end must be Int, got {}", end_ty)));
                    }
                    return Ok(Type::String);
                }
                (Type::String, "trim") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("trim() takes no arguments"));
                    }
                    return Ok(Type::String);
                }
                (Type::String, "starts_with") | (Type::String, "sw") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("starts_with() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::String {
                        return Err(TypeError::new(format!("starts_with() requires String argument, got {}", arg_ty)));
                    }
                    return Ok(Type::Bool);
                }
                (Type::String, "ends_with") | (Type::String, "ew") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("ends_with() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::String {
                        return Err(TypeError::new(format!("ends_with() requires String argument, got {}", arg_ty)));
                    }
                    return Ok(Type::Bool);
                }
                (Type::String, "contains") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("contains() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::String {
                        return Err(TypeError::new(format!("contains() requires String argument, got {}", arg_ty)));
                    }
                    return Ok(Type::Bool);
                }
                (Type::String, "split") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("split() takes exactly 1 argument (delimiter)"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::String {
                        return Err(TypeError::new(format!("split() requires String delimiter, got {}", arg_ty)));
                    }
                    return Ok(Type::Array { inner: Box::new(Type::String) });
                }
                (Type::String, "replace") => {
                    if args.len() != 2 {
                        return Err(TypeError::new("replace() takes exactly 2 arguments (old, new)"));
                    }
                    let old_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if old_ty != Type::String {
                        return Err(TypeError::new(format!("replace() first argument must be String, got {}", old_ty)));
                    }
                    let new_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if new_ty != Type::String {
                        return Err(TypeError::new(format!("replace() second argument must be String, got {}", new_ty)));
                    }
                    return Ok(Type::String);
                }
                (Type::JsonValue, "get") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("get() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::String {
                        return Err(TypeError::new(format!("get() key must be String, got {}", arg_ty)));
                    }
                    return Ok(Type::JsonValue);
                }
                (Type::JsonValue, "get_index") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("get_index() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::Int {
                        return Err(TypeError::new(format!("get_index() index must be Int, got {}", arg_ty)));
                    }
                    return Ok(Type::JsonValue);
                }
                (Type::JsonValue, "get_string") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("get_string() takes no arguments"));
                    }
                    return Ok(Type::String);
                }
                (Type::JsonValue, "get_int") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("get_int() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
                (Type::JsonValue, "get_bool") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("get_bool() takes no arguments"));
                    }
                    return Ok(Type::Bool);
                }
                (Type::JsonValue, "len") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("len() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
                (Type::JsonValue, "type_of") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("type_of() takes no arguments"));
                    }
                    return Ok(Type::String);
                }
                (Type::JsonValue, "set") => {
                    if args.len() != 2 {
                        return Err(TypeError::new("set() takes exactly 2 arguments (key, value)"));
                    }
                    let key_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if key_ty != Type::String {
                        return Err(TypeError::new(format!("set() key must be String, got {}", key_ty)));
                    }
                    let val_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if val_ty != Type::JsonValue {
                        return Err(TypeError::new(format!("set() value must be JsonValue, got {}", val_ty)));
                    }
                    return Ok(Type::Int);
                }
                (Type::JsonValue, "push") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("push() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::JsonValue {
                        return Err(TypeError::new(format!("push() requires JsonValue argument, got {}", arg_ty)));
                    }
                    return Ok(Type::Int);
                }
                (Type::JsonValue, other) => {
                    return Err(TypeError::new(format!("JsonValue has no method '{}'", other)));
                }
                (Type::Map, "get") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("get() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::String {
                        return Err(TypeError::new(format!("get() key must be String, got {}", arg_ty)));
                    }
                    return Ok(Type::Int);
                }
                (Type::Map, "set") => {
                    if args.len() != 2 {
                        return Err(TypeError::new("set() takes exactly 2 arguments (key, value)"));
                    }
                    let key_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if key_ty != Type::String {
                        return Err(TypeError::new(format!("set() key must be String, got {}", key_ty)));
                    }
                    // Accept any value type — stored as i64 (pointer for heap types)
                    check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    return Ok(Type::Int);
                }
                (Type::Map, "has") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("has() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::String {
                        return Err(TypeError::new(format!("has() key must be String, got {}", arg_ty)));
                    }
                    return Ok(Type::Bool);
                }
                (Type::Map, "len") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("len() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
                (Type::Map, "keys") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("keys() takes no arguments"));
                    }
                    return Ok(Type::Array { inner: Box::new(Type::String) });
                }
                (Type::Map, "vals") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("vals() takes no arguments"));
                    }
                    return Ok(Type::Array { inner: Box::new(Type::Int) });
                }
                (Type::Map, other) => {
                    return Err(TypeError::new(format!("Map has no method '{}'", other)));
                }
                (Type::HttpResponse, "status") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("status() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
                (Type::HttpResponse, "body") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("body() takes no arguments"));
                    }
                    return Ok(Type::String);
                }
                (Type::HttpResponse, "header") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("header() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if arg_ty != Type::String {
                        return Err(TypeError::new(format!("header() name must be String, got {}", arg_ty)));
                    }
                    return Ok(Type::String);
                }
                (Type::HttpResponse, "ok") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("ok() takes no arguments"));
                    }
                    return Ok(Type::Bool);
                }
                (Type::HttpResponse, other) => {
                    return Err(TypeError::new(format!("HttpResponse has no method '{}'", other)));
                }
                (Type::HttpServer, "accept") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("accept() takes no arguments"));
                    }
                    return Ok(Type::HttpRequest);
                }
                (Type::HttpServer, other) => {
                    return Err(TypeError::new(format!("HttpServer has no method '{}'", other)));
                }
                (Type::HttpRequest, "path") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("path() takes no arguments"));
                    }
                    return Ok(Type::String);
                }
                (Type::HttpRequest, "method") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("method() takes no arguments"));
                    }
                    return Ok(Type::String);
                }
                (Type::HttpRequest, "body") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("body() takes no arguments"));
                    }
                    return Ok(Type::String);
                }
                (Type::HttpRequest, "respond") => {
                    if args.len() < 2 || args.len() > 3 {
                        return Err(TypeError::new("respond() takes 2 or 3 arguments (status, body[, content_type])"));
                    }
                    let status_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if status_ty != Type::Int {
                        return Err(TypeError::new(format!("respond() status must be Int, got {}", status_ty)));
                    }
                    let body_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if body_ty != Type::String {
                        return Err(TypeError::new(format!("respond() body must be String, got {}", body_ty)));
                    }
                    if args.len() == 3 {
                        let ct_ty = check_expr(&args[2], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                        if ct_ty != Type::String {
                            return Err(TypeError::new(format!("respond() content_type must be String, got {}", ct_ty)));
                        }
                    }
                    return Ok(Type::Int);
                }
                (Type::HttpRequest, other) => {
                    return Err(TypeError::new(format!("HttpRequest has no method '{}'", other)));
                }
                (Type::Result { .. } | Type::ResultErr, "is_ok") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("is_ok() takes no arguments"));
                    }
                    return Ok(Type::Bool);
                }
                (Type::Result { .. } | Type::ResultErr, "is_err") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("is_err() takes no arguments"));
                    }
                    return Ok(Type::Bool);
                }
                (Type::Result { inner }, "unwrap") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("unwrap() takes no arguments"));
                    }
                    return Ok(*inner.clone());
                }
                (Type::Result { inner }, "unwrap_or") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("unwrap_or() takes exactly 1 argument"));
                    }
                    let default_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                    if default_ty != **inner {
                        return Err(TypeError::new(format!("unwrap_or() default must be {}, got {}", inner, default_ty)));
                    }
                    return Ok(*inner.clone());
                }
                (Type::Result { .. } | Type::ResultErr, "error") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("error() takes no arguments"));
                    }
                    return Ok(Type::String);
                }
                (Type::Result { .. }, other) => {
                    return Err(TypeError::new(format!("Result has no method '{}'", other)));
                }
                _ => {}
            }

            let type_name = match &obj_ty {
                Type::Struct { name, .. } => name.clone(),
                Type::Enum { name, .. } => name.clone(),
                other => return Err(TypeError::new(format!(
                    "method call on non-struct/enum type {}", other
                ))),
            };
            let (param_types, call_ret_type) = methods
                .get(&(type_name.clone(), method.clone()))
                .ok_or_else(|| TypeError::new(format!(
                    "no method '{}' on type '{}'", method, type_name
                )))?;
            if args.len() != param_types.len() {
                return Err(TypeError::new(format!(
                    "method '{}' on '{}' expects {} argument(s) but got {}",
                    method, type_name, param_types.len(), args.len()
                )));
            }
            for (i, (arg, expected)) in args.iter().zip(param_types.iter()).enumerate() {
                let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if actual != *expected {
                    return Err(TypeError::new(format!(
                        "argument {} to method '{}': expected {} but got {}",
                        i + 1, method, expected, actual
                    )));
                }
            }
            Ok(call_ret_type.clone())
        }

        Expr::Lambda { params, return_type, body, .. } => {
            let mut param_types = Vec::new();
            let mut lambda_locals = locals.clone();
            for param in params {
                let ty = resolve_type(&param.type_name.name, structs, enums, module_exports)?;
                param_types.push(ty.clone());
                lambda_locals.insert(param.name.clone(), (ty, false));
            }
            let ret_ty = resolve_type(&return_type.name, structs, enums, module_exports)?;
            for stmt in body {
                check_stmt(stmt, &mut lambda_locals, fn_env, &ret_ty, structs, enums, methods, generic_fns, traits, module_exports)?;
            }
            Ok(Type::Fn { params: param_types, ret: Box::new(ret_ty) })
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn do_check(src: &str) -> Result<(), TypeError> {
        let prog = sans_parser::parse(src)
            .expect("parse error in test input");
        check(&prog, &HashMap::new())?;
        Ok(())
    }

    #[test]
    fn check_valid_minimal() {
        assert!(do_check("fn main() Int { 42 }").is_ok());
    }

    #[test]
    fn check_valid_let_binding() {
        assert!(do_check("fn main() Int { let x Int = 42 x }").is_ok());
    }

    #[test]
    fn check_valid_arithmetic() {
        assert!(do_check("fn main() Int { let x Int = 1 + 2 x }").is_ok());
    }

    #[test]
    fn check_valid_function_call() {
        assert!(do_check(
            "fn add(a Int, b Int) Int { a + b } fn main() Int { add(1, 2) }"
        ).is_ok());
    }

    #[test]
    fn check_undefined_variable() {
        let err = do_check("fn main() Int { x }").unwrap_err();
        assert!(
            err.message.contains("undefined variable"),
            "expected 'undefined variable' in error, got: {}",
            err.message
        );
    }

    #[test]
    fn check_undefined_function() {
        let err = do_check("fn main() Int { foo(1) }").unwrap_err();
        assert!(
            err.message.contains("undefined function"),
            "expected 'undefined function' in error, got: {}",
            err.message
        );
    }

    #[test]
    fn check_wrong_arg_count() {
        let err = do_check(
            "fn add(a Int, b Int) Int { a + b } fn main() Int { add(1) }"
        ).unwrap_err();
        assert!(
            err.message.contains("argument"),
            "expected 'argument' in error, got: {}",
            err.message
        );
    }

    #[test]
    fn check_requires_main() {
        let err = do_check("fn add(a Int, b Int) Int { a + b }").unwrap_err();
        assert!(
            err.message.contains("main"),
            "expected 'main' in error, got: {}",
            err.message
        );
    }

    #[test]
    fn check_missing_return_expression() {
        let err = do_check("fn main() Int { let x Int = 42 }").unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "expected 'missing return' in error, got: {}",
            err.message
        );
    }

    #[test]
    fn check_bool_literal() {
        assert!(do_check("fn main() Bool { true }").is_ok());
    }

    #[test]
    fn check_comparison_returns_bool() {
        assert!(do_check("fn main() Bool { 1 == 2 }").is_ok());
    }

    #[test]
    fn check_if_else_valid() {
        assert!(do_check("fn main() Int { if true { 1 } else { 2 } }").is_ok());
    }

    #[test]
    fn check_if_condition_must_be_bool() {
        let err = do_check("fn main() Int { if 1 { 1 } else { 2 } }").unwrap_err();
        assert!(err.message.contains("Bool"), "expected Bool error, got: {}", err.message);
    }

    #[test]
    fn check_if_branch_type_mismatch() {
        let err = do_check("fn main() Int { if true { 1 } else { true } }").unwrap_err();
        assert!(err.message.contains("mismatch"), "expected mismatch error, got: {}", err.message);
    }

    #[test]
    fn check_not_operator() {
        assert!(do_check("fn main() Bool { !true }").is_ok());
    }

    #[test]
    fn check_boolean_operators() {
        assert!(do_check("fn main() Bool { true && false }").is_ok());
        assert!(do_check("fn main() Bool { true || false }").is_ok());
    }

    #[test]
    fn check_comparison_in_if() {
        assert!(do_check("fn main() Int { let x Int = 5 if x > 3 { 1 } else { 0 } }").is_ok());
    }

    #[test]
    fn check_while_loop() {
        assert!(do_check("fn main() Int { let mut x Int = 0 while x < 10 { x = x + 1 } x }").is_ok());
    }

    #[test]
    fn check_while_condition_must_be_bool() {
        let err = do_check("fn main() Int { while 1 { 0 } 0 }").unwrap_err();
        assert!(err.message.contains("Bool"), "got: {}", err.message);
    }

    #[test]
    fn check_return_statement() {
        assert!(do_check("fn main() Int { return 42 }").is_ok());
    }

    #[test]
    fn check_return_type_mismatch() {
        let err = do_check("fn main() Int { return true }").unwrap_err();
        assert!(err.message.contains("mismatch"), "got: {}", err.message);
    }

    #[test]
    fn check_mutable_assignment() {
        assert!(do_check("fn main() Int { let mut x Int = 1 x = 2 x }").is_ok());
    }

    #[test]
    fn check_immutable_assignment_error() {
        let err = do_check("fn main() Int { let x Int = 1 x = 2 x }").unwrap_err();
        assert!(err.message.contains("immutable"), "got: {}", err.message);
    }

    #[test]
    fn check_assign_type_mismatch() {
        let err = do_check("fn main() Int { let mut x Int = 1 x = true x }").unwrap_err();
        assert!(err.message.contains("mismatch"), "got: {}", err.message);
    }

    #[test]
    fn check_inferred_int() {
        assert!(do_check("fn main() Int { let x = 42 x }").is_ok());
    }

    #[test]
    fn check_inferred_bool() {
        assert!(do_check("fn main() Bool { let x = true x }").is_ok());
    }

    #[test]
    fn check_inferred_mut() {
        assert!(do_check("fn main() Int { let mut x = 0 x = 42 x }").is_ok());
    }

    #[test]
    fn check_print_string() {
        assert!(do_check(r#"fn main() Int { print("hello") }"#).is_ok());
    }

    #[test]
    fn check_print_int() {
        assert!(do_check("fn main() Int { print(42) }").is_ok());
    }

    #[test]
    fn check_while_with_return() {
        assert!(do_check("fn main() Int { while true { return 42 } 0 }").is_ok());
    }

    #[test]
    fn check_enum_valid() {
        assert!(do_check(
            "enum Color { Red, Green, Blue, } fn main() Int { let c = Color::Green match c { Color::Red => 1, Color::Green => 2, Color::Blue => 3, } }"
        ).is_ok());
    }

    #[test]
    fn check_enum_with_data() {
        assert!(do_check(
            "enum Shape { Circle(Int), Rectangle(Int, Int), } fn main() Int { let s = Shape::Rectangle(3, 4) match s { Shape::Circle(r) => r, Shape::Rectangle(w, h) => w * h, } }"
        ).is_ok());
    }

    #[test]
    fn check_enum_undefined() {
        let err = do_check(
            "fn main() Int { let c = Foo::Bar 0 }"
        ).unwrap_err();
        assert!(err.message.contains("undefined enum"), "got: {}", err.message);
    }

    #[test]
    fn check_enum_wrong_variant() {
        let err = do_check(
            "enum Color { Red, Green, Blue, } fn main() Int { let c = Color::Yellow 0 }"
        ).unwrap_err();
        assert!(err.message.contains("unknown variant"), "got: {}", err.message);
    }

    #[test]
    fn check_match_arm_type_mismatch() {
        let err = do_check(
            "enum Color { Red, Green, } fn main() Int { let c = Color::Red match c { Color::Red => 1, Color::Green => true, } }"
        ).unwrap_err();
        assert!(err.message.contains("mismatch"), "got: {}", err.message);
    }

    #[test]
    fn check_struct_valid() {
        assert!(do_check(
            "struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 1, y: 2 } p.x }"
        ).is_ok());
    }

    #[test]
    fn check_struct_wrong_field_type() {
        let err = do_check(
            "struct Point { x Int, y Int, } fn main() Int { let p = Point { x: true, y: 2 } p.x }"
        ).unwrap_err();
        assert!(err.message.contains("mismatch"), "got: {}", err.message);
    }

    #[test]
    fn check_struct_unknown_field() {
        let err = do_check(
            "struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 1, y: 2 } p.z }"
        ).unwrap_err();
        assert!(err.message.contains("no field"), "got: {}", err.message);
    }

    #[test]
    fn check_struct_missing_field() {
        let err = do_check(
            "struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 1 } p.x }"
        ).unwrap_err();
        assert!(err.message.contains("missing field"), "got: {}", err.message);
    }

    #[test]
    fn check_struct_undefined() {
        let err = do_check(
            "fn main() Int { let p = Foo { x: 1 } 0 }"
        ).unwrap_err();
        assert!(err.message.contains("undefined struct"), "got: {}", err.message);
    }

    #[test]
    fn check_method_call_valid() {
        assert!(do_check(
            "struct Point { x Int, y Int, } impl Point { fn sum(self) Int { self.x + self.y } } fn main() Int { let p = Point { x: 3, y: 4 } p.sum() }"
        ).is_ok());
    }

    #[test]
    fn check_method_call_with_args() {
        assert!(do_check(
            "struct Point { x Int, y Int, } impl Point { fn add(self, n Int) Int { self.x + self.y + n } } fn main() Int { let p = Point { x: 3, y: 4 } p.add(10) }"
        ).is_ok());
    }

    #[test]
    fn check_method_undefined() {
        let err = do_check(
            "struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 1, y: 2 } p.foo() }"
        ).unwrap_err();
        assert!(err.message.contains("no method"), "got: {}", err.message);
    }

    #[test]
    fn check_method_wrong_args() {
        let err = do_check(
            "struct Point { x Int, y Int, } impl Point { fn sum(self) Int { self.x + self.y } } fn main() Int { let p = Point { x: 1, y: 2 } p.sum(1) }"
        ).unwrap_err();
        assert!(err.message.contains("expects"), "got: {}", err.message);
    }

    #[test]
    fn check_trait_impl_valid() {
        assert!(do_check(
            "trait Summable { fn sum(self) Int } struct Point { x Int, y Int, } impl Summable for Point { fn sum(self) Int { self.x + self.y } } fn main() Int { let p = Point { x: 3, y: 4 } p.sum() }"
        ).is_ok());
    }

    #[test]
    fn check_trait_missing_method() {
        let err = do_check(
            "trait Summable { fn sum(self) Int } struct Point { x Int, y Int, } impl Summable for Point { } fn main() Int { 0 }"
        ).unwrap_err();
        assert!(err.message.contains("missing method"), "got: {}", err.message);
    }

    #[test]
    fn check_trait_undefined() {
        let err = do_check(
            "struct Point { x Int, y Int, } impl Foo for Point { fn bar(self) Int { 0 } } fn main() Int { 0 }"
        ).unwrap_err();
        assert!(err.message.contains("undefined trait"), "got: {}", err.message);
    }

    #[test]
    fn check_generic_identity() {
        assert!(do_check(
            "fn identity<T>(x T) T { x } fn main() Int { identity(42) }"
        ).is_ok());
    }

    #[test]
    fn check_generic_identity_bool() {
        assert!(do_check(
            "fn identity<T>(x T) T { x } fn main() Bool { identity(true) }"
        ).is_ok());
    }

    #[test]
    fn check_generic_with_trait_bound() {
        assert!(do_check(
            "trait Summable { fn sum(self) Int } struct Point { x Int, y Int, } impl Summable for Point { fn sum(self) Int { self.x + self.y } } fn get_sum<T: Summable>(x T) Int { x.sum() } fn main() Int { let p = Point { x: 3, y: 4 } get_sum(p) }"
        ).is_ok());
    }

    #[test]
    fn check_generic_bound_not_satisfied() {
        let err = do_check(
            "trait Summable { fn sum(self) Int } fn get_sum<T: Summable>(x T) Int { x.sum() } fn main() Int { get_sum(42) }"
        ).unwrap_err();
        assert!(err.message.contains("cannot satisfy") || err.message.contains("does not implement"), "got: {}", err.message);
    }

    #[test]
    fn check_generic_type_mismatch() {
        let err = do_check(
            "fn same<T>(a T, b T) T { a } fn main() Int { same(42, true) }"
        ).unwrap_err();
        assert!(err.message.contains("inferred") || err.message.contains("mismatch"), "got: {}", err.message);
    }

    #[test]
    fn check_generic_wrong_arg_count() {
        let err = do_check(
            "fn identity<T>(x T) T { x } fn main() Int { identity(1, 2) }"
        ).unwrap_err();
        assert!(err.message.contains("argument"), "got: {}", err.message);
    }

    #[test]
    fn typeck_spawn_produces_join_handle() {
        let program = sans_parser::parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() 0 }").unwrap();
        assert!(check(&program, &HashMap::new()).is_ok());
    }

    #[test]
    fn typeck_spawn_wrong_args() {
        let program = sans_parser::parse("fn worker(x Int) Int { x } fn main() Int { let h = spawn worker() 0 }").unwrap();
        assert!(check(&program, &HashMap::new()).is_err());
    }

    #[test]
    fn typeck_channel_creates_sender_receiver() {
        let program = sans_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }").unwrap();
        assert!(check(&program, &HashMap::new()).is_ok());
    }

    #[test]
    fn typeck_send_type_mismatch() {
        let program = sans_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(true) 0 }").unwrap();
        let err = check(&program, &HashMap::new()).unwrap_err();
        assert!(err.message.contains("mismatch"), "got: {}", err.message);
    }

    #[test]
    fn typeck_recv_returns_element_type() {
        let program = sans_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }").unwrap();
        assert!(check(&program, &HashMap::new()).is_ok());
    }

    #[test]
    fn typeck_join_on_handle() {
        let program = sans_parser::parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() h.join() }").unwrap();
        assert!(check(&program, &HashMap::new()).is_ok());
    }

    #[test]
    fn typeck_join_on_non_handle() {
        let program = sans_parser::parse("fn main() Int { let x Int = 42 x.join() }").unwrap();
        assert!(check(&program, &HashMap::new()).is_err());
    }

    #[test]
    fn typeck_send_on_non_sender() {
        let program = sans_parser::parse("fn main() Int { let x Int = 42 x.send(1) 0 }").unwrap();
        assert!(check(&program, &HashMap::new()).is_err());
    }

    #[test]
    fn check_mutex_create() {
        assert!(do_check("fn main() Int { let m = mutex(0) 0 }").is_ok());
    }

    #[test]
    fn check_mutex_lock_returns_inner_type() {
        assert!(do_check("fn main() Int { let m = mutex(42) let v = m.lock() v }").is_ok());
    }

    #[test]
    fn check_mutex_unlock_matching_type() {
        assert!(do_check("fn main() Int { let m = mutex(0) let v = m.lock() m.unlock(v + 1) 0 }").is_ok());
    }

    #[test]
    fn check_mutex_unlock_wrong_type() {
        let err = do_check("fn main() Int { let m = mutex(0) m.unlock(true) 0 }").unwrap_err();
        assert!(err.message.contains("mismatch") || err.message.contains("type"),
            "expected type error, got: {}", err.message);
    }

    #[test]
    fn check_lock_on_non_mutex() {
        let err = do_check("fn main() Int { let x = 42 x.lock() }").unwrap_err();
        assert!(err.message.contains("method") || err.message.contains("lock"),
            "expected method error, got: {}", err.message);
    }

    #[test]
    fn check_bounded_channel_capacity_non_int() {
        let err = do_check("fn main() Int { let (tx, rx) = channel<Int>(true) 0 }").unwrap_err();
        assert!(err.message.contains("Int") || err.message.contains("capacity"),
            "expected capacity type error, got: {}", err.message);
    }

    #[test]
    fn check_array_create() {
        assert!(do_check("fn main() Int { let a = array<Int>() 0 }").is_ok());
    }

    #[test]
    fn check_array_push_matching_type() {
        assert!(do_check("fn main() Int { let a = array<Int>() a.push(1) 0 }").is_ok());
    }

    #[test]
    fn check_array_push_wrong_type() {
        let err = do_check("fn main() Int { let a = array<Int>() a.push(true) 0 }").unwrap_err();
        assert!(err.message.contains("mismatch") || err.message.contains("type"),
            "expected type error, got: {}", err.message);
    }

    #[test]
    fn check_array_get_returns_element_type() {
        assert!(do_check("fn main() Int { let a = array<Int>() a.push(1) a.get(0) }").is_ok());
    }

    #[test]
    fn check_array_len_returns_int() {
        assert!(do_check("fn main() Int { let a = array<Int>() a.len() }").is_ok());
    }

    #[test]
    fn check_for_in_binds_element_type() {
        assert!(do_check("fn main() Int { let a = array<Int>() a.push(1) for x in a { print(x) } 0 }").is_ok());
    }

    #[test]
    fn check_for_in_non_array_error() {
        let err = do_check("fn main() Int { for x in 42 { print(x) } 0 }").unwrap_err();
        assert!(err.message.contains("Array") || err.message.contains("for"),
            "expected for-in type error, got: {}", err.message);
    }

    #[test]
    fn check_string_len() {
        assert!(do_check(r#"fn main() Int { let s = "hello" s.len() }"#).is_ok());
    }

    #[test]
    fn check_string_concat() {
        assert!(do_check(r#"fn main() Int { let s = "a" + "b" 0 }"#).is_ok());
    }

    #[test]
    fn check_string_plus_int_error() {
        let err = do_check(r#"fn main() Int { let s = "a" + 1 0 }"#).unwrap_err();
        assert!(err.message.contains("type") || err.message.contains("mismatch") || err.message.contains("operand"),
            "expected type error, got: {}", err.message);
    }

    #[test]
    fn check_int_to_string() {
        assert!(do_check(r#"fn main() Int { let s = int_to_string(42) 0 }"#).is_ok());
    }

    #[test]
    fn check_string_to_int() {
        assert!(do_check(r#"fn main() Int { string_to_int("42") }"#).is_ok());
    }

    #[test]
    fn check_cross_module_function_call() {
        let prog = sans_parser::parse(
            "fn main() Int { utils.add(1, 2) }"
        ).expect("parse error");

        let mut module_exports = HashMap::new();
        let mut utils_fns = HashMap::new();
        utils_fns.insert("add".to_string(), FunctionSignature {
            params: vec![Type::Int, Type::Int],
            return_type: Type::Int,
        });
        module_exports.insert("utils".to_string(), ModuleExports {
            functions: utils_fns,
            structs: HashMap::new(),
            enums: HashMap::new(),
        });

        assert!(check(&prog, &module_exports).is_ok());
    }

    #[test]
    fn check_cross_module_function_with_struct_return() {
        let prog = sans_parser::parse(
            "fn main() Int { let u = models.create() u.age }"
        ).expect("parse error");

        let mut module_exports = HashMap::new();
        let user_fields = vec![
            ("name".to_string(), Type::String),
            ("age".to_string(), Type::Int),
        ];
        let mut models_fns = HashMap::new();
        models_fns.insert("create".to_string(), FunctionSignature {
            params: vec![],
            return_type: Type::Struct {
                name: "User".to_string(),
                fields: user_fields.clone(),
            },
        });
        let mut models_structs = HashMap::new();
        models_structs.insert("User".to_string(), user_fields);
        module_exports.insert("models".to_string(), ModuleExports {
            functions: models_fns,
            structs: models_structs,
            enums: HashMap::new(),
        });

        assert!(check(&prog, &module_exports).is_ok());
    }

    #[test]
    fn check_unknown_function_in_module() {
        let prog = sans_parser::parse(
            "fn main() Int { utils.nonexistent() }"
        ).expect("parse error");

        let mut module_exports = HashMap::new();
        module_exports.insert("utils".to_string(), ModuleExports {
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
        });

        let err = check(&prog, &module_exports).unwrap_err();
        assert!(err.message.contains("not found in module"),
            "expected module function error, got: {}", err.message);
    }

    #[test]
    fn check_unknown_module_prefix() {
        let prog = sans_parser::parse(
            "fn main() Int { nomod.func() }"
        ).expect("parse error");

        let err = check(&prog, &HashMap::new()).unwrap_err();
        assert!(err.message.contains("undefined") || err.message.contains("no method"),
            "expected undefined/no method error, got: {}", err.message);
    }

    #[test]
    fn check_field_access_on_module_errors() {
        let prog = sans_parser::parse(
            "fn main() Int { utils.x }"
        ).expect("parse error");

        let mut module_exports = HashMap::new();
        module_exports.insert("utils".to_string(), ModuleExports {
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
        });

        let err = check(&prog, &module_exports).unwrap_err();
        assert!(err.message.contains("cannot access field on module"),
            "expected module field access error, got: {}", err.message);
    }

    #[test]
    fn check_duplicate_import_is_ok() {
        let prog = sans_parser::parse(
            "fn main() Int { utils.add(1, 2) }"
        ).expect("parse error");

        let mut module_exports = HashMap::new();
        let mut utils_fns = HashMap::new();
        utils_fns.insert("add".to_string(), FunctionSignature {
            params: vec![Type::Int, Type::Int],
            return_type: Type::Int,
        });
        module_exports.insert("utils".to_string(), ModuleExports {
            functions: utils_fns,
            structs: HashMap::new(),
            enums: HashMap::new(),
        });

        assert!(check(&prog, &module_exports).is_ok());
    }

    #[test]
    fn check_file_read_builtin() {
        do_check("fn main() Int { let s = file_read(\"test.txt\") 0 }").unwrap();
    }

    #[test]
    fn check_file_write_builtin() {
        do_check("fn main() Int { file_write(\"test.txt\", \"hello\") }").unwrap();
    }

    #[test]
    fn check_file_exists_builtin() {
        do_check("fn main() Int { if file_exists(\"test.txt\") { 1 } else { 0 } }").unwrap();
    }

    #[test]
    fn check_file_read_wrong_type() {
        let err = do_check("fn main() Int { let s = file_read(42) 0 }").unwrap_err();
        assert!(err.message.contains("String"),
            "expected type error mentioning String, got: {}", err.message);
    }

    #[test]
    fn check_json_parse_returns_json_value() {
        assert!(do_check("fn main() Int { let v = json_parse(\"{}\") \n 0 }").is_ok());
    }

    #[test]
    fn check_json_object_returns_json_value() {
        assert!(do_check("fn main() Int { let v = json_object() \n 0 }").is_ok());
    }

    #[test]
    fn check_json_array_returns_json_value() {
        assert!(do_check("fn main() Int { let v = json_array() \n 0 }").is_ok());
    }

    #[test]
    fn check_json_string_accepts_string() {
        assert!(do_check("fn main() Int { let v = json_string(\"hi\") \n 0 }").is_ok());
    }

    #[test]
    fn check_json_int_accepts_int() {
        assert!(do_check("fn main() Int { let v = json_int(42) \n 0 }").is_ok());
    }

    #[test]
    fn check_json_bool_accepts_bool() {
        assert!(do_check("fn main() Int { let v = json_bool(true) \n 0 }").is_ok());
    }

    #[test]
    fn check_json_null_no_args() {
        assert!(do_check("fn main() Int { let v = json_null() \n 0 }").is_ok());
    }

    #[test]
    fn check_json_stringify_accepts_json_value() {
        assert!(do_check("fn main() Int { let v = json_object() \n let s = json_stringify(v) \n 0 }").is_ok());
    }

    #[test]
    fn check_json_parse_rejects_int() {
        let err = do_check("fn main() Int { let v = json_parse(42) \n 0 }").unwrap_err();
        assert!(err.message.contains("String"), "expected String error, got: {}", err.message);
    }

    #[test]
    fn check_json_stringify_rejects_string() {
        let err = do_check("fn main() Int { let s = json_stringify(\"hello\") \n 0 }").unwrap_err();
        assert!(err.message.contains("JsonValue"), "expected JsonValue error, got: {}", err.message);
    }

    #[test]
    fn check_json_get_method() {
        assert!(do_check("fn main() Int { let v = json_parse(\"{}\") \n let inner = v.get(\"key\") \n 0 }").is_ok());
    }

    #[test]
    fn check_json_get_index_method() {
        assert!(do_check("fn main() Int { let v = json_array() \n let inner = v.get_index(0) \n 0 }").is_ok());
    }

    #[test]
    fn check_json_get_string_method() {
        assert!(do_check("fn main() Int { let v = json_string(\"hi\") \n let s = v.get_string() \n 0 }").is_ok());
    }

    #[test]
    fn check_json_get_int_method() {
        assert!(do_check("fn main() Int { let v = json_int(5) \n let n = v.get_int() \n 0 }").is_ok());
    }

    #[test]
    fn check_json_get_bool_method() {
        assert!(do_check("fn main() Int { let v = json_bool(true) \n let b = v.get_bool() \n 0 }").is_ok());
    }

    #[test]
    fn check_json_len_method() {
        assert!(do_check("fn main() Int { let v = json_array() \n v.len() }").is_ok());
    }

    #[test]
    fn check_json_type_of_method() {
        assert!(do_check("fn main() Int { let v = json_null() \n let t = v.type_of() \n 0 }").is_ok());
    }

    #[test]
    fn check_json_set_method() {
        assert!(do_check("fn main() Int { let obj = json_object() \n let v = json_int(1) \n obj.set(\"k\", v) \n 0 }").is_ok());
    }

    #[test]
    fn check_json_push_method() {
        assert!(do_check("fn main() Int { let arr = json_array() \n let v = json_int(1) \n arr.push(v) \n 0 }").is_ok());
    }

    #[test]
    fn check_http_get_returns_http_response() {
        assert!(do_check("fn main() Int { let r = http_get(\"http://example.com\") \n r.status() }").is_ok());
    }

    #[test]
    fn check_http_post_returns_http_response() {
        assert!(do_check("fn main() Int { let r = http_post(\"http://example.com\", \"body\", \"text/plain\") \n r.status() }").is_ok());
    }

    #[test]
    fn check_http_response_methods() {
        assert!(do_check("fn main() Int { let r = http_get(\"http://example.com\") \n let s = r.status() \n let b = r.body() \n let h = r.header(\"content-type\") \n 0 }").is_ok());
    }

    #[test]
    fn check_http_response_ok_method() {
        assert!(do_check("fn main() Int { let r = http_get(\"http://example.com\") \n if r.ok() { 1 } else { 0 } }").is_ok());
    }

    #[test]
    fn check_http_get_wrong_arg_type() {
        let err = do_check("fn main() Int { let r = http_get(42) \n 0 }").unwrap_err();
        assert!(err.message.contains("String"), "expected String error, got: {}", err.message);
    }

    #[test]
    fn check_log_info_accepts_string() {
        assert!(do_check("fn main() Int { log_info(\"hello\") }").is_ok());
    }

    #[test]
    fn check_log_set_level_accepts_int() {
        assert!(do_check("fn main() Int { log_set_level(2) }").is_ok());
    }

    #[test]
    fn check_log_info_rejects_int() {
        let err = do_check("fn main() Int { log_info(42) }").unwrap_err();
        assert!(err.message.contains("String"), "expected String error, got: {}", err.message);
    }

    #[test]
    fn check_ok_returns_result() {
        assert!(do_check("fn main() Int { let r = ok(42) \n r.unwrap() }").is_ok());
    }

    #[test]
    fn check_err_returns_result() {
        assert!(do_check("fn main() Int { let r = err(\"bad\") \n 0 }").is_ok());
    }

    #[test]
    fn check_result_is_ok_method() {
        assert!(do_check("fn main() Int { let r = ok(42) \n if r.is_ok() { 1 } else { 0 } }").is_ok());
    }

    #[test]
    fn check_result_unwrap_or() {
        assert!(do_check("fn main() Int { let r = ok(42) \n r.unwrap_or(0) }").is_ok());
    }

    #[test]
    fn check_result_error_method() {
        assert!(do_check("fn main() Int { let r = err(\"oops\") \n let msg = r.error() \n 0 }").is_ok());
    }

    #[test]
    fn check_result_return_type() {
        assert!(do_check("fn divide(a Int, b Int) Result<Int> { if b == 0 { err(\"div by zero\") } else { ok(a / b) } } fn main() Int { let r = divide(10, 2) \n r.unwrap() }").is_ok());
    }

    #[test]
    fn check_err_wrong_arg_type() {
        let err = do_check("fn main() Int { let r = err(42) \n 0 }").unwrap_err();
        assert!(err.message.contains("String"), "expected String error, got: {}", err.message);
    }

    #[test]
    fn check_result_if_else_compat() {
        // err() in then, ok() in else — should be compatible
        assert!(do_check("fn main() Int { let r = if true { err(\"bad\") } else { ok(42) } \n 0 }").is_ok());
    }

    #[test]
    fn check_float_literal() {
        assert!(do_check("fn main() Float { 3.14 }").is_ok());
    }

    #[test]
    fn check_float_arithmetic() {
        assert!(do_check("fn main() Float { let a = 1.5 \n let b = 2.5 \n a + b }").is_ok());
    }

    #[test]
    fn check_float_comparison() {
        assert!(do_check("fn main() Int { let a = 1.5 \n let b = 2.5 \n if a < b { 1 } else { 0 } }").is_ok());
    }

    #[test]
    fn check_int_to_float() {
        assert!(do_check("fn main() Float { int_to_float(42) }").is_ok());
    }

    #[test]
    fn check_float_to_int() {
        assert!(do_check("fn main() Int { float_to_int(3.14) }").is_ok());
    }

    #[test]
    fn check_float_to_string() {
        assert!(do_check("fn main() Int { let s = float_to_string(3.14) \n 0 }").is_ok());
    }

    #[test]
    fn check_string_trim() {
        assert!(do_check("fn main() Int { let s = \"hello\".trim() \n 0 }").is_ok());
    }

    #[test]
    fn check_string_starts_with() {
        assert!(do_check("fn main() Int { if \"hello\".starts_with(\"he\") { 1 } else { 0 } }").is_ok());
    }

    #[test]
    fn check_string_ends_with() {
        assert!(do_check("fn main() Int { if \"hello\".ends_with(\"lo\") { 1 } else { 0 } }").is_ok());
    }

    #[test]
    fn check_string_contains() {
        assert!(do_check("fn main() Int { if \"hello\".contains(\"ll\") { 1 } else { 0 } }").is_ok());
    }

    #[test]
    fn check_string_split() {
        assert!(do_check("fn main() Int { let parts = \"a,b,c\".split(\",\") \n parts.len() }").is_ok());
    }

    #[test]
    fn check_array_pop() {
        assert!(do_check("fn main() Int { let a = array<Int>() \n a.push(1) \n a.pop() }").is_ok());
    }

    #[test]
    fn check_array_contains() {
        assert!(do_check("fn main() Int { let a = array<Int>() \n a.push(1) \n if a.contains(1) { 1 } else { 0 } }").is_ok());
    }

    #[test]
    fn check_string_replace() {
        assert!(do_check("fn main() Int { let s = \"hello world\".replace(\"world\", \"there\") \n 0 }").is_ok());
    }

    #[test]
    fn check_array_remove() {
        assert!(do_check("fn main() Int { let a = array<Int>() \n a.push(1) \n a.push(2) \n a.remove(0) }").is_ok());
    }

    #[test]
    fn check_http_listen() {
        assert!(do_check("fn main() Int { let s = http_listen(8080) \n 0 }").is_ok());
    }

    #[test]
    fn check_http_server_accept() {
        assert!(do_check("fn main() Int { let s = http_listen(8080) \n let r = s.accept() \n 0 }").is_ok());
    }

    #[test]
    fn check_http_request_methods() {
        assert!(do_check("fn main() Int { let s = http_listen(8080) \n let r = s.accept() \n let p = r.path() \n let m = r.method() \n r.respond(200, \"ok\") }").is_ok());
    }

    #[test]
    fn check_array_literal() {
        assert!(do_check("fn main() Int { let a = [1, 2, 3] \n a.len() }").is_ok());
    }

    #[test]
    fn check_array_literal_type_mismatch() {
        let err = do_check(r#"fn main() Int { let a = [1, "hello"]
 a.len() }"#).unwrap_err();
        assert!(err.message.contains("array literal element"));
    }

    // Modulo operator tests
    #[test]
    fn check_modulo_int() {
        assert!(do_check("fn main() Int { 17 % 5 }").is_ok());
    }

    #[test]
    fn check_modulo_float() {
        assert!(do_check("fn main() Float { 3.14 % 1.0 }").is_ok());
    }

    #[test]
    fn check_modulo_type_mismatch() {
        assert!(do_check("fn main() Int { true % 5 }").is_err());
    }

    // Unary negation tests
    #[test]
    fn check_neg_int() {
        assert!(do_check("fn main() Int { -42 }").is_ok());
    }

    #[test]
    fn check_neg_float() {
        assert!(do_check("fn main() Float { -3.14 }").is_ok());
    }

    #[test]
    fn check_neg_bool_error() {
        assert!(do_check("fn main() Int { -true }").is_err());
    }

    #[test]
    fn check_neg_in_expr() {
        assert!(do_check("fn main() Int { let x = 10 \n x + -3 }").is_ok());
    }

    // Array literal tests
    #[test]
    fn check_array_literal_string() {
        assert!(do_check("fn main() Int { let a = [\"a\", \"b\"] \n a.len() }").is_ok());
    }

    // String interpolation tests
    #[test]
    fn check_string_interp() {
        assert!(do_check("fn main() Int { let name = \"world\" \n let s = \"Hello {name}!\" \n s.len() }").is_ok());
    }

    #[test]
    fn check_string_interp_type_error() {
        assert!(do_check("fn main() Int { let x = 42 \n let s = \"value: {x}\" \n s.len() }").is_err());
    }

    // Multiline string test
    #[test]
    fn check_multiline_string() {
        assert!(do_check("fn main() Int { let s = \"\"\"\nhello\nworld\n\"\"\" \n s.len() }").is_ok());
    }

    #[test]
    fn check_tuple_literal() {
        let src = "main() I { t = (1 2 3)\n t.0 }";
        let program = sans_parser::parse(src).unwrap();
        let result = check(&program, &std::collections::HashMap::new());
        assert!(result.is_ok());
    }

    #[test]
    fn check_tuple_access_out_of_bounds() {
        let src = "main() I { t = (1 2)\n t.5 }";
        let program = sans_parser::parse(src).unwrap();
        let result = check(&program, &std::collections::HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn check_lambda_basic() {
        let src = "main() I { f = |x:I| I { x + 10 }\n 0 }";
        let program = sans_parser::parse(src).unwrap();
        let result = check(&program, &std::collections::HashMap::new());
        assert!(result.is_ok());
    }

    #[test]
    fn check_lambda_capture() {
        let src = "main() I { offset = 10\n f = |x:I| I { x + offset }\n 0 }";
        let program = sans_parser::parse(src).unwrap();
        let result = check(&program, &std::collections::HashMap::new());
        assert!(result.is_ok());
    }

    #[test]
    fn check_lambda_type_error() {
        let src = "main() I { f = |x:S| I { x + 10 }\n 0 }";
        let program = sans_parser::parse(src).unwrap();
        let result = check(&program, &std::collections::HashMap::new());
        assert!(result.is_err()); // Can't add String + Int
    }
}
