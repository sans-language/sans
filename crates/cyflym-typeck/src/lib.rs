pub mod types;

use std::collections::HashMap;
use cyflym_parser::ast::{Expr, Program, Stmt};
use types::Type;

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

/// Resolve an AST type name string to a `Type`.
fn resolve_type(
    name: &str,
    structs: &HashMap<String, Vec<(String, Type)>>,
    enums: &HashMap<String, Vec<(String, Vec<Type>)>>,
) -> Result<Type, TypeError> {
    match name {
        "Int" => Ok(Type::Int),
        "Bool" => Ok(Type::Bool),
        "String" => Ok(Type::String),
        other => {
            if let Some(fields) = structs.get(other) {
                Ok(Type::Struct { name: other.to_string(), fields: fields.clone() })
            } else if let Some(variants) = enums.get(other) {
                Ok(Type::Enum { name: other.to_string(), variants: variants.clone() })
            } else {
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
) -> Result<(), TypeError> {
    for stmt in stmts {
        check_stmt(stmt, locals, fn_env, ret_type, structs, enums, methods)?;
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
) -> Result<(), TypeError> {
    match stmt {
        Stmt::Let { name, mutable, type_name, value, .. } => {
            let actual = check_expr(value, locals, fn_env, ret_type, structs, enums, methods)?;
            let ty = if let Some(tn) = type_name {
                let declared = resolve_type(&tn.name, structs, enums)?;
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
            check_expr(expr, locals, fn_env, ret_type, structs, enums, methods)?;
            Ok(())
        }
        Stmt::While { condition, body, .. } => {
            let cond_ty = check_expr(condition, locals, fn_env, ret_type, structs, enums, methods)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "while condition must be Bool, got {}", cond_ty
                )));
            }
            check_stmts(body, locals, fn_env, ret_type, structs, enums, methods)?;
            Ok(())
        }
        Stmt::Return { value, .. } => {
            let ty = check_expr(value, locals, fn_env, ret_type, structs, enums, methods)?;
            if ty != *ret_type {
                return Err(TypeError::new(format!(
                    "return type mismatch: expected {} but got {}",
                    ret_type, ty
                )));
            }
            Ok(())
        }
        Stmt::Assign { name, value, .. } => {
            let (expected_ty, is_mutable) = locals
                .get(name)
                .ok_or_else(|| TypeError::new(format!("undefined variable '{}'", name)))?
                .clone();
            if !is_mutable {
                return Err(TypeError::new(format!(
                    "cannot assign to immutable variable '{}'", name
                )));
            }
            let actual = check_expr(value, locals, fn_env, ret_type, structs, enums, methods)?;
            if actual != expected_ty {
                return Err(TypeError::new(format!(
                    "type mismatch in assignment to '{}': expected {} but got {}",
                    name, expected_ty, actual
                )));
            }
            Ok(())
        }
        Stmt::If { condition, body, .. } => {
            let cond_ty = check_expr(condition, locals, fn_env, ret_type, structs, enums, methods)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "if condition must be Bool, got {}", cond_ty
                )));
            }
            check_stmts(body, locals, fn_env, ret_type, structs, enums, methods)?;
            Ok(())
        }
    }
}

