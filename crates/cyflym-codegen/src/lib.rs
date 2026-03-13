use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use cyflym_ir::ir::{Instruction, IrBinOp, IrCmpOp, Module};
use inkwell::context::Context;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::values::{IntValue, PointerValue};
use inkwell::module::Linkage;
use inkwell::IntPredicate;
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

    // Declare printf
    let i8_ptr_type = context.ptr_type(inkwell::AddressSpace::default());
    let printf_type = context.i32_type().fn_type(&[i8_ptr_type.into()], true);
    llvm_module.add_function("printf", printf_type, Some(Linkage::External));

    // Declare pthread and memory functions
    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
    let i32_type = context.i32_type();

    let pthread_create_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false);
    llvm_module.add_function("pthread_create", pthread_create_type, Some(Linkage::External));

    let pthread_join_type = i32_type.fn_type(&[i64_type.into(), ptr_type.into()], false);
    llvm_module.add_function("pthread_join", pthread_join_type, Some(Linkage::External));

    let mutex_init_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    llvm_module.add_function("pthread_mutex_init", mutex_init_type, Some(Linkage::External));
    let mutex_op_type = i32_type.fn_type(&[ptr_type.into()], false);
    llvm_module.add_function("pthread_mutex_lock", mutex_op_type, Some(Linkage::External));
    llvm_module.add_function("pthread_mutex_unlock", mutex_op_type, Some(Linkage::External));

    let cond_init_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    llvm_module.add_function("pthread_cond_init", cond_init_type, Some(Linkage::External));
    let cond_wait_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
    llvm_module.add_function("pthread_cond_wait", cond_wait_type, Some(Linkage::External));
    let cond_signal_type = i32_type.fn_type(&[ptr_type.into()], false);
    llvm_module.add_function("pthread_cond_signal", cond_signal_type, Some(Linkage::External));

    let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
    llvm_module.add_function("malloc", malloc_type, Some(Linkage::External));
    let free_type = context.void_type().fn_type(&[ptr_type.into()], false);
    llvm_module.add_function("free", free_type, Some(Linkage::External));

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
        let mut ptrs: HashMap<String, PointerValue<'ctx>> = HashMap::new();
        let mut struct_sizes: HashMap<String, usize> = HashMap::new();

        // Map parameter names to LLVM parameter values
        for (i, param_name) in func.params.iter().enumerate() {
            let param_val = llvm_fn
                .get_nth_param(i as u32)
                .ok_or_else(|| CodegenError::LlvmError(format!("missing param {}", i)))?
                .into_int_value();
            regs.insert(param_name.clone(), param_val);

            let size = func.param_struct_sizes[i];
            if size > 0 {
                // This is a struct/enum pointer passed as i64 — convert back to pointer
                let ptr_val = builder.build_int_to_ptr(
                    param_val,
                    context.ptr_type(inkwell::AddressSpace::default()),
                    &format!("{}_ptr", param_name),
                ).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                ptrs.insert(param_name.clone(), ptr_val);
                struct_sizes.insert(param_name.clone(), size);
            }
        }

        // Pre-create basic blocks for all Label instructions
        let mut label_blocks: HashMap<String, inkwell::basic_block::BasicBlock<'ctx>> =
            HashMap::new();
        for instr in &func.body {
            if let Instruction::Label { name } = instr {
                let bb = context.append_basic_block(llvm_fn, name);
                label_blocks.insert(name.clone(), bb);
            }
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
                        args.iter().map(|a| {
                            if let Some(ptr_val) = ptrs.get(a) {
                                // Convert pointer to i64 for passing struct/enum self
                                builder.build_ptr_to_int(*ptr_val, i64_type, "ptr2int")
                                    .unwrap()
                                    .into()
                            } else {
                                regs[a].into()
                            }
                        }).collect();
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
                    // If returning an i1 (bool), zext to i64 for the function return type
                    let ret_val = if val.get_type().get_bit_width() == 1 {
                        builder
                            .build_int_z_extend(val, i64_type, "zext_ret")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    } else {
                        val
                    };
                    builder
                        .build_return(Some(&ret_val))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::BoolConst { dest, value } => {
                    let bool_type = context.bool_type();
                    let val = bool_type.const_int(*value as u64, false);
                    regs.insert(dest.clone(), val);
                }
                Instruction::CmpOp {
                    dest,
                    op,
                    left,
                    right,
                } => {
                    let lhs = regs[left];
                    let rhs = regs[right];
                    let pred = match op {
                        IrCmpOp::Eq => IntPredicate::EQ,
                        IrCmpOp::NotEq => IntPredicate::NE,
                        IrCmpOp::Lt => IntPredicate::SLT,
                        IrCmpOp::Gt => IntPredicate::SGT,
                        IrCmpOp::LtEq => IntPredicate::SLE,
                        IrCmpOp::GtEq => IntPredicate::SGE,
                    };
                    let result = builder
                        .build_int_compare(pred, lhs, rhs, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result);
                }
                Instruction::Not { dest, src } => {
                    let val = regs[src];
                    let result = builder
                        .build_not(val, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result);
                }
                Instruction::Label { name } => {
                    let bb = label_blocks[name];
                    builder.position_at_end(bb);
                }
                Instruction::Branch {
                    cond,
                    then_label,
                    else_label,
                } => {
                    let cond_val = regs[cond];
                    let then_bb = label_blocks[then_label];
                    let else_bb = label_blocks[else_label];
                    builder
                        .build_conditional_branch(cond_val, then_bb, else_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::Jump { target } => {
                    let target_bb = label_blocks[target];
                    builder
                        .build_unconditional_branch(target_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::Phi {
                    dest,
                    a_val,
                    a_label,
                    b_val,
                    b_label,
                } => {
                    let a = regs[a_val];
                    let b = regs[b_val];
                    let a_bb = label_blocks[a_label];
                    let b_bb = label_blocks[b_label];

                    // Determine the phi type from the incoming values
                    let phi_type = a.get_type();
                    let phi = builder
                        .build_phi(phi_type, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    phi.add_incoming(&[(&a, a_bb), (&b, b_bb)]);
                    regs.insert(dest.clone(), phi.as_basic_value().into_int_value());
                }
                Instruction::StringConst { dest, value } => {
                    let string_val = context.const_string(value.as_bytes(), true);
                    let global = llvm_module.add_global(string_val.get_type(), None, &format!("str.{}", dest));
                    global.set_initializer(&string_val);
                    global.set_constant(true);
                    global.set_unnamed_addr(true);
                    let ptr = global.as_pointer_value();
                    ptrs.insert(dest.clone(), ptr);
                }
                Instruction::PrintInt { value } => {
                    let val = regs[value];
                    let fmt_str = context.const_string(b"%ld\n", true);
                    let fmt_global = llvm_module.add_global(fmt_str.get_type(), None, &format!("fmt.int.{}", value));
                    fmt_global.set_initializer(&fmt_str);
                    fmt_global.set_constant(true);
                    let fmt_ptr = fmt_global.as_pointer_value();
                    let printf_fn = llvm_module.get_function("printf").unwrap();
                    builder.build_call(printf_fn, &[fmt_ptr.into(), val.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::PrintString { value } => {
                    let str_ptr = ptrs[value];
                    let fmt_str = context.const_string(b"%s\n", true);
                    let fmt_global = llvm_module.add_global(fmt_str.get_type(), None, &format!("fmt.str.{}", value));
                    fmt_global.set_initializer(&fmt_str);
                    fmt_global.set_constant(true);
                    let fmt_ptr = fmt_global.as_pointer_value();
                    let printf_fn = llvm_module.get_function("printf").unwrap();
                    builder.build_call(printf_fn, &[fmt_ptr.into(), str_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::PrintBool { value } => {
                    let val = regs[value];
                    let true_str = context.const_string(b"true\n", true);
                    let false_str = context.const_string(b"false\n", true);
                    let tg = llvm_module.add_global(true_str.get_type(), None, &format!("bool.t.{}", value));
                    tg.set_initializer(&true_str);
                    tg.set_constant(true);
                    let fg = llvm_module.add_global(false_str.get_type(), None, &format!("bool.f.{}", value));
                    fg.set_initializer(&false_str);
                    fg.set_constant(true);

                    let selected = builder.build_select(val, tg.as_pointer_value(), fg.as_pointer_value(), "boolstr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let printf_fn = llvm_module.get_function("printf").unwrap();
                    builder.build_call(printf_fn, &[selected.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::Alloca { dest } => {
                    let ptr = builder
                        .build_alloca(i64_type, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    ptrs.insert(dest.clone(), ptr);
                }
                Instruction::Store { ptr, value } => {
                    let ptr_val = ptrs[ptr];
                    let val = regs[value];
                    builder
                        .build_store(ptr_val, val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::Load { dest, ptr } => {
                    let ptr_val = ptrs[ptr];
                    let loaded = builder
                        .build_load(i64_type, ptr_val, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                        .into_int_value();
                    regs.insert(dest.clone(), loaded);
                }
                Instruction::StructAlloc { dest, num_fields } => {
                    let field_types: Vec<inkwell::types::BasicTypeEnum> =
                        (0..*num_fields).map(|_| i64_type.into()).collect();
                    let struct_type = context.struct_type(&field_types, false);
                    let ptr = builder
                        .build_alloca(struct_type, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    ptrs.insert(dest.clone(), ptr);
                    struct_sizes.insert(dest.clone(), *num_fields);
                }
                Instruction::FieldStore { ptr, field_index, value } => {
                    let struct_ptr = ptrs[ptr];
                    let val = regs[value];
                    let num_fields = struct_sizes[ptr];
                    let field_types: Vec<inkwell::types::BasicTypeEnum> =
                        (0..num_fields).map(|_| i64_type.into()).collect();
                    let struct_type = context.struct_type(&field_types, false);
                    let field_ptr = builder
                        .build_struct_gep(struct_type, struct_ptr, *field_index as u32, "field_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder
                        .build_store(field_ptr, val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::FieldLoad { dest, ptr, field_index } => {
                    let struct_ptr = ptrs[ptr];
                    let num_fields = struct_sizes[ptr];
                    let field_types: Vec<inkwell::types::BasicTypeEnum> =
                        (0..num_fields).map(|_| i64_type.into()).collect();
                    let struct_type = context.struct_type(&field_types, false);
                    let field_ptr = builder
                        .build_struct_gep(struct_type, struct_ptr, *field_index as u32, "field_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let loaded = builder
                        .build_load(i64_type, field_ptr, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                        .into_int_value();
                    regs.insert(dest.clone(), loaded);
                }
                Instruction::EnumAlloc { dest, tag, num_data_fields } => {
                    let total_fields = 1 + num_data_fields;
                    let field_types: Vec<inkwell::types::BasicTypeEnum> =
                        (0..total_fields).map(|_| i64_type.into()).collect();
                    let struct_type = context.struct_type(&field_types, false);
                    let ptr = builder.build_alloca(struct_type, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Store tag at field 0
                    let tag_ptr = builder.build_struct_gep(struct_type, ptr, 0, "tag_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let tag_val = i64_type.const_int(*tag as u64, true);
                    builder.build_store(tag_ptr, tag_val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    ptrs.insert(dest.clone(), ptr);
                    struct_sizes.insert(dest.clone(), total_fields);
                }
                Instruction::EnumTag { dest, ptr } => {
                    let enum_ptr = ptrs[ptr];
                    let num_fields = struct_sizes[ptr];
                    let field_types: Vec<inkwell::types::BasicTypeEnum> =
                        (0..num_fields).map(|_| i64_type.into()).collect();
                    let struct_type = context.struct_type(&field_types, false);
                    let tag_ptr = builder.build_struct_gep(struct_type, enum_ptr, 0, "tag_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let tag_val = builder.build_load(i64_type, tag_ptr, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                        .into_int_value();
                    regs.insert(dest.clone(), tag_val);
                }
                Instruction::EnumData { dest, ptr, field_index } => {
                    let enum_ptr = ptrs[ptr];
                    let num_fields = struct_sizes[ptr];
                    let field_types: Vec<inkwell::types::BasicTypeEnum> =
                        (0..num_fields).map(|_| i64_type.into()).collect();
                    let struct_type = context.struct_type(&field_types, false);
                    let gep_index = (*field_index + 1) as u32; // +1 for tag at field 0
                    let data_ptr = builder.build_struct_gep(struct_type, enum_ptr, gep_index, "data_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let data_val = builder.build_load(i64_type, data_ptr, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                        .into_int_value();
                    regs.insert(dest.clone(), data_val);
                }
                Instruction::ChannelCreate { tx_dest, rx_dest } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();

                    // Allocate channel: 19 * 8 = 152 bytes
                    let chan_call = builder.build_call(malloc_fn, &[i64_type.const_int(152, false).into()], "chan")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let chan_ptr = match chan_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Allocate buffer: 16 * 8 = 128 bytes
                    let buf_call = builder.build_call(malloc_fn, &[i64_type.const_int(128, false).into()], "buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Store buffer ptr as i64 at offset 0
                    let buf_int: IntValue<'ctx> = builder.build_ptr_to_int(buf_ptr, i64_type, "buf_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(chan_ptr, buf_int)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store capacity=16 at offset 1
                    let off1 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cap_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off1, i64_type.const_int(16, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store count=0 at offset 2
                    let off2 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cnt_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off2, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store head=0 at offset 3
                    let off3 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "head_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off3, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store tail=0 at offset 4
                    let off4 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tail_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off4, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init mutex at offset 5
                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let null = ptr_type.const_null();
                    builder.build_call(llvm_module.get_function("pthread_mutex_init").unwrap(), &[mutex_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init condvar at offset 13
                    let cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "cnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Both tx and rx are the channel ptr as i64
                    let chan_int = builder.build_ptr_to_int(chan_ptr, i64_type, "chan_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(tx_dest.clone(), chan_int);
                    regs.insert(rx_dest.clone(), chan_int);
                }
                Instruction::ChannelSend { tx, value } => {
                    let chan_int = regs[tx];
                    let val = regs[value];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let chan_ptr = builder.build_int_to_ptr(chan_int, ptr_type, "chan")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_lock").unwrap(), &[mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load tail, capacity, buffer ptr
                    let tail_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let tail = builder.build_load(i64_type, tail_ptr, "tail").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap = builder.build_load(i64_type, cap_ptr, "cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_int = builder.build_load(i64_type, chan_ptr, "bi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_ptr = builder.build_int_to_ptr(buf_int, ptr_type, "bp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Write at buffer[tail % cap]
                    let idx = builder.build_int_unsigned_rem(tail, cap, "idx").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let wp = unsafe { builder.build_gep(i64_type, buf_ptr, &[idx], "wp") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(wp, val).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Increment tail and count
                    let one = i64_type.const_int(1, false);
                    let new_tail = builder.build_int_add(tail, one, "nt").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(tail_ptr, new_tail).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt = builder.build_load(i64_type, cnt_ptr, "cnt").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let new_cnt = builder.build_int_add(cnt, one, "nc").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(cnt_ptr, new_cnt).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Signal condvar
                    let cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "cnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_signal").unwrap(), &[cond_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Unlock
                    builder.build_call(llvm_module.get_function("pthread_mutex_unlock").unwrap(), &[mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::ChannelRecv { dest, rx } => {
                    let chan_int = regs[rx];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let chan_ptr = builder.build_int_to_ptr(chan_int, ptr_type, "chan")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "cnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Lock
                    builder.build_call(llvm_module.get_function("pthread_mutex_lock").unwrap(), &[mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // While count == 0, wait
                    let wait_bb = context.append_basic_block(llvm_fn, "recv_wait");
                    let do_wait_bb = context.append_basic_block(llvm_fn, "recv_do_wait");
                    let body_bb = context.append_basic_block(llvm_fn, "recv_body");

                    builder.build_unconditional_branch(wait_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.position_at_end(wait_bb);

                    let cnt_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt = builder.build_load(i64_type, cnt_ptr, "cnt").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let empty = builder.build_int_compare(IntPredicate::EQ, cnt, i64_type.const_int(0, false), "empty")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_conditional_branch(empty, do_wait_bb, body_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(do_wait_bb);
                    builder.build_call(llvm_module.get_function("pthread_cond_wait").unwrap(), &[cond_ptr.into(), mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(wait_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(body_bb);

                    // Read buffer[head % cap]
                    let head_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "hp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let head = builder.build_load(i64_type, head_ptr, "head").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap = builder.build_load(i64_type, cap_ptr, "cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let idx = builder.build_int_unsigned_rem(head, cap, "idx").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_int = builder.build_load(i64_type, chan_ptr, "bi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_ptr = builder.build_int_to_ptr(buf_int, ptr_type, "bp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let rp = unsafe { builder.build_gep(i64_type, buf_ptr, &[idx], "rp") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let received = builder.build_load(i64_type, rp, dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

                    // Update head and count
                    let one = i64_type.const_int(1, false);
                    let new_head = builder.build_int_add(head, one, "nh").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(head_ptr, new_head).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt2 = builder.build_load(i64_type, cnt_ptr, "cnt2").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let new_cnt = builder.build_int_sub(cnt2, one, "nc").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(cnt_ptr, new_cnt).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Unlock
                    builder.build_call(llvm_module.get_function("pthread_mutex_unlock").unwrap(), &[mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    regs.insert(dest.clone(), received);
                }
                Instruction::ThreadSpawn { dest, function, args } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let free_fn = llvm_module.get_function("free").unwrap();
                    let num_args = args.len();

                    // Malloc arg struct (N * 8 bytes)
                    let arg_ptr = if num_args > 0 {
                        let sz = i64_type.const_int((num_args * 8) as u64, false);
                        let arg_call = builder.build_call(malloc_fn, &[sz.into()], "args")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                        match arg_call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                            _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                        }
                    } else {
                        ptr_type.const_null()
                    };

                    // Store args
                    for (i, a) in args.iter().enumerate() {
                        let val = if let Some(p) = ptrs.get(a) {
                            builder.build_ptr_to_int(*p, i64_type, "p2i").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                        } else {
                            regs[a]
                        };
                        let fp = unsafe { builder.build_gep(i64_type, arg_ptr, &[i64_type.const_int(i as u64, false)], "af") }
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                        builder.build_store(fp, val).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    }

                    // Build trampoline
                    let tramp_name = format!("__trampoline_{}", function);
                    let tramp_ty = ptr_type.fn_type(&[ptr_type.into()], false);
                    let tramp_fn = if let Some(f) = llvm_module.get_function(&tramp_name) {
                        f
                    } else {
                        let tramp = llvm_module.add_function(&tramp_name, tramp_ty, None);
                        let tramp_bb = context.append_basic_block(tramp, "entry");
                        let saved = builder.get_insert_block().unwrap();
                        builder.position_at_end(tramp_bb);

                        let ap = tramp.get_nth_param(0).unwrap().into_pointer_value();
                        let target = llvm_module.get_function(function).unwrap();

                        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                        for i in 0..num_args {
                            let fp = unsafe { builder.build_gep(i64_type, ap, &[i64_type.const_int(i as u64, false)], "la") }
                                .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                            let v = builder.build_load(i64_type, fp, &format!("a{}", i))
                                .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                            call_args.push(v.into());
                        }
                        builder.build_call(target, &call_args, "").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                        if num_args > 0 {
                            builder.build_call(free_fn, &[ap.into()], "").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                        }
                        builder.build_return(Some(&ptr_type.const_null())).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                        builder.position_at_end(saved);
                        tramp
                    };

                    // Call pthread_create
                    let thread_alloca = builder.build_alloca(i64_type, "tid").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let pcreate = llvm_module.get_function("pthread_create").unwrap();
                    let tramp_ptr = tramp_fn.as_global_value().as_pointer_value();
                    builder.build_call(pcreate, &[thread_alloca.into(), ptr_type.const_null().into(), tramp_ptr.into(), arg_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let handle = builder.build_load(i64_type, thread_alloca, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), handle);
                }
                Instruction::ThreadJoin { handle } => {
                    let h = regs[handle];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    builder.build_call(
                        llvm_module.get_function("pthread_join").unwrap(),
                        &[h.into(), ptr_type.const_null().into()],
                        ""
                    ).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
            }
        }
    }

    Ok(llvm_module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyflym_ir::ir::{Instruction, IrBinOp, IrCmpOp, IrFunction, Module};

    #[test]
    fn codegen_while_loop() {
        let program = cyflym_parser::parse(
            "fn main() Int { let mut x Int = 0 while x < 3 { x = x + 1 } x }"
        ).expect("parse failed");
        let module = cyflym_ir::lower(&program);
        let ir = compile_to_llvm_ir(&module).expect("codegen failed");

        assert!(ir.contains("alloca"), "expected alloca in:\n{}", ir);
        assert!(ir.contains("br "), "expected branch in:\n{}", ir);
        assert!(ir.contains("ret i64"), "expected ret in:\n{}", ir);
    }

    #[test]
    fn codegen_print() {
        let program = cyflym_parser::parse(r#"fn main() Int { print("hello") }"#).expect("parse failed");
        let module = cyflym_ir::lower(&program);
        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        assert!(ir.contains("printf"), "expected printf in:\n{}", ir);
    }

    #[test]
    fn codegen_produces_llvm_ir() {
        let module = Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                param_struct_sizes: vec![],
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
                param_struct_sizes: vec![],
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

    #[test]
    fn codegen_if_else() {
        // Equivalent to: if true { 1 } else { 2 }
        let module = Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                param_struct_sizes: vec![],
                body: vec![
                    Instruction::BoolConst {
                        dest: "%0".to_string(),
                        value: true,
                    },
                    Instruction::Branch {
                        cond: "%0".to_string(),
                        then_label: "then_0".to_string(),
                        else_label: "else_0".to_string(),
                    },
                    Instruction::Label {
                        name: "then_0".to_string(),
                    },
                    Instruction::Const {
                        dest: "%1".to_string(),
                        value: 1,
                    },
                    Instruction::Jump {
                        target: "merge_0".to_string(),
                    },
                    Instruction::Label {
                        name: "else_0".to_string(),
                    },
                    Instruction::Const {
                        dest: "%2".to_string(),
                        value: 2,
                    },
                    Instruction::Jump {
                        target: "merge_0".to_string(),
                    },
                    Instruction::Label {
                        name: "merge_0".to_string(),
                    },
                    Instruction::Phi {
                        dest: "%3".to_string(),
                        a_val: "%1".to_string(),
                        a_label: "then_0".to_string(),
                        b_val: "%2".to_string(),
                        b_label: "else_0".to_string(),
                    },
                    Instruction::Ret {
                        value: "%3".to_string(),
                    },
                ],
            }],
        };

        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        assert!(
            ir.contains("br i1"),
            "expected conditional branch in:\n{}",
            ir
        );
        assert!(ir.contains("phi"), "expected phi node in:\n{}", ir);
        assert!(ir.contains("ret i64"), "expected ret i64 in:\n{}", ir);
    }

    #[test]
    fn codegen_struct() {
        let program = cyflym_parser::parse(
            "struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 3, y: 4 } p.x + p.y }"
        ).expect("parse failed");
        let module = cyflym_ir::lower(&program);
        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        assert!(ir.contains("alloca"), "expected alloca in:\n{}", ir);
        assert!(ir.contains("getelementptr"), "expected GEP in:\n{}", ir);
        assert!(ir.contains("ret i64"), "expected ret in:\n{}", ir);
    }

    #[test]
    fn codegen_enum_match() {
        let program = cyflym_parser::parse(
            "enum Color { Red, Green, Blue, } fn main() Int { let c = Color::Green match c { Color::Red => 1, Color::Green => 2, Color::Blue => 3, } }"
        ).expect("parse failed");
        let module = cyflym_ir::lower(&program);
        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        assert!(ir.contains("alloca"), "expected alloca in:\n{}", ir);
        assert!(ir.contains("getelementptr"), "expected GEP in:\n{}", ir);
        assert!(ir.contains("br "), "expected branch in:\n{}", ir);
        assert!(ir.contains("ret i64"), "expected ret in:\n{}", ir);
    }

    #[test]
    fn codegen_comparison() {
        // Equivalent to: 1 == 2 (returns bool, zext to i64 for return)
        let module = Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                param_struct_sizes: vec![],
                body: vec![
                    Instruction::Const {
                        dest: "%0".to_string(),
                        value: 1,
                    },
                    Instruction::Const {
                        dest: "%1".to_string(),
                        value: 2,
                    },
                    Instruction::CmpOp {
                        dest: "%2".to_string(),
                        op: IrCmpOp::Eq,
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
        // LLVM may constant-fold `1 == 2` to `false` (ret i64 0), so accept either
        assert!(
            ir.contains("icmp eq") || ir.contains("ret i64 0"),
            "expected icmp eq or constant-folded 'ret i64 0' in:\n{}",
            ir
        );
        assert!(
            ir.contains("zext") || ir.contains("ret i64 0"),
            "expected zext or constant-folded 'ret i64 0' in:\n{}",
            ir
        );
    }

    #[test]
    fn codegen_method_call() {
        let program = cyflym_parser::parse(
            "struct Point { x Int, y Int, } impl Point { fn sum(self) Int { self.x + self.y } } fn main() Int { let p = Point { x: 3, y: 4 } p.sum() }"
        ).expect("parse failed");
        cyflym_typeck::check(&program).expect("type error");
        let module = cyflym_ir::lower(&program);
        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        assert!(ir.contains("define i64 @Point_sum"), "expected Point_sum function in:\n{}", ir);
        assert!(ir.contains("call i64 @Point_sum"), "expected call to Point_sum in:\n{}", ir);
    }

    #[test]
    fn codegen_channel_create() {
        let program = cyflym_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() 0 }").expect("parse");
        cyflym_typeck::check(&program).expect("typeck");
        let module = cyflym_ir::lower(&program);
        let ir = compile_to_llvm_ir(&module).expect("codegen");
        assert!(ir.contains("malloc"), "expected malloc in:\n{}", ir);
        assert!(ir.contains("pthread_mutex_init"), "expected mutex_init in:\n{}", ir);
    }

    #[test]
    fn codegen_spawn() {
        let program = cyflym_parser::parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() h.join() }").expect("parse");
        cyflym_typeck::check(&program).expect("typeck");
        let module = cyflym_ir::lower(&program);
        let ir = compile_to_llvm_ir(&module).expect("codegen");
        assert!(ir.contains("pthread_create"), "expected pthread_create in:\n{}", ir);
        assert!(ir.contains("pthread_join"), "expected pthread_join in:\n{}", ir);
        assert!(ir.contains("__trampoline_worker"), "expected trampoline in:\n{}", ir);
    }

    #[test]
    fn codegen_send_recv() {
        let program = cyflym_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }").expect("parse");
        cyflym_typeck::check(&program).expect("typeck");
        let module = cyflym_ir::lower(&program);
        let ir = compile_to_llvm_ir(&module).expect("codegen");
        assert!(ir.contains("pthread_mutex_lock"), "expected lock in:\n{}", ir);
        assert!(ir.contains("pthread_cond_signal"), "expected signal in:\n{}", ir);
        assert!(ir.contains("pthread_cond_wait"), "expected wait in:\n{}", ir);
    }
}
