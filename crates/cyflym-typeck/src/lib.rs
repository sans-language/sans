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
fn resolve_type(name: &str) -> Result<Type, TypeError> {
    match name {
        "Int" => Ok(Type::Int),
        "Bool" => Ok(Type::Bool),
        other => Err(TypeError::new(format!("unknown type '{}'", other))),
    }
}

/// Type-check the given `Program`. Returns `Ok(())` if the program is
/// well-typed, or a `TypeError` describing the first problem found.
pub fn check(program: &Program) -> Result<(), TypeError> {
    // Pass 1: collect all function signatures into an environment.
    let mut fn_env: HashMap<String, (Vec<Type>, Type)> = HashMap::new();

    for func in &program.functions {
        let mut param_types = Vec::new();
        for param in &func.params {
            param_types.push(resolve_type(&param.type_name.name)?);
        }
        let ret_type = resolve_type(&func.return_type.name)?;
        fn_env.insert(func.name.clone(), (param_types, ret_type));
    }

    // Require a `main` function.
    if !fn_env.contains_key("main") {
        return Err(TypeError::new("missing 'main' function"));
    }

    // Pass 2: type-check each function body.
    for func in &program.functions {
        let (_, ret_type) = fn_env.get(&func.name).unwrap();
        let ret_type = ret_type.clone();

        // Build the locals map, seeded with the function parameters.
        let mut locals: HashMap<String, Type> = HashMap::new();
        for param in &func.params {
            let ty = resolve_type(&param.type_name.name)?;
            locals.insert(param.name.clone(), ty);
        }

        // Check the body. The last statement must be an Expr whose type
        // matches the declared return type.
        if func.body.is_empty() {
            return Err(TypeError::new(format!(
                "function '{}': missing return expression",
                func.name
            )));
        }

        // Check all statements, updating locals for `let` bindings.
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == func.body.len() - 1;
            match stmt {
                Stmt::Let { name, type_name, value, .. } => {
                    if is_last {
                        return Err(TypeError::new(format!(
                            "function '{}': missing return expression",
                            func.name
                        )));
                    }
                    let declared = resolve_type(&type_name.name)?;
                    let actual = check_expr(value, &locals, &fn_env)?;
                    if declared != actual {
                        return Err(TypeError::new(format!(
                            "type mismatch in let '{}': declared {} but expression has type {}",
                            name, declared, actual
                        )));
                    }
                    locals.insert(name.clone(), declared);
                }
                Stmt::Expr(expr) => {
                    let ty = check_expr(expr, &locals, &fn_env)?;
                    if is_last && ty != ret_type {
                        return Err(TypeError::new(format!(
                            "function '{}': return type mismatch: expected {} but got {}",
                            func.name, ret_type, ty
                        )));
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
    locals: &HashMap<String, Type>,
    fn_env: &HashMap<String, (Vec<Type>, Type)>,
) -> Result<Type, TypeError> {
    match expr {
        Expr::IntLiteral { .. } => Ok(Type::Int),

        Expr::Identifier { name, .. } => {
            locals
                .get(name)
                .cloned()
                .ok_or_else(|| TypeError::new(format!("undefined variable '{}'", name)))
        }

        Expr::BinaryOp { left, op, right, .. } => {
            use cyflym_parser::ast::BinOp;
            let lt = check_expr(left, locals, fn_env)?;
            let rt = check_expr(right, locals, fn_env)?;

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
                    let ty = check_expr(operand, locals, fn_env)?;
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
            // Condition must be Bool
            let cond_ty = check_expr(condition, locals, fn_env)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "if condition must be Bool, got {}",
                    cond_ty
                )));
            }

            // Type-check then branch (body stmts + final expr)
            let mut then_locals = locals.clone();
            for stmt in then_body {
                match stmt {
                    Stmt::Let { name, type_name, value, .. } => {
                        let declared = resolve_type(&type_name.name)?;
                        let actual = check_expr(value, &then_locals, fn_env)?;
                        if declared != actual {
                            return Err(TypeError::new(format!(
                                "type mismatch in let '{}': declared {} but expression has type {}",
                                name, declared, actual
                            )));
                        }
                        then_locals.insert(name.clone(), declared);
                    }
                    Stmt::Expr(expr) => {
                        check_expr(expr, &then_locals, fn_env)?;
                    }
                }
            }
            let then_ty = check_expr(then_expr, &then_locals, fn_env)?;

            // Type-check else branch
            let mut else_locals = locals.clone();
            for stmt in else_body {
                match stmt {
                    Stmt::Let { name, type_name, value, .. } => {
                        let declared = resolve_type(&type_name.name)?;
                        let actual = check_expr(value, &else_locals, fn_env)?;
                        if declared != actual {
                            return Err(TypeError::new(format!(
                                "type mismatch in let '{}': declared {} but expression has type {}",
                                name, declared, actual
                            )));
                        }
                        else_locals.insert(name.clone(), declared);
                    }
                    Stmt::Expr(expr) => {
                        check_expr(expr, &else_locals, fn_env)?;
                    }
                }
            }
            let else_ty = check_expr(else_expr, &else_locals, fn_env)?;

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
            let (param_types, ret_type) = fn_env
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
                let actual = check_expr(arg, locals, fn_env)?;
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

            Ok(ret_type.clone())
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
}