/// Type-check the given `Program`. Returns `Ok(())` if the program is
/// well-typed, or a `TypeError` describing the first problem found.
pub fn check(program: &Program) -> Result<(), TypeError> {
    // Pass 0a: collect struct definitions.
    let mut struct_registry: HashMap<String, Vec<(String, Type)>> = HashMap::new();
    let enum_registry: HashMap<String, Vec<(String, Vec<Type>)>> = HashMap::new();
    for s in &program.structs {
        let mut fields = Vec::new();
        for f in &s.fields {
            let ty = resolve_type(&f.type_name.name, &struct_registry, &enum_registry)?;
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
                let ty = resolve_type(&f.name, &struct_registry, &enum_registry)?;
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
                .map(|p| resolve_type(&p.type_name.name, &struct_registry, &enum_registry))
                .collect::<Result<_, _>>()?;
            let ret_type = resolve_type(&m.return_type.name, &struct_registry, &enum_registry)?;
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
                    .map(|p| resolve_type(&p.type_name.name, &struct_registry, &enum_registry))
                    .collect::<Result<_, _>>()?;
                if actual_params != *expected_params {
                    return Err(TypeError::new(format!(
                        "method '{}' params don't match trait '{}' signature",
                        method_name, trait_name
                    )));
                }
                let actual_ret = resolve_type(&method.return_type.name, &struct_registry, &enum_registry)?;
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
                .map(|p| resolve_type(&p.type_name.name, &struct_registry, &enum_registry))
                .collect::<Result<_, _>>()?;
            let ret_type = resolve_type(&method.return_type.name, &struct_registry, &enum_registry)?;
            method_registry.insert(
                (imp.target_type.clone(), method.name.clone()),
                (param_types, ret_type),
            );
        }
    }

    // Pass 1: collect all function signatures into an environment.
    let mut fn_env: HashMap<String, (Vec<Type>, Type)> = HashMap::new();

    for func in &program.functions {
        let mut param_types = Vec::new();
        for param in &func.params {
            param_types.push(resolve_type(&param.type_name.name, &struct_registry, &enum_registry)?);
        }
        let ret_type = resolve_type(&func.return_type.name, &struct_registry, &enum_registry)?;
        fn_env.insert(func.name.clone(), (param_types, ret_type));
    }

    // Add mangled method signatures to fn_env for IR/codegen
    for imp in &program.impls {
        for method in &imp.methods {
            let mangled = format!("{}_{}", imp.target_type, method.name);
            let mut param_types = Vec::new();
            for p in &method.params {
                param_types.push(resolve_type(&p.type_name.name, &struct_registry, &enum_registry)?);
            }
            let ret_type = resolve_type(&method.return_type.name, &struct_registry, &enum_registry)?;
            fn_env.insert(mangled, (param_types, ret_type));
        }
    }

    // Require a `main` function.
    if !fn_env.contains_key("main") {
        return Err(TypeError::new("missing 'main' function"));
    }

    // Pass 2: type-check each function body.
    for func in &program.functions {
        let (_, ret_type) = fn_env.get(&func.name).unwrap();
        let ret_type = ret_type.clone();

        let mut locals: HashMap<String, (Type, bool)> = HashMap::new();
        for param in &func.params {
            let ty = resolve_type(&param.type_name.name, &struct_registry, &enum_registry)?;
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
            check_stmt(stmt, &mut locals, &fn_env, &ret_type, &struct_registry, &enum_registry, &method_registry)?;

            if is_last {
                match stmt {
                    Stmt::Expr(expr) => {
                        let ty = check_expr(expr, &locals, &fn_env, &ret_type, &struct_registry, &enum_registry, &method_registry)?;
                        if ty != ret_type {
                            return Err(TypeError::new(format!(
                                "function '{}': return type mismatch: expected {} but got {}",
                                func.name, ret_type, ty
                            )));
                        }
                    }
                    Stmt::Return { .. } => {
                        // Already type-checked in check_stmt
                    }
                    Stmt::Let { .. } | Stmt::While { .. } | Stmt::Assign { .. } | Stmt::If { .. } => {
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
            let ret_type = resolve_type(&method.return_type.name, &struct_registry, &enum_registry)?;

            let mut locals: HashMap<String, (Type, bool)> = HashMap::new();
            for param in &method.params {
                let ty = resolve_type(&param.type_name.name, &struct_registry, &enum_registry)?;
                locals.insert(param.name.clone(), (ty, false));
            }

            if method.body.is_empty() {
                return Err(TypeError::new(format!(
                    "method '{}': missing return expression", method.name
                )));
            }

            for (i, stmt) in method.body.iter().enumerate() {
                let is_last = i == method.body.len() - 1;
                check_stmt(stmt, &mut locals, &fn_env, &ret_type, &struct_registry, &enum_registry, &method_registry)?;

                if is_last {
                    match stmt {
                        Stmt::Expr(expr) => {
                            let ty = check_expr(expr, &locals, &fn_env, &ret_type, &struct_registry, &enum_registry, &method_registry)?;
                            if ty != ret_type {
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

    Ok(())
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
) -> Result<Type, TypeError> {
    match expr {
        Expr::IntLiteral { .. } => Ok(Type::Int),
        Expr::StringLiteral { .. } => Ok(Type::String),

        Expr::Identifier { name, .. } => {
            locals
                .get(name)
                .map(|(ty, _)| ty.clone())
                .ok_or_else(|| TypeError::new(format!("undefined variable '{}'", name)))
        }

        Expr::BinaryOp { left, op, right, .. } => {
            use cyflym_parser::ast::BinOp;
            let lt = check_expr(left, locals, fn_env, ret_type, structs, enums, methods)?;
            let rt = check_expr(right, locals, fn_env, ret_type, structs, enums, methods)?;

            match op {
                // Arithmetic: Int x Int -> Int
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
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
                // Comparison: Int x Int -> Bool
                BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                    if lt != Type::Int {
                        return Err(TypeError::new(format!(
                            "comparison operator requires Int operands, left operand is {}", lt
                        )));
                    }
                    if rt != Type::Int {
                        return Err(TypeError::new(format!(
                            "comparison operator requires Int operands, right operand is {}", rt
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
                cyflym_parser::ast::UnaryOp::Not => {
                    let ty = check_expr(operand, locals, fn_env, ret_type, structs, enums, methods)?;
                    if ty != Type::Bool {
                        return Err(TypeError::new(format!(
                            "'!' operator requires Bool operand, got {}",
                            ty
                        )));
                    }
                    Ok(Type::Bool)
                }
            }
        }

        Expr::If { condition, then_body, then_expr, else_body, else_expr, .. } => {
            let cond_ty = check_expr(condition, locals, fn_env, ret_type, structs, enums, methods)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "if condition must be Bool, got {}",
                    cond_ty
                )));
            }

            // Type-check then branch (body stmts + final expr)
            let mut then_locals = locals.clone();
            check_stmts(then_body, &mut then_locals, fn_env, ret_type, structs, enums, methods)?;
            let then_ty = check_expr(then_expr, &then_locals, fn_env, ret_type, structs, enums, methods)?;

            // Type-check else branch
            let mut else_locals = locals.clone();
            check_stmts(else_body, &mut else_locals, fn_env, ret_type, structs, enums, methods)?;
            let else_ty = check_expr(else_expr, &else_locals, fn_env, ret_type, structs, enums, methods)?;

            // Both branches must have the same type
            if then_ty != else_ty {
                return Err(TypeError::new(format!(
                    "if/else branch type mismatch: then branch is {} but else branch is {}",
                    then_ty, else_ty
                )));
            }

            Ok(then_ty)
        }

        Expr::Call { function, args, .. } => {
            if function == "print" {
                if args.len() != 1 {
                    return Err(TypeError::new(format!(
                        "print() takes exactly 1 argument, got {}", args.len()
                    )));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods)?;
                match arg_ty {
                    Type::String | Type::Int | Type::Bool => {}
                    other => {
                        return Err(TypeError::new(format!(
                            "print() cannot print type {}", other
                        )));
                    }
                }
                return Ok(Type::Int);
            }

            let (param_types, call_ret_type) = fn_env
                .get(function)
                .ok_or_else(|| TypeError::new(format!("undefined function '{}'", function)))?;

            if args.len() != param_types.len() {
                return Err(TypeError::new(format!(
                    "wrong argument count calling '{}': expected {} argument(s) but got {}",
                    function,
                    param_types.len(),
                    args.len()
                )));
            }

            for (i, (arg, expected)) in args.iter().zip(param_types.iter()).enumerate() {
                let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods)?;
                if actual != *expected {
                    return Err(TypeError::new(format!(
                        "argument {} to '{}': expected {} but got {}",
                        i + 1,
                        function,
                        expected,
                        actual
                    )));
                }
            }

            Ok(call_ret_type.clone())
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
                let actual_type = check_expr(field_expr, locals, fn_env, ret_type, structs, enums, methods)?;
                if actual_type != *expected_type {
                    return Err(TypeError::new(format!(
                        "type mismatch for field '{}' of struct '{}': expected {} but got {}",
                        field_name, name, expected_type, actual_type
                    )));
                }
            }

            Ok(Type::Struct { name: name.clone(), fields: expected_fields })
        }

        Expr::FieldAccess { object, field, .. } => {
            let obj_ty = check_expr(object, locals, fn_env, ret_type, structs, enums, methods)?;
            match &obj_ty {
                Type::Struct { name, fields } => {
                    fields
                        .iter()
                        .find(|(n, _)| n == field)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| TypeError::new(format!(
                            "no field '{}' on struct '{}'", field, name
                        )))
                }
                other => Err(TypeError::new(format!(
                    "field access on non-struct type {}", other
                ))),
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
                let actual_ty = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods)?;
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
            let scrutinee_ty = check_expr(scrutinee, locals, fn_env, ret_type, structs, enums, methods)?;
            let (enum_name, variants) = match &scrutinee_ty {
                Type::Enum { name, variants } => (name.clone(), variants.clone()),
                other => return Err(TypeError::new(format!(
                    "match scrutinee must be an enum type, got {}", other
                ))),
            };

            let mut result_type: Option<Type> = None;
            for arm in arms {
                let cyflym_parser::ast::Pattern::EnumVariant {
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

                let arm_ty = check_expr(&arm.body, &arm_locals, fn_env, ret_type, structs, enums, methods)?;

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

        Expr::MethodCall { object, method, args, .. } => {
            let obj_ty = check_expr(object, locals, fn_env, ret_type, structs, enums, methods)?;
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
                let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods)?;
                if actual != *expected {
                    return Err(TypeError::new(format!(
                        "argument {} to method '{}': expected {} but got {}",
                        i + 1, method, expected, actual
                    )));
                }
            }
            Ok(call_ret_type.clone())
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn do_check(src: &str) -> Result<(), TypeError> {
        let prog = cyflym_parser::parse(src)
            .expect("parse error in test input");
        check(&prog)
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
}
