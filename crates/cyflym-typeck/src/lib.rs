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

/// Type-check a list of statements.
fn check_stmts(
    stmts: &[Stmt],
    locals: &mut HashMap<String, (Type, bool)>,
    fn_env: &HashMap<String, (Vec<Type>, Type)>,
    ret_type: &Type,
) -> Result<(), TypeError> {
    for stmt in stmts {
        check_stmt(stmt, locals, fn_env, ret_type)?;
    }
    Ok(())
}

fn check_stmt(
    stmt: &Stmt,
    locals: &mut HashMap<String, (Type, bool)>,
    fn_env: &HashMap<String, (Vec<Type>, Type)>,
    ret_type: &Type,
) -> Result<(), TypeError> {
    match stmt {
        Stmt::Let { name, mutable, type_name, value, .. } => {
            let actual = check_expr(value, locals, fn_env, ret_type)?;
            let ty = if let Some(tn) = type_name {
                let declared = resolve_type(&tn.name)?;
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
            check_expr(expr, locals, fn_env, ret_type)?;
            Ok(())
        }
        Stmt::While { condition, body, .. } => {
            let cond_ty = check_expr(condition, locals, fn_env, ret_type)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "while condition must be Bool, got {}", cond_ty
                )));
            }
            check_stmts(body, locals, fn_env, ret_type)?;
            Ok(())
        }
        Stmt::Return { value, .. } => {
            let ty = check_expr(value, locals, fn_env, ret_type)?;
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
            let actual = check_expr(value, locals, fn_env, ret_type)?;
            if actual != expected_ty {
                return Err(TypeError::new(format!(
                    "type mismatch in assignment to '{}': expected {} but got {}",
                    name, expected_ty, actual
                )));
            }
            Ok(())
        }
        Stmt::If { condition, body, .. } => {
            let cond_ty = check_expr(condition, locals, fn_env, ret_type)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "if condition must be Bool, got {}", cond_ty
                )));
            }
            check_stmts(body, locals, fn_env, ret_type)?;
            Ok(())
        }
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

        let mut locals: HashMap<String, (Type, bool)> = HashMap::new();
        for param in &func.params {
            let ty = resolve_type(&param.type_name.name)?;
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
            check_stmt(stmt, &mut locals, &fn_env, &ret_type)?;

            if is_last {
                match stmt {
                    Stmt::Expr(expr) => {
                        let ty = check_expr(expr, &locals, &fn_env, &ret_type)?;
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

    Ok(())
}

/// Type-check a single expression and return its type.
fn check_expr(
    expr: &Expr,
    locals: &HashMap<String, (Type, bool)>,
    fn_env: &HashMap<String, (Vec<Type>, Type)>,
    ret_type: &Type,
) -> Result<Type, TypeError> {
    match expr {
        Expr::IntLiteral { .. } => Ok(Type::Int),

        Expr::Identifier { name, .. } => {
            locals
                .get(name)
                .map(|(ty, _)| ty.clone())
                .ok_or_else(|| TypeError::new(format!("undefined variable '{}'", name)))
        }

        Expr::BinaryOp { left, op, right, .. } => {
            use cyflym_parser::ast::BinOp;
            let lt = check_expr(left, locals, fn_env, ret_type)?;
            let rt = check_expr(right, locals, fn_env, ret_type)?;

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
                    let ty = check_expr(operand, locals, fn_env, ret_type)?;
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
            let cond_ty = check_expr(condition, locals, fn_env, ret_type)?;
            if cond_ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "if condition must be Bool, got {}",
                    cond_ty
                )));
            }

            // Type-check then branch (body stmts + final expr)
            let mut then_locals = locals.clone();
            check_stmts(then_body, &mut then_locals, fn_env, ret_type)?;
            let then_ty = check_expr(then_expr, &then_locals, fn_env, ret_type)?;

            // Type-check else branch
            let mut else_locals = locals.clone();
            check_stmts(else_body, &mut else_locals, fn_env, ret_type)?;
            let else_ty = check_expr(else_expr, &else_locals, fn_env, ret_type)?;

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
                let actual = check_expr(arg, locals, fn_env, ret_type)?;
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
    fn check_while_with_return() {
        assert!(do_check("fn main() Int { while true { return 42 } 0 }").is_ok());
    }
}
