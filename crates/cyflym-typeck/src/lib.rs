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

        Expr::BinaryOp { left, right, .. } => {
            let lt = check_expr(left, locals, fn_env)?;
            let rt = check_expr(right, locals, fn_env)?;
            if lt != Type::Int {
                return Err(TypeError::new(format!(
                    "binary operator requires Int operands, left operand is {}",
                    lt
                )));
            }
            if rt != Type::Int {
                return Err(TypeError::new(format!(
                    "binary operator requires Int operands, right operand is {}",
                    rt
                )));
            }
            Ok(Type::Int)
        }

        // TODO(Task 4): properly type-check these new expression forms
        Expr::BoolLiteral { .. } => Ok(Type::Int), // placeholder
        Expr::If { .. } => Ok(Type::Int),           // placeholder
        Expr::UnaryOp { .. } => Ok(Type::Int),      // placeholder

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
}
