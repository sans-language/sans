use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use cyflym_ir::ir::{Instruction, IrBinOp, Module};
use inkwell::context::Context;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::values::IntValue;
use inkwell::OptimizationLevel;

#[derive(Debug)]
pub enum CodegenError {
    LlvmError(String),
    TargetError(String),
    IoError(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::LlvmError(msg) => write!(f, "LLVM error: {}", msg),
            CodegenError::TargetError(msg) => write!(f, "Target error: {}", msg),
            CodegenError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

/// Compile an IR module to a native object file.
pub fn compile_to_object(module: &Module, output_path: &str) -> Result<(), CodegenError> {
    let context = Context::create();
    let llvm_module = generate_llvm(&context, module)?;

    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| CodegenError::TargetError(e.to_string()))?;

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple)
        .map_err(|e| CodegenError::TargetError(e.to_string()))?;

    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| CodegenError::TargetError("failed to create target machine".into()))?;

    machine
        .write_to_file(&llvm_module, FileType::Object, Path::new(output_path))
        .map_err(|e| CodegenError::IoError(e.to_string()))?;

    Ok(())
}

/// Compile an IR module to an LLVM IR string (useful for testing).
pub fn compile_to_llvm_ir(module: &Module) -> Result<String, CodegenError> {
    let context = Context::create();
    let llvm_module = generate_llvm(&context, module)?;
    Ok(llvm_module.print_to_string().to_string())
}

fn generate_llvm<'ctx>(
    context: &'ctx Context,
    module: &Module,
) -> Result<inkwell::module::Module<'ctx>, CodegenError> {
    let llvm_module = context.create_module("cyflym");
    let builder = context.create_builder();
    let i64_type = context.i64_type();

    // First pass: declare all functions
    for func in &module.functions {
        let param_types: Vec<inkwell::types::BasicMetadataTypeEnum> =
            func.params.iter().map(|_| i64_type.into()).collect();
        let fn_type = i64_type.fn_type(&param_types, false);
        llvm_module.add_function(&func.name, fn_type, None);
    }

    // Second pass: generate function bodies
    for func in &module.functions {
        let llvm_fn = llvm_module
            .get_function(&func.name)
            .ok_or_else(|| CodegenError::LlvmError(format!("function {} not found", func.name)))?;

        let entry = context.append_basic_block(llvm_fn, "entry");
        builder.position_at_end(entry);

        let mut regs: HashMap<String, IntValue<'ctx>> = HashMap::new();

        // Map parameter names to LLVM parameter values
        for (i, param_name) in func.params.iter().enumerate() {
            let param_val = llvm_fn
                .get_nth_param(i as u32)
                .ok_or_else(|| CodegenError::LlvmError(format!("missing param {}", i)))?
                .into_int_value();
            regs.insert(param_name.clone(), param_val);
        }

        // Generate instructions
        for instr in &func.body {
            match instr {
                Instruction::Const { dest, value } => {
                    let val = i64_type.const_int(*value as u64, true);
                    regs.insert(dest.clone(), val);
                }
                Instruction::BinOp {
                    dest,
                    op,
                    left,
                    right,
                } => {
                    let lhs = regs[left];
                    let rhs = regs[right];
                    let result = match op {
                        IrBinOp::Add => builder
                            .build_int_add(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                        IrBinOp::Sub => builder
                            .build_int_sub(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                        IrBinOp::Mul => builder
                            .build_int_mul(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                        IrBinOp::Div => builder
                            .build_int_signed_div(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::Copy { dest, src } => {
                    let val = regs[src];
                    regs.insert(dest.clone(), val);
                }
                Instruction::Call {
                    dest,
                    function,
                    args,
                } => {
                    let callee = llvm_module.get_function(function).ok_or_else(|| {
                        CodegenError::LlvmError(format!("undefined function: {}", function))
                    })?;
                    let arg_vals: Vec<inkwell::values::BasicMetadataValueEnum> =
                        args.iter().map(|a| regs[a].into()).collect();
                    let call_site = builder
                        .build_call(callee, &arg_vals, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ret_val = match call_site.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => {
                            bv.into_int_value()
                        }
                        _ => {
                            return Err(CodegenError::LlvmError("call returned void".into()));
                        }
                    };
                    regs.insert(dest.clone(), ret_val);
                }
                Instruction::Ret { value } => {
                    let val = regs[value];
                    builder
                        .build_return(Some(&val))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
            }
        }
    }

    Ok(llvm_module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyflym_ir::ir::{Instruction, IrBinOp, IrFunction, Module};

    #[test]
    fn codegen_produces_llvm_ir() {
        let module = Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                body: vec![
                    Instruction::Const {
                        dest: "%0".to_string(),
                        value: 42,
                    },
                    Instruction::Ret {
                        value: "%0".to_string(),
                    },
                ],
            }],
        };

        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        assert!(ir.contains("define i64 @main()"), "expected 'define i64 @main()' in:\n{}", ir);
        assert!(ir.contains("ret i64"), "expected 'ret i64' in:\n{}", ir);
    }

    #[test]
    fn codegen_arithmetic() {
        let module = Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                body: vec![
                    Instruction::Const {
                        dest: "%0".to_string(),
                        value: 1,
                    },
                    Instruction::Const {
                        dest: "%1".to_string(),
                        value: 2,
                    },
                    Instruction::BinOp {
                        dest: "%2".to_string(),
                        op: IrBinOp::Add,
                        left: "%0".to_string(),
                        right: "%1".to_string(),
                    },
                    Instruction::Ret {
                        value: "%2".to_string(),
                    },
                ],
            }],
        };

        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        // LLVM may constant-fold 1+2 into 3, so accept either an add instruction
        // or the folded constant result.
        assert!(
            ir.contains("add") || ir.contains("ret i64 3"),
            "expected 'add' or constant-folded 'ret i64 3' in:\n{}",
            ir
        );
        assert!(ir.contains("ret i64"), "expected 'ret i64' in:\n{}", ir);
    }
}
