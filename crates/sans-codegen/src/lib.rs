use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use sans_ir::ir::{Instruction, IrBinOp, IrCmpOp, Module};
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
    let llvm_module = context.create_module("sans");
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

    // Declare C stdlib functions for string/array ops
    let strlen_type = i64_type.fn_type(&[ptr_type.into()], false);
    llvm_module.add_function("strlen", strlen_type, Some(Linkage::External));

    let memcpy_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false);
    llvm_module.add_function("memcpy", memcpy_type, Some(Linkage::External));

    let snprintf_type = i32_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into()], true);
    llvm_module.add_function("snprintf", snprintf_type, Some(Linkage::External));

    let strtol_type = i64_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false);
    llvm_module.add_function("strtol", strtol_type, Some(Linkage::External));

    // File I/O functions
    let fopen_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("fopen", fopen_type, Some(Linkage::External));

    let fclose_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("fclose", fclose_type, Some(Linkage::External));

    let fread_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("fread", fread_type, Some(Linkage::External));

    let fwrite_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("fwrite", fwrite_type, Some(Linkage::External));

    let fseek_type = i32_type.fn_type(&[i8_ptr_type.into(), i64_type.into(), i32_type.into()], false);
    llvm_module.add_function("fseek", fseek_type, Some(Linkage::External));

    let ftell_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("ftell", ftell_type, Some(Linkage::External));

    let access_type = i32_type.fn_type(&[i8_ptr_type.into(), i32_type.into()], false);
    llvm_module.add_function("access", access_type, Some(Linkage::External));

    // Declare JSON runtime functions
    let json_ptr_from_ptr_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_json_parse", json_ptr_from_ptr_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_stringify", json_ptr_from_ptr_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_get_string", json_ptr_from_ptr_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_type_of", json_ptr_from_ptr_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_string", json_ptr_from_ptr_type, Some(Linkage::External));

    let json_noarg_type = i8_ptr_type.fn_type(&[], false);
    llvm_module.add_function("cy_json_object", json_noarg_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_array", json_noarg_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_null", json_noarg_type, Some(Linkage::External));

    let json_int_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
    llvm_module.add_function("cy_json_int", json_int_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_bool", json_int_type, Some(Linkage::External));

    let json_get_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_json_get", json_get_type, Some(Linkage::External));

    let json_get_index_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    llvm_module.add_function("cy_json_get_index", json_get_index_type, Some(Linkage::External));

    let json_get_int_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_json_get_int", json_get_int_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_get_bool", json_get_int_type, Some(Linkage::External));
    llvm_module.add_function("cy_json_len", json_get_int_type, Some(Linkage::External));

    let json_set_type = context.void_type().fn_type(&[i8_ptr_type.into(), i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_json_set", json_set_type, Some(Linkage::External));

    let json_push_type = context.void_type().fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_json_push", json_push_type, Some(Linkage::External));

    // Declare strcmp for string comparison
    let strcmp_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("strcmp", strcmp_type, Some(Linkage::External));

    // Declare HTTP server runtime functions
    let http_listen_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
    llvm_module.add_function("cy_http_listen", http_listen_type, Some(Linkage::External));

    let http_accept_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_http_accept", http_accept_type, Some(Linkage::External));
    llvm_module.add_function("cy_http_request_path", http_accept_type, Some(Linkage::External));
    llvm_module.add_function("cy_http_request_method", http_accept_type, Some(Linkage::External));
    llvm_module.add_function("cy_http_request_body", http_accept_type, Some(Linkage::External));

    let http_respond_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_http_respond", http_respond_type, Some(Linkage::External));

    // Declare functional runtime functions (map/filter)
    let array_map_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_array_map", array_map_type, Some(Linkage::External));
    llvm_module.add_function("cy_array_filter", array_map_type, Some(Linkage::External));

    // Declare array extension runtime functions
    let array_contains_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    llvm_module.add_function("cy_array_contains", array_contains_type, Some(Linkage::External));

    let array_pop_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_array_pop", array_pop_type, Some(Linkage::External));

    // Declare string extension runtime functions
    let string_trim_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_string_trim", string_trim_type, Some(Linkage::External));

    let string_check_type = i64_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_string_starts_with", string_check_type, Some(Linkage::External));
    llvm_module.add_function("cy_string_ends_with", string_check_type, Some(Linkage::External));
    llvm_module.add_function("cy_string_contains", string_check_type, Some(Linkage::External));

    let string_split_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_string_split", string_split_type, Some(Linkage::External));

    // Declare string replace function
    let string_replace_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_string_replace", string_replace_type, Some(Linkage::External));

    // Declare array remove function
    let array_remove_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    llvm_module.add_function("cy_array_remove", array_remove_type, Some(Linkage::External));

    // Declare result runtime functions
    let result_ok_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
    llvm_module.add_function("cy_result_ok", result_ok_type, Some(Linkage::External));

    let result_err_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_result_err", result_err_type, Some(Linkage::External));

    let result_check_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_result_is_ok", result_check_type, Some(Linkage::External));
    llvm_module.add_function("cy_result_is_err", result_check_type, Some(Linkage::External));
    llvm_module.add_function("cy_result_unwrap", result_check_type, Some(Linkage::External));

    let result_unwrap_or_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    llvm_module.add_function("cy_result_unwrap_or", result_unwrap_or_type, Some(Linkage::External));

    let result_error_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_result_error", result_error_type, Some(Linkage::External));

    // Declare logging runtime functions
    let log_msg_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_log_debug", log_msg_type, Some(Linkage::External));
    llvm_module.add_function("cy_log_info", log_msg_type, Some(Linkage::External));
    llvm_module.add_function("cy_log_warn", log_msg_type, Some(Linkage::External));
    llvm_module.add_function("cy_log_error", log_msg_type, Some(Linkage::External));

    let log_set_level_type = i64_type.fn_type(&[i64_type.into()], false);
    llvm_module.add_function("cy_log_set_level", log_set_level_type, Some(Linkage::External));

    // Declare HTTP runtime functions
    let http_get_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_http_get", http_get_type, Some(Linkage::External));

    let http_post_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_http_post", http_post_type, Some(Linkage::External));

    let http_status_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_http_status", http_status_type, Some(Linkage::External));
    llvm_module.add_function("cy_http_ok", http_status_type, Some(Linkage::External));

    let http_body_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    llvm_module.add_function("cy_http_body", http_body_type, Some(Linkage::External));

    let http_header_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    llvm_module.add_function("cy_http_header", http_header_type, Some(Linkage::External));

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
                Instruction::HttpListen { dest, port } => {
                    let port_val = regs[port];
                    let fn_ref = llvm_module.get_function("cy_http_listen").unwrap();
                    let call = builder.build_call(fn_ref, &[port_val.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_listen: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "hlisten_int").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::HttpAccept { dest, server } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let srv_ptr = if let Some(p) = ptrs.get(server) { *p } else {
                        builder.build_int_to_ptr(regs[server], ptr_type, "haccept_sp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_http_accept").unwrap();
                    let call = builder.build_call(fn_ref, &[srv_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_accept: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "haccept_int").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::HttpRequestPath { dest, request } | Instruction::HttpRequestMethod { dest, request } | Instruction::HttpRequestBody { dest, request } => {
                    let fn_name = match instr {
                        Instruction::HttpRequestPath { .. } => "cy_http_request_path",
                        Instruction::HttpRequestMethod { .. } => "cy_http_request_method",
                        Instruction::HttpRequestBody { .. } => "cy_http_request_body",
                        _ => unreachable!(),
                    };
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let req_ptr = if let Some(p) = ptrs.get(request) { *p } else {
                        builder.build_int_to_ptr(regs[request], ptr_type, "hreq_rp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function(fn_name).unwrap();
                    let call = builder.build_call(fn_ref, &[req_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError(format!("{}: expected return", fn_name))),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "hreq_int").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::HttpRespond { dest, request, status, body } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let req_ptr = if let Some(p) = ptrs.get(request) { *p } else {
                        builder.build_int_to_ptr(regs[request], ptr_type, "hresp_rp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let status_val = regs[status];
                    let body_ptr = if let Some(p) = ptrs.get(body) { *p } else {
                        builder.build_int_to_ptr(regs[body], ptr_type, "hresp_bp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_http_respond").unwrap();
                    let call = builder.build_call(fn_ref, &[req_ptr.into(), status_val.into(), body_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_respond: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::FnRef { dest, name } => {
                    let fn_val = llvm_module.get_function(name).ok_or_else(|| {
                        CodegenError::LlvmError(format!("undefined function for fn_ref: {}", name))
                    })?;
                    let fn_ptr = fn_val.as_global_value().as_pointer_value();
                    let as_int = builder.build_ptr_to_int(fn_ptr, i64_type, "fnref_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), fn_ptr);
                }
                Instruction::ArrayMap { dest, array, fn_ptr } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = if let Some(p) = ptrs.get(array) { *p } else {
                        builder.build_int_to_ptr(regs[array], ptr_type, "amap_ap").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fp = if let Some(p) = ptrs.get(fn_ptr) { *p } else {
                        builder.build_int_to_ptr(regs[fn_ptr], ptr_type, "amap_fp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_array_map").unwrap();
                    let call = builder.build_call(fn_ref, &[arr_ptr.into(), fp.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_array_map: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "amap_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::ArrayFilter { dest, array, fn_ptr } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = if let Some(p) = ptrs.get(array) { *p } else {
                        builder.build_int_to_ptr(regs[array], ptr_type, "afilt_ap").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fp = if let Some(p) = ptrs.get(fn_ptr) { *p } else {
                        builder.build_int_to_ptr(regs[fn_ptr], ptr_type, "afilt_fp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_array_filter").unwrap();
                    let call = builder.build_call(fn_ref, &[arr_ptr.into(), fp.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_array_filter: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "afilt_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::StringTrim { dest, string } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = if let Some(p) = ptrs.get(string) { *p } else {
                        builder.build_int_to_ptr(regs[string], ptr_type, "trim_sp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_string_trim").unwrap();
                    let call = builder.build_call(fn_ref, &[str_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_string_trim: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "trim_int").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::StringStartsWith { dest, string, prefix } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = if let Some(p) = ptrs.get(string) { *p } else {
                        builder.build_int_to_ptr(regs[string], ptr_type, "sw_sp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let pfx_ptr = if let Some(p) = ptrs.get(prefix) { *p } else {
                        builder.build_int_to_ptr(regs[prefix], ptr_type, "sw_pp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_string_starts_with").unwrap();
                    let call = builder.build_call(fn_ref, &[str_ptr.into(), pfx_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_string_starts_with: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::StringEndsWith { dest, string, suffix } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = if let Some(p) = ptrs.get(string) { *p } else {
                        builder.build_int_to_ptr(regs[string], ptr_type, "ew_sp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let sfx_ptr = if let Some(p) = ptrs.get(suffix) { *p } else {
                        builder.build_int_to_ptr(regs[suffix], ptr_type, "ew_pp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_string_ends_with").unwrap();
                    let call = builder.build_call(fn_ref, &[str_ptr.into(), sfx_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_string_ends_with: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::StringContains { dest, string, needle } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = if let Some(p) = ptrs.get(string) { *p } else {
                        builder.build_int_to_ptr(regs[string], ptr_type, "sc_sp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let ndl_ptr = if let Some(p) = ptrs.get(needle) { *p } else {
                        builder.build_int_to_ptr(regs[needle], ptr_type, "sc_np").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_string_contains").unwrap();
                    let call = builder.build_call(fn_ref, &[str_ptr.into(), ndl_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_string_contains: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::StringSplit { dest, string, delimiter } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = if let Some(p) = ptrs.get(string) { *p } else {
                        builder.build_int_to_ptr(regs[string], ptr_type, "ss_sp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let dlm_ptr = if let Some(p) = ptrs.get(delimiter) { *p } else {
                        builder.build_int_to_ptr(regs[delimiter], ptr_type, "ss_dp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_string_split").unwrap();
                    let call = builder.build_call(fn_ref, &[str_ptr.into(), dlm_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_string_split: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "ss_int").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::StringReplace { dest, string, old, new_str } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = if let Some(p) = ptrs.get(string) { *p } else {
                        builder.build_int_to_ptr(regs[string], ptr_type, "srep_sp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let old_ptr = if let Some(p) = ptrs.get(old) { *p } else {
                        builder.build_int_to_ptr(regs[old], ptr_type, "srep_op").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let new_ptr = if let Some(p) = ptrs.get(new_str) { *p } else {
                        builder.build_int_to_ptr(regs[new_str], ptr_type, "srep_np").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_string_replace").unwrap();
                    let call = builder.build_call(fn_ref, &[str_ptr.into(), old_ptr.into(), new_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_string_replace: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "srep_int").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::ArrayRemove { dest, array, index } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = if let Some(p) = ptrs.get(array) { *p } else {
                        builder.build_int_to_ptr(regs[array], ptr_type, "arem_ap").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let idx_val = regs[index];
                    let fn_ref = llvm_module.get_function("cy_array_remove").unwrap();
                    let call = builder.build_call(fn_ref, &[arr_ptr.into(), idx_val.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_array_remove: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::ArrayPop { dest, array } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = if let Some(p) = ptrs.get(array) { *p } else {
                        builder.build_int_to_ptr(regs[array], ptr_type, "apop_ap").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_array_pop").unwrap();
                    let call = builder.build_call(fn_ref, &[arr_ptr.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_array_pop: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::ArrayContains { dest, array, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = if let Some(p) = ptrs.get(array) { *p } else {
                        builder.build_int_to_ptr(regs[array], ptr_type, "ac_ap").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let search_val = regs[value];
                    let fn_ref = llvm_module.get_function("cy_array_contains").unwrap();
                    let call = builder.build_call(fn_ref, &[arr_ptr.into(), search_val.into()], dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_array_contains: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::FloatConst { dest, value } => {
                    let f64_type = context.f64_type();
                    let float_val = f64_type.const_float(*value);
                    let as_int = builder.build_bit_cast(float_val, i64_type, "fconst_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                        .into_int_value();
                    regs.insert(dest.clone(), as_int);
                }
                Instruction::PrintFloat { value } => {
                    let f64_type = context.f64_type();
                    let val = regs[value];
                    let float_val = builder.build_bit_cast(val, f64_type, "pf_cast")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                        .into_float_value();
                    let fmt_str = context.const_string(b"%g\n", true);
                    let fmt_global = llvm_module.add_global(fmt_str.get_type(), None, &format!("fmt.float.{}", value));
                    fmt_global.set_initializer(&fmt_str);
                    fmt_global.set_constant(true);
                    let fmt_ptr = fmt_global.as_pointer_value();
                    let printf_fn = llvm_module.get_function("printf").unwrap();
                    builder.build_call(printf_fn, &[fmt_ptr.into(), float_val.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::FloatBinOp { dest, op, left, right } => {
                    let f64_type = context.f64_type();
                    let lhs = builder.build_bit_cast(regs[left], f64_type, "fbo_l")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_float_value();
                    let rhs = builder.build_bit_cast(regs[right], f64_type, "fbo_r")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_float_value();
                    let result = match op {
                        IrBinOp::Add => builder.build_float_add(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                        IrBinOp::Sub => builder.build_float_sub(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                        IrBinOp::Mul => builder.build_float_mul(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                        IrBinOp::Div => builder.build_float_div(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                        IrBinOp::Mod => builder.build_float_rem(lhs, rhs, dest)
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?,
                    };
                    let as_int = builder.build_bit_cast(result, i64_type, "fbo_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), as_int);
                }
                Instruction::FloatCmpOp { dest, op, left, right } => {
                    let f64_type = context.f64_type();
                    let lhs = builder.build_bit_cast(regs[left], f64_type, "fco_l")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_float_value();
                    let rhs = builder.build_bit_cast(regs[right], f64_type, "fco_r")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_float_value();
                    use inkwell::FloatPredicate;
                    let predicate = match op {
                        IrCmpOp::Eq => FloatPredicate::OEQ,
                        IrCmpOp::NotEq => FloatPredicate::ONE,
                        IrCmpOp::Lt => FloatPredicate::OLT,
                        IrCmpOp::Gt => FloatPredicate::OGT,
                        IrCmpOp::LtEq => FloatPredicate::OLE,
                        IrCmpOp::GtEq => FloatPredicate::OGE,
                    };
                    let cmp = builder.build_float_compare(predicate, lhs, rhs, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = builder.build_int_z_extend(cmp, i64_type, &format!("{}_ext", dest))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result);
                }
                Instruction::IntToFloat { dest, value } => {
                    let f64_type = context.f64_type();
                    let int_val = regs[value];
                    let float_val = builder.build_signed_int_to_float(int_val, f64_type, "i2f")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let as_int = builder.build_bit_cast(float_val, i64_type, "i2f_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), as_int);
                }
                Instruction::FloatToInt { dest, value } => {
                    let f64_type = context.f64_type();
                    let float_val = builder.build_bit_cast(regs[value], f64_type, "f2i_cast")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_float_value();
                    let int_val = builder.build_float_to_signed_int(float_val, i64_type, "f2i")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), int_val);
                }
                Instruction::FloatToString { dest, value } => {
                    let f64_type = context.f64_type();
                    let float_val = builder.build_bit_cast(regs[value], f64_type, "f2s_cast")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_float_value();
                    let snprintf_fn = llvm_module.get_function("snprintf").unwrap();
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let buf_size = i64_type.const_int(32, false);
                    let buf_call = builder.build_call(malloc_fn, &[buf_size.into()], "f2s_buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc failed".into())),
                    };
                    let fmt_str = context.const_string(b"%g", true);
                    let fmt_global = llvm_module.add_global(fmt_str.get_type(), None, &format!("fmt.f2s.{}", value));
                    fmt_global.set_initializer(&fmt_str);
                    fmt_global.set_constant(true);
                    builder.build_call(snprintf_fn, &[buf_ptr.into(), buf_size.into(), fmt_global.as_pointer_value().into(), float_val.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let as_int = builder.build_ptr_to_int(buf_ptr, i64_type, "f2s_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), buf_ptr);
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
                        IrBinOp::Mod => builder
                            .build_int_signed_rem(lhs, rhs, dest)
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
                    let ret_val = if let Some(ptr_val) = ptrs.get(value) {
                        // Returning a struct/enum pointer — convert to i64
                        builder
                            .build_ptr_to_int(*ptr_val, i64_type, "ret_ptr2int")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    } else {
                        let val = regs[value];
                        // If returning an i1 (bool), zext to i64 for the function return type
                        if val.get_type().get_bit_width() == 1 {
                            builder
                                .build_int_z_extend(val, i64_type, "zext_ret")
                                .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                        } else {
                            val
                        }
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
                Instruction::StringCmpOp { dest, op, left, right } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let lhs_ptr = if let Some(p) = ptrs.get(left) { *p } else {
                        builder.build_int_to_ptr(regs[left], ptr_type, "scmp_lp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let rhs_ptr = if let Some(p) = ptrs.get(right) { *p } else {
                        builder.build_int_to_ptr(regs[right], ptr_type, "scmp_rp").map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let strcmp_fn = llvm_module.get_function("strcmp").unwrap();
                    let call = builder.build_call(strcmp_fn, &[lhs_ptr.into(), rhs_ptr.into()], "scmp_res")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let strcmp_result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("strcmp: expected return".into())),
                    };
                    let zero_i32 = context.i32_type().const_int(0, false);
                    let pred = match op {
                        IrCmpOp::Eq => IntPredicate::EQ,
                        IrCmpOp::NotEq => IntPredicate::NE,
                        _ => IntPredicate::EQ, // only == and != supported for strings
                    };
                    let cmp = builder.build_int_compare(pred, strcmp_result, zero_i32, "scmp_cmp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = builder.build_int_z_extend(cmp, i64_type, dest)
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
                Instruction::Neg { dest, src } => {
                    let val = regs[src];
                    let result = builder
                        .build_int_neg(val, dest)
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
                Instruction::FieldLoad { dest, ptr, field_index, num_fields } => {
                    // `ptr` is normally in `ptrs` (alloca). If it's in `regs` instead
                    // (e.g. returned as i64 from a cross-module call), convert it to a ptr.
                    let struct_ptr = if let Some(ptr_val) = ptrs.get(ptr) {
                        *ptr_val
                    } else {
                        let int_val = regs[ptr];
                        builder.build_int_to_ptr(
                            int_val,
                            context.ptr_type(inkwell::AddressSpace::default()),
                            &format!("{}_ptr", ptr),
                        ).map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let num_fields = *num_fields;
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

                    // Allocate channel: 26 * 8 = 208 bytes
                    let chan_call = builder.build_call(malloc_fn, &[i64_type.const_int(208, false).into()], "chan")
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

                    // capacity=16 at offset 1
                    let off1 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cap_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off1, i64_type.const_int(16, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // count=0 at offset 2
                    let off2 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cnt_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off2, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // head=0 at offset 3
                    let off3 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "head_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off3, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // tail=0 at offset 4
                    let off4 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tail_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off4, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init mutex at offset 5
                    let null = ptr_type.const_null();
                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_init").unwrap(), &[mutex_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init recv condvar at offset 13
                    let recv_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "rcnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[recv_cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init send condvar at offset 19
                    let send_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(19, false)], "scnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[send_cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // is_bounded=0 at offset 25
                    let off25 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(25, false)], "bnd_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off25, i64_type.const_int(0, false))
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

                    // Load is_bounded flag (offset 25)
                    let bnd_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(25, false)], "bnd_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let is_bounded = builder.build_load(i64_type, bnd_ptr, "is_bnd").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let is_bnd_bool = builder.build_int_compare(IntPredicate::NE, is_bounded, i64_type.const_int(0, false), "bnd")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let bounded_bb = context.append_basic_block(llvm_fn, "send_bounded");
                    let unbounded_check_bb = context.append_basic_block(llvm_fn, "send_unbnd_check");
                    let grow_bb = context.append_basic_block(llvm_fn, "send_grow");
                    let write_bb = context.append_basic_block(llvm_fn, "send_write");

                    builder.build_conditional_branch(is_bnd_bool, bounded_bb, unbounded_check_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // -- Bounded path: while count == capacity, wait on send condvar --
                    builder.position_at_end(bounded_bb);
                    let send_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(19, false)], "scnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_ptr_b = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_b = builder.build_load(i64_type, cnt_ptr_b, "cnt").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr_b = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap_b = builder.build_load(i64_type, cap_ptr_b, "cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let full_b = builder.build_int_compare(IntPredicate::EQ, cnt_b, cap_b, "full")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let bnd_wait_bb = context.append_basic_block(llvm_fn, "bnd_wait");
                    let bnd_recheck_bb = context.append_basic_block(llvm_fn, "bnd_recheck");
                    builder.build_conditional_branch(full_b, bnd_wait_bb, write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(bnd_wait_bb);
                    builder.build_call(llvm_module.get_function("pthread_cond_wait").unwrap(), &[send_cond_ptr.into(), mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(bnd_recheck_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(bnd_recheck_bb);
                    let cnt_b2 = builder.build_load(i64_type, cnt_ptr_b, "cnt2").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_b2 = builder.build_load(i64_type, cap_ptr_b, "cap2").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let still_full = builder.build_int_compare(IntPredicate::EQ, cnt_b2, cap_b2, "sfull")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_conditional_branch(still_full, bnd_wait_bb, write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // -- Unbounded path: if count == capacity, grow buffer --
                    builder.position_at_end(unbounded_check_bb);
                    let cnt_ptr_u = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp_u") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_u = builder.build_load(i64_type, cnt_ptr_u, "cnt_u").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr_u = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp_u") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap_u = builder.build_load(i64_type, cap_ptr_u, "cap_u").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let need_grow = builder.build_int_compare(IntPredicate::EQ, cnt_u, cap_u, "needgrow")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_conditional_branch(need_grow, grow_bb, write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // -- Grow buffer: malloc new at 2x, copy elements, free old --
                    builder.position_at_end(grow_bb);
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let free_fn = llvm_module.get_function("free").unwrap();
                    let old_cap = builder.build_load(i64_type, cap_ptr_u, "old_cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let two = i64_type.const_int(2, false);
                    let new_cap = builder.build_int_mul(old_cap, two, "new_cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let eight = i64_type.const_int(8, false);
                    let new_buf_size = builder.build_int_mul(new_cap, eight, "nbs").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_call = builder.build_call(malloc_fn, &[new_buf_size.into()], "nbuf").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_ptr = match new_buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Copy elements from old circular buffer to new linear buffer
                    let old_buf_int = builder.build_load(i64_type, chan_ptr, "obi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let old_buf_ptr = builder.build_int_to_ptr(old_buf_int, ptr_type, "obp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let head_ptr_g = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "hp_g") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let head_g = builder.build_load(i64_type, head_ptr_g, "head_g").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let count_g = builder.build_load(i64_type, cnt_ptr_u, "cnt_g").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

                    // Alloca loop counter
                    let idx_ptr = builder.build_alloca(i64_type, "gidx").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(idx_ptr, i64_type.const_int(0, false)).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let gcond_bb = context.append_basic_block(llvm_fn, "gcond");
                    let gbody_bb = context.append_basic_block(llvm_fn, "gbody");
                    let gdone_bb = context.append_basic_block(llvm_fn, "gdone");

                    builder.build_unconditional_branch(gcond_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.position_at_end(gcond_bb);
                    let gi = builder.build_load(i64_type, idx_ptr, "gi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let gi_lt = builder.build_int_compare(IntPredicate::ULT, gi, count_g, "glt").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_conditional_branch(gi_lt, gbody_bb, gdone_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(gbody_bb);
                    let src_idx = builder.build_int_add(head_g, gi, "si").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let src_idx_mod = builder.build_int_unsigned_rem(src_idx, old_cap, "sim").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let src_gep = unsafe { builder.build_gep(i64_type, old_buf_ptr, &[src_idx_mod], "sg") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let elem = builder.build_load(i64_type, src_gep, "elem").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let dst_gep = unsafe { builder.build_gep(i64_type, new_buf_ptr, &[gi], "dg") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(dst_gep, elem).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let one = i64_type.const_int(1, false);
                    let gi_next = builder.build_int_add(gi, one, "gin").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(idx_ptr, gi_next).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(gcond_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(gdone_bb);
                    // Free old buffer
                    builder.build_call(free_fn, &[old_buf_ptr.into()], "").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Store new buffer ptr
                    let new_buf_int = builder.build_ptr_to_int(new_buf_ptr, i64_type, "nbi").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(chan_ptr, new_buf_int).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Update capacity
                    builder.build_store(cap_ptr_u, new_cap).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Reset head=0, tail=count
                    builder.build_store(head_ptr_g, i64_type.const_int(0, false)).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let tail_ptr_g = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tp_g") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(tail_ptr_g, count_g).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(write_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // -- Write value at buffer[tail % capacity] --
                    builder.position_at_end(write_bb);
                    let tail_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let tail = builder.build_load(i64_type, tail_ptr, "tail").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr_w = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp_w") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap_w = builder.build_load(i64_type, cap_ptr_w, "cap_w").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_int_w = builder.build_load(i64_type, chan_ptr, "bi_w").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_ptr_w = builder.build_int_to_ptr(buf_int_w, ptr_type, "bp_w").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let idx = builder.build_int_unsigned_rem(tail, cap_w, "idx").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let wp = unsafe { builder.build_gep(i64_type, buf_ptr_w, &[idx], "wp") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(wp, val).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Increment tail and count
                    let one = i64_type.const_int(1, false);
                    let new_tail = builder.build_int_add(tail, one, "nt").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(tail_ptr, new_tail).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_ptr_w = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp_w") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_w = builder.build_load(i64_type, cnt_ptr_w, "cnt_w").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let new_cnt = builder.build_int_add(cnt_w, one, "nc").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(cnt_ptr_w, new_cnt).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Signal recv condvar
                    let recv_cond = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "rcnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_signal").unwrap(), &[recv_cond.into()], "")
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
                    let recv_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "rcnd") }
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
                    builder.build_call(llvm_module.get_function("pthread_cond_wait").unwrap(), &[recv_cond_ptr.into(), mutex_ptr.into()], "")
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

                    // Signal send condvar if bounded
                    let bnd_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(25, false)], "bnd_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let is_bounded = builder.build_load(i64_type, bnd_ptr, "is_bnd").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let is_bnd_bool = builder.build_int_compare(IntPredicate::NE, is_bounded, i64_type.const_int(0, false), "bnd")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let signal_bb = context.append_basic_block(llvm_fn, "recv_signal");
                    let unlock_bb = context.append_basic_block(llvm_fn, "recv_unlock");
                    builder.build_conditional_branch(is_bnd_bool, signal_bb, unlock_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(signal_bb);
                    let send_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(19, false)], "scnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_signal").unwrap(), &[send_cond_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(unlock_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(unlock_bb);
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
                Instruction::MutexCreate { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let val = regs[value];

                    // Allocate mutex struct: 9 * 8 = 72 bytes (1 value + 8 mutex)
                    let mtx_call = builder.build_call(malloc_fn, &[i64_type.const_int(72, false).into()], "mtx_alloc")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let mtx_ptr = match mtx_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Store value at offset 0
                    builder.build_store(mtx_ptr, val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init pthread_mutex at offset 1
                    let null = ptr_type.const_null();
                    let mutex_inner = unsafe { builder.build_gep(i64_type, mtx_ptr, &[i64_type.const_int(1, false)], "mi") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_init").unwrap(), &[mutex_inner.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let mtx_int = builder.build_ptr_to_int(mtx_ptr, i64_type, "mtx_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), mtx_int);
                }
                Instruction::MutexLock { dest, mutex } => {
                    let mtx_int = regs[mutex];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let mtx_ptr = builder.build_int_to_ptr(mtx_int, ptr_type, "mtx_p")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Lock at offset 1
                    let mutex_inner = unsafe { builder.build_gep(i64_type, mtx_ptr, &[i64_type.const_int(1, false)], "mi") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_lock").unwrap(), &[mutex_inner.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load value from offset 0
                    let val = builder.build_load(i64_type, mtx_ptr, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), val);
                }
                Instruction::MutexUnlock { mutex, value } => {
                    let mtx_int = regs[mutex];
                    let val = regs[value];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let mtx_ptr = builder.build_int_to_ptr(mtx_int, ptr_type, "mtx_p")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store new value at offset 0
                    builder.build_store(mtx_ptr, val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Unlock at offset 1
                    let mutex_inner = unsafe { builder.build_gep(i64_type, mtx_ptr, &[i64_type.const_int(1, false)], "mi") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_unlock").unwrap(), &[mutex_inner.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::ChannelCreateBounded { tx_dest, rx_dest, capacity } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let cap_val = regs[capacity];

                    // Allocate channel: 26 * 8 = 208 bytes
                    let chan_call = builder.build_call(malloc_fn, &[i64_type.const_int(208, false).into()], "chan")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let chan_ptr = match chan_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Allocate buffer: capacity * 8 bytes
                    let eight = i64_type.const_int(8, false);
                    let buf_size = builder.build_int_mul(cap_val, eight, "bufsz")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_call = builder.build_call(malloc_fn, &[buf_size.into()], "buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Store buffer ptr at offset 0
                    let buf_int: IntValue<'ctx> = builder.build_ptr_to_int(buf_ptr, i64_type, "buf_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(chan_ptr, buf_int)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // capacity at offset 1
                    let off1 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cap_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off1, cap_val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // count=0, head=0, tail=0 at offsets 2-4
                    let off2 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cnt_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off2, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let off3 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "head_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off3, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let off4 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tail_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off4, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init mutex at offset 5
                    let null = ptr_type.const_null();
                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_init").unwrap(), &[mutex_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init recv condvar at offset 13
                    let recv_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "rcnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[recv_cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init send condvar at offset 19
                    let send_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(19, false)], "scnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[send_cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // is_bounded=1 at offset 25
                    let off25 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(25, false)], "bnd_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off25, i64_type.const_int(1, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Both tx and rx are the channel ptr as i64
                    let chan_int = builder.build_ptr_to_int(chan_ptr, i64_type, "chan_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(tx_dest.clone(), chan_int);
                    regs.insert(rx_dest.clone(), chan_int);
                }
                Instruction::ArrayCreate { dest } => {
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();

                    // Allocate 24-byte struct: [data_ptr, len, capacity] (3 x i64)
                    let struct_call = builder.build_call(malloc_fn, &[i64_type.const_int(24, false).into()], "arr_struct")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let struct_ptr = match struct_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Allocate initial buffer: capacity=8, 8*8=64 bytes
                    let buf_call = builder.build_call(malloc_fn, &[i64_type.const_int(64, false).into()], "arr_buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Store buffer ptr (as i64) at offset 0
                    let buf_int = builder.build_ptr_to_int(buf_ptr, i64_type, "buf_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(struct_ptr, buf_int)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store len=0 at offset 1
                    let len_ptr = unsafe { builder.build_gep(i64_type, struct_ptr, &[i64_type.const_int(1, false)], "len_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(len_ptr, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store capacity=8 at offset 2
                    let cap_ptr = unsafe { builder.build_gep(i64_type, struct_ptr, &[i64_type.const_int(2, false)], "cap_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(cap_ptr, i64_type.const_int(8, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Convert struct ptr to i64
                    let arr_int = builder.build_ptr_to_int(struct_ptr, i64_type, "arr_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), arr_int);
                }
                Instruction::ArrayPush { array, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let free_fn = llvm_module.get_function("free").unwrap();
                    let memcpy_fn = llvm_module.get_function("memcpy").unwrap();

                    let arr_int = regs[array];
                    let val = regs[value];
                    let arr_ptr = builder.build_int_to_ptr(arr_int, ptr_type, "arr_p")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load len and capacity
                    let len_ptr = unsafe { builder.build_gep(i64_type, arr_ptr, &[i64_type.const_int(1, false)], "lp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len = builder.build_load(i64_type, len_ptr, "len")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr = unsafe { builder.build_gep(i64_type, arr_ptr, &[i64_type.const_int(2, false)], "cp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap = builder.build_load(i64_type, cap_ptr, "cap")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

                    // Compare len == cap
                    let need_grow = builder.build_int_compare(IntPredicate::EQ, len, cap, "needgrow")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let grow_bb = context.append_basic_block(llvm_fn, "arr_grow");
                    let write_bb = context.append_basic_block(llvm_fn, "arr_write");

                    builder.build_conditional_branch(need_grow, grow_bb, write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // -- grow_bb: malloc(2*cap*8), memcpy, free old, update --
                    builder.position_at_end(grow_bb);
                    let two = i64_type.const_int(2, false);
                    let eight = i64_type.const_int(8, false);
                    let new_cap = builder.build_int_mul(cap, two, "newcap")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_size = builder.build_int_mul(new_cap, eight, "nbs")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_call = builder.build_call(malloc_fn, &[new_buf_size.into()], "nbuf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_ptr = match new_buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Load old data ptr
                    let old_data_int = builder.build_load(i64_type, arr_ptr, "odi")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let old_data_ptr = builder.build_int_to_ptr(old_data_int, ptr_type, "odp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // memcpy old to new (len * 8 bytes)
                    let copy_size = builder.build_int_mul(len, eight, "csz")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(memcpy_fn, &[new_buf_ptr.into(), old_data_ptr.into(), copy_size.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Free old buffer
                    builder.build_call(free_fn, &[old_data_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Update data ptr and capacity
                    let new_buf_int = builder.build_ptr_to_int(new_buf_ptr, i64_type, "nbi")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(arr_ptr, new_buf_int)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(cap_ptr, new_cap)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.build_unconditional_branch(write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // -- write_bb: store value at data[len], increment len --
                    builder.position_at_end(write_bb);

                    // Reload len and data ptr (may have changed after grow)
                    let len2 = builder.build_load(i64_type, len_ptr, "len2")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let data_int2 = builder.build_load(i64_type, arr_ptr, "di2")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let data_ptr2 = builder.build_int_to_ptr(data_int2, ptr_type, "dp2")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store value at data[len]
                    let elem_ptr = unsafe { builder.build_gep(i64_type, data_ptr2, &[len2], "ep") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(elem_ptr, val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Increment len
                    let one = i64_type.const_int(1, false);
                    let new_len = builder.build_int_add(len2, one, "nl")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(len_ptr, new_len)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::ArrayGet { dest, array, index } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_int = regs[array];
                    let idx = regs[index];
                    let arr_ptr = builder.build_int_to_ptr(arr_int, ptr_type, "arr_p")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load data ptr from offset 0
                    let data_int = builder.build_load(i64_type, arr_ptr, "di")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let data_ptr = builder.build_int_to_ptr(data_int, ptr_type, "dp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // GEP to data[index], load value
                    let elem_ptr = unsafe { builder.build_gep(i64_type, data_ptr, &[idx], "ep") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let val = builder.build_load(i64_type, elem_ptr, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), val);
                }
                Instruction::ArraySet { array, index, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_int = regs[array];
                    let idx = regs[index];
                    let val = regs[value];
                    let arr_ptr = builder.build_int_to_ptr(arr_int, ptr_type, "arr_p")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load data ptr from offset 0
                    let data_int = builder.build_load(i64_type, arr_ptr, "di")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let data_ptr = builder.build_int_to_ptr(data_int, ptr_type, "dp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // GEP to data[index], store value
                    let elem_ptr = unsafe { builder.build_gep(i64_type, data_ptr, &[idx], "ep") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(elem_ptr, val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::ArrayLen { dest, array } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_int = regs[array];
                    let arr_ptr = builder.build_int_to_ptr(arr_int, ptr_type, "arr_p")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load len from offset 1
                    let len_ptr = unsafe { builder.build_gep(i64_type, arr_ptr, &[i64_type.const_int(1, false)], "lp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len = builder.build_load(i64_type, len_ptr, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), len);
                }
                Instruction::StringLen { dest, string } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let strlen_fn = llvm_module.get_function("strlen").unwrap();

                    // String may be in ptrs (literal) or regs (from concat/etc)
                    let str_ptr = if let Some(p) = ptrs.get(string) {
                        *p
                    } else {
                        let str_int = regs[string];
                        builder.build_int_to_ptr(str_int, ptr_type, "sp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    let len_call = builder.build_call(strlen_fn, &[str_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len = match len_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("strlen returned void".into())),
                    };
                    regs.insert(dest.clone(), len);
                }
                Instruction::StringConcat { dest, left, right } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let strlen_fn = llvm_module.get_function("strlen").unwrap();
                    let memcpy_fn = llvm_module.get_function("memcpy").unwrap();

                    // Get left string ptr
                    let left_ptr = if let Some(p) = ptrs.get(left) {
                        *p
                    } else {
                        let li = regs[left];
                        builder.build_int_to_ptr(li, ptr_type, "lp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    // Get right string ptr
                    let right_ptr = if let Some(p) = ptrs.get(right) {
                        *p
                    } else {
                        let ri = regs[right];
                        builder.build_int_to_ptr(ri, ptr_type, "rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    // strlen(left)
                    let len1_call = builder.build_call(strlen_fn, &[left_ptr.into()], "len1")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len1 = match len1_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("strlen returned void".into())),
                    };

                    // strlen(right)
                    let len2_call = builder.build_call(strlen_fn, &[right_ptr.into()], "len2")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len2 = match len2_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("strlen returned void".into())),
                    };

                    // malloc(len1 + len2 + 1)
                    let total = builder.build_int_add(len1, len2, "total")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let one = i64_type.const_int(1, false);
                    let alloc_size = builder.build_int_add(total, one, "asz")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_call = builder.build_call(malloc_fn, &[alloc_size.into()], "buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // memcpy left to buf
                    builder.build_call(memcpy_fn, &[buf_ptr.into(), left_ptr.into(), len1.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // GEP buf+len1 for mid
                    let i8_type = context.i8_type();
                    let mid_ptr = unsafe { builder.build_gep(i8_type, buf_ptr, &[len1], "mid") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // memcpy right to mid
                    builder.build_call(memcpy_fn, &[mid_ptr.into(), right_ptr.into(), len2.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // null-terminate at buf[total]
                    let end_ptr = unsafe { builder.build_gep(i8_type, buf_ptr, &[total], "end") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(end_ptr, i8_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store result as i64 in regs AND as ptr in ptrs
                    let result_int = builder.build_ptr_to_int(buf_ptr, i64_type, "cat_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result_int);
                    ptrs.insert(dest.clone(), buf_ptr);
                }
                Instruction::StringSubstring { dest, string, start, end } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let memcpy_fn = llvm_module.get_function("memcpy").unwrap();

                    // Get string ptr
                    let str_ptr = if let Some(p) = ptrs.get(string) {
                        *p
                    } else {
                        let si = regs[string];
                        builder.build_int_to_ptr(si, ptr_type, "sp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    let start_val = regs[start];
                    let end_val = regs[end];

                    // length = end - start
                    let length = builder.build_int_sub(end_val, start_val, "sublen")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // malloc(length + 1)
                    let one = i64_type.const_int(1, false);
                    let alloc_size = builder.build_int_add(length, one, "asz")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_call = builder.build_call(malloc_fn, &[alloc_size.into()], "subbuf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // GEP str_ptr+start for src
                    let i8_type = context.i8_type();
                    let src_ptr = unsafe { builder.build_gep(i8_type, str_ptr, &[start_val], "src") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // memcpy src to buf
                    builder.build_call(memcpy_fn, &[buf_ptr.into(), src_ptr.into(), length.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // null-terminate
                    let end_ptr = unsafe { builder.build_gep(i8_type, buf_ptr, &[length], "endp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(end_ptr, i8_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store in both regs and ptrs
                    let result_int = builder.build_ptr_to_int(buf_ptr, i64_type, "sub_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result_int);
                    ptrs.insert(dest.clone(), buf_ptr);
                }
                Instruction::IntToString { dest, value } => {
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let snprintf_fn = llvm_module.get_function("snprintf").unwrap();

                    let val = regs[value];

                    // malloc(21) — enough for i64 including sign and null
                    let buf_call = builder.build_call(malloc_fn, &[i64_type.const_int(21, false).into()], "itsbuf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Build format string "%ld"
                    let fmt_str = builder.build_global_string_ptr("%ld", "its_fmt")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Call snprintf(buf, 21, "%ld", val)
                    builder.build_call(snprintf_fn, &[buf_ptr.into(), i64_type.const_int(21, false).into(), fmt_str.as_pointer_value().into(), val.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store in both regs and ptrs
                    let result_int = builder.build_ptr_to_int(buf_ptr, i64_type, "its_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result_int);
                    ptrs.insert(dest.clone(), buf_ptr);
                }
                Instruction::StringToInt { dest, string } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let strtol_fn = llvm_module.get_function("strtol").unwrap();

                    // Get string ptr
                    let str_ptr = if let Some(p) = ptrs.get(string) {
                        *p
                    } else {
                        let si = regs[string];
                        builder.build_int_to_ptr(si, ptr_type, "sp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    // Call strtol(ptr, null, 10)
                    let null = ptr_type.const_null();
                    let ten = i64_type.const_int(10, false);
                    let result_call = builder.build_call(strtol_fn, &[str_ptr.into(), null.into(), ten.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match result_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("strtol returned void".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::FileExists { dest, path } => {
                    let access_fn = llvm_module.get_function("access").unwrap();
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());

                    // Get path pointer
                    let path_ptr = if let Some(p) = ptrs.get(path) {
                        *p
                    } else {
                        let pi = regs[path];
                        builder.build_int_to_ptr(pi, ptr_type, "fep")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    // Call access(path, 0) — F_OK = 0
                    let zero_i32 = i32_type.const_int(0, false);
                    let access_call = builder.build_call(access_fn, &[path_ptr.into(), zero_i32.into()], "access_ret")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let access_ret = match access_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("access returned void".into())),
                    };

                    // result = (access_ret == 0) ? 1 : 0
                    let is_ok = builder.build_int_compare(inkwell::IntPredicate::EQ, access_ret, zero_i32, "fe_ok")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = builder.build_int_z_extend(is_ok, i64_type, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result);
                }
                Instruction::FileRead { dest, path } => {
                    let fopen_fn = llvm_module.get_function("fopen").unwrap();
                    let fclose_fn = llvm_module.get_function("fclose").unwrap();
                    let fread_fn = llvm_module.get_function("fread").unwrap();
                    let fseek_fn = llvm_module.get_function("fseek").unwrap();
                    let ftell_fn = llvm_module.get_function("ftell").unwrap();
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let i8_type = context.i8_type();

                    // Get path pointer
                    let path_ptr = if let Some(p) = ptrs.get(path) {
                        *p
                    } else {
                        let pi = regs[path];
                        builder.build_int_to_ptr(pi, ptr_type, "frp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    // Build mode string "r"
                    let read_mode = builder.build_global_string_ptr("r", "read_mode")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Call fopen(path, "r")
                    let fopen_call = builder.build_call(fopen_fn, &[path_ptr.into(), read_mode.as_pointer_value().into()], "file_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let file_ptr = match fopen_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("fopen returned void".into())),
                    };

                    // Check if null
                    let file_int = builder.build_ptr_to_int(file_ptr, i64_type, "file_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let null_int = builder.build_ptr_to_int(ptr_type.const_null(), i64_type, "null_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let is_null = builder.build_int_compare(inkwell::IntPredicate::EQ, file_int, null_int, "is_null")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let pre_branch_bb = builder.get_insert_block().unwrap();
                    let then_bb = context.append_basic_block(llvm_fn, "fr_ok");
                    let error_bb = context.append_basic_block(llvm_fn, "fr_err");
                    let merge_bb = context.append_basic_block(llvm_fn, "fr_merge");

                    builder.build_conditional_branch(is_null, error_bb, then_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Error path: return empty string
                    builder.position_at_end(error_bb);
                    let empty_call = builder.build_call(malloc_fn, &[i64_type.const_int(1, false).into()], "empty_buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let empty_ptr = match empty_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };
                    builder.build_store(empty_ptr, i8_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let empty_int = builder.build_ptr_to_int(empty_ptr, i64_type, "empty_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(merge_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let error_end_bb = builder.get_insert_block().unwrap();

                    // Success path: fseek/ftell/fread
                    builder.position_at_end(then_bb);
                    let seek_end = i32_type.const_int(2, false); // SEEK_END
                    let seek_set = i32_type.const_int(0, false); // SEEK_SET
                    builder.build_call(fseek_fn, &[file_ptr.into(), i64_type.const_int(0, false).into(), seek_end.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ftell_call = builder.build_call(ftell_fn, &[file_ptr.into()], "fsize")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let file_size = match ftell_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("ftell returned void".into())),
                    };
                    builder.build_call(fseek_fn, &[file_ptr.into(), i64_type.const_int(0, false).into(), seek_set.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // malloc(size + 1) for buffer
                    let buf_size = builder.build_int_add(file_size, i64_type.const_int(1, false), "buf_size")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_call = builder.build_call(malloc_fn, &[buf_size.into()], "frbuf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // fread(buf, 1, size, file)
                    builder.build_call(fread_fn, &[buf_ptr.into(), i64_type.const_int(1, false).into(), file_size.into(), file_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Null-terminate: buf[size] = 0
                    let end_ptr = unsafe { builder.build_gep(i8_type, buf_ptr, &[file_size], "endp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(end_ptr, i8_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // fclose(file)
                    builder.build_call(fclose_fn, &[file_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let buf_int = builder.build_ptr_to_int(buf_ptr, i64_type, "buf_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(merge_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let then_end_bb = builder.get_insert_block().unwrap();

                    // Merge: phi node
                    builder.position_at_end(merge_bb);
                    let phi = builder.build_phi(i64_type, "read_result")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    phi.add_incoming(&[(&empty_int, error_end_bb), (&buf_int, then_end_bb)]);
                    let result_int = phi.as_basic_value().into_int_value();
                    let result_ptr = builder.build_int_to_ptr(result_int, ptr_type, "fr_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result_int);
                    ptrs.insert(dest.clone(), result_ptr);

                    let _ = pre_branch_bb; // suppress unused warning
                }
                Instruction::FileWrite { dest, path, content } => {
                    let fopen_fn = llvm_module.get_function("fopen").unwrap();
                    let fclose_fn = llvm_module.get_function("fclose").unwrap();
                    let fwrite_fn = llvm_module.get_function("fwrite").unwrap();
                    let strlen_fn = llvm_module.get_function("strlen").unwrap();
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());

                    // Get path and content pointers
                    let path_ptr = if let Some(p) = ptrs.get(path) {
                        *p
                    } else {
                        let pi = regs[path];
                        builder.build_int_to_ptr(pi, ptr_type, "fwpp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let content_ptr = if let Some(p) = ptrs.get(content) {
                        *p
                    } else {
                        let ci = regs[content];
                        builder.build_int_to_ptr(ci, ptr_type, "fwcp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    // strlen(content)
                    let strlen_call = builder.build_call(strlen_fn, &[content_ptr.into()], "fw_len")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let content_len = match strlen_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("strlen returned void".into())),
                    };

                    // fopen(path, "w")
                    let write_mode = builder.build_global_string_ptr("w", "write_mode")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let fopen_call = builder.build_call(fopen_fn, &[path_ptr.into(), write_mode.as_pointer_value().into()], "fw_file")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let file_ptr = match fopen_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("fopen returned void".into())),
                    };

                    // Null check
                    let file_int = builder.build_ptr_to_int(file_ptr, i64_type, "fw_fi")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let null_int = builder.build_ptr_to_int(ptr_type.const_null(), i64_type, "fw_ni")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let is_null = builder.build_int_compare(inkwell::IntPredicate::EQ, file_int, null_int, "fw_null")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let pre_branch_bb = builder.get_insert_block().unwrap();
                    let ok_bb = context.append_basic_block(llvm_fn, "fw_ok");
                    let merge_bb = context.append_basic_block(llvm_fn, "fw_merge");

                    builder.build_conditional_branch(is_null, merge_bb, ok_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Success path: fwrite + fclose
                    builder.position_at_end(ok_bb);
                    builder.build_call(fwrite_fn, &[content_ptr.into(), i64_type.const_int(1, false).into(), content_len.into(), file_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(fclose_fn, &[file_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(merge_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ok_end_bb = builder.get_insert_block().unwrap();

                    // Merge: phi(0 on null, 1 on success)
                    builder.position_at_end(merge_bb);
                    let phi = builder.build_phi(i64_type, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    phi.add_incoming(&[
                        (&i64_type.const_int(0, false), pre_branch_bb),
                        (&i64_type.const_int(1, false), ok_end_bb),
                    ]);
                    regs.insert(dest.clone(), phi.as_basic_value().into_int_value());
                }
                Instruction::FileAppend { dest, path, content } => {
                    let fopen_fn = llvm_module.get_function("fopen").unwrap();
                    let fclose_fn = llvm_module.get_function("fclose").unwrap();
                    let fwrite_fn = llvm_module.get_function("fwrite").unwrap();
                    let strlen_fn = llvm_module.get_function("strlen").unwrap();
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());

                    // Get path and content pointers
                    let path_ptr = if let Some(p) = ptrs.get(path) {
                        *p
                    } else {
                        let pi = regs[path];
                        builder.build_int_to_ptr(pi, ptr_type, "fapp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let content_ptr = if let Some(p) = ptrs.get(content) {
                        *p
                    } else {
                        let ci = regs[content];
                        builder.build_int_to_ptr(ci, ptr_type, "facp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };

                    // strlen(content)
                    let strlen_call = builder.build_call(strlen_fn, &[content_ptr.into()], "fa_len")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let content_len = match strlen_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("strlen returned void".into())),
                    };

                    // fopen(path, "a")
                    let append_mode = builder.build_global_string_ptr("a", "append_mode")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let fopen_call = builder.build_call(fopen_fn, &[path_ptr.into(), append_mode.as_pointer_value().into()], "fa_file")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let file_ptr = match fopen_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("fopen returned void".into())),
                    };

                    // Null check
                    let file_int = builder.build_ptr_to_int(file_ptr, i64_type, "fa_fi")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let null_int = builder.build_ptr_to_int(ptr_type.const_null(), i64_type, "fa_ni")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let is_null = builder.build_int_compare(inkwell::IntPredicate::EQ, file_int, null_int, "fa_null")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let pre_branch_bb = builder.get_insert_block().unwrap();
                    let ok_bb = context.append_basic_block(llvm_fn, "fa_ok");
                    let merge_bb = context.append_basic_block(llvm_fn, "fa_merge");

                    builder.build_conditional_branch(is_null, merge_bb, ok_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Success path: fwrite + fclose
                    builder.position_at_end(ok_bb);
                    builder.build_call(fwrite_fn, &[content_ptr.into(), i64_type.const_int(1, false).into(), content_len.into(), file_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(fclose_fn, &[file_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(merge_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ok_end_bb = builder.get_insert_block().unwrap();

                    // Merge: phi(0 on null, 1 on success)
                    builder.position_at_end(merge_bb);
                    let phi = builder.build_phi(i64_type, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    phi.add_incoming(&[
                        (&i64_type.const_int(0, false), pre_branch_bb),
                        (&i64_type.const_int(1, false), ok_end_bb),
                    ]);
                    regs.insert(dest.clone(), phi.as_basic_value().into_int_value());
                }
                Instruction::JsonParse { dest, source } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let src_ptr = if let Some(p) = ptrs.get(source) {
                        *p
                    } else {
                        let iv = regs[source];
                        builder.build_int_to_ptr(iv, ptr_type, "jp_sp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_parse").unwrap();
                    let call = builder.build_call(fn_ref, &[src_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_parse: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jp_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonObject { dest } => {
                    let fn_ref = llvm_module.get_function("cy_json_object").unwrap();
                    let call = builder.build_call(fn_ref, &[], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_object: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jo_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonArray { dest } => {
                    let fn_ref = llvm_module.get_function("cy_json_array").unwrap();
                    let call = builder.build_call(fn_ref, &[], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_array: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "ja_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonString { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jstr_sp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_string").unwrap();
                    let call = builder.build_call(fn_ref, &[val_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_string: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jstr_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonInt { dest, value } => {
                    let val = regs[value];
                    let fn_ref = llvm_module.get_function("cy_json_int").unwrap();
                    let call = builder.build_call(fn_ref, &[val.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_int: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jint_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonBool { dest, value } => {
                    let val = regs[value];
                    let fn_ref = llvm_module.get_function("cy_json_bool").unwrap();
                    let call = builder.build_call(fn_ref, &[val.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_bool: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jbool_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonNull { dest } => {
                    let fn_ref = llvm_module.get_function("cy_json_null").unwrap();
                    let call = builder.build_call(fn_ref, &[], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_null: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jnull_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonStringify { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jsfy_sp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_stringify").unwrap();
                    let call = builder.build_call(fn_ref, &[val_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_stringify: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jsfy_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonGet { dest, object, key } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let obj_ptr = if let Some(p) = ptrs.get(object) {
                        *p
                    } else {
                        let iv = regs[object];
                        builder.build_int_to_ptr(iv, ptr_type, "jget_op")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let key_ptr = if let Some(p) = ptrs.get(key) {
                        *p
                    } else {
                        let iv = regs[key];
                        builder.build_int_to_ptr(iv, ptr_type, "jget_kp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_get").unwrap();
                    let call = builder.build_call(fn_ref, &[obj_ptr.into(), key_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_get: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jget_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonGetIndex { dest, array, index } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = if let Some(p) = ptrs.get(array) {
                        *p
                    } else {
                        let iv = regs[array];
                        builder.build_int_to_ptr(iv, ptr_type, "jgi_ap")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let idx_val = regs[index];
                    let fn_ref = llvm_module.get_function("cy_json_get_index").unwrap();
                    let call = builder.build_call(fn_ref, &[arr_ptr.into(), idx_val.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_get_index: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jgi_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonGetString { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jgs_vp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_get_string").unwrap();
                    let call = builder.build_call(fn_ref, &[val_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_get_string: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jgs_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonGetInt { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jgi_vp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_get_int").unwrap();
                    let call = builder.build_call(fn_ref, &[val_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_get_int: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::JsonGetBool { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jgb_vp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_get_bool").unwrap();
                    let call = builder.build_call(fn_ref, &[val_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_get_bool: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::JsonLen { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jlen_vp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_len").unwrap();
                    let call = builder.build_call(fn_ref, &[val_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_len: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::JsonTypeOf { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jto_vp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_type_of").unwrap();
                    let call = builder.build_call(fn_ref, &[val_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_json_type_of: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "jto_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::JsonSet { object, key, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let obj_ptr = if let Some(p) = ptrs.get(object) {
                        *p
                    } else {
                        let iv = regs[object];
                        builder.build_int_to_ptr(iv, ptr_type, "jset_op")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let key_ptr = if let Some(p) = ptrs.get(key) {
                        *p
                    } else {
                        let iv = regs[key];
                        builder.build_int_to_ptr(iv, ptr_type, "jset_kp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jset_vp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_set").unwrap();
                    builder.build_call(fn_ref, &[obj_ptr.into(), key_ptr.into(), val_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::JsonPush { array, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = if let Some(p) = ptrs.get(array) {
                        *p
                    } else {
                        let iv = regs[array];
                        builder.build_int_to_ptr(iv, ptr_type, "jpush_ap")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let val_ptr = if let Some(p) = ptrs.get(value) {
                        *p
                    } else {
                        let iv = regs[value];
                        builder.build_int_to_ptr(iv, ptr_type, "jpush_vp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_json_push").unwrap();
                    builder.build_call(fn_ref, &[arr_ptr.into(), val_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::ResultOk { dest, value } => {
                    let val = regs[value];
                    let fn_ref = llvm_module.get_function("cy_result_ok").unwrap();
                    let call = builder.build_call(fn_ref, &[val.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_result_ok: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "rok_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::ResultErr { dest, message } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let msg_ptr = if let Some(p) = ptrs.get(message) {
                        *p
                    } else {
                        let iv = regs[message];
                        builder.build_int_to_ptr(iv, ptr_type, "rerr_mp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_result_err").unwrap();
                    let call = builder.build_call(fn_ref, &[msg_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_result_err: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "rerr_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::ResultIsOk { dest, result } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let res_ptr = if let Some(p) = ptrs.get(result) {
                        *p
                    } else {
                        let iv = regs[result];
                        builder.build_int_to_ptr(iv, ptr_type, "risok_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_result_is_ok").unwrap();
                    let call = builder.build_call(fn_ref, &[res_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_result_is_ok: expected return".into())),
                    };
                    regs.insert(dest.clone(), result_val);
                }
                Instruction::ResultIsErr { dest, result } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let res_ptr = if let Some(p) = ptrs.get(result) {
                        *p
                    } else {
                        let iv = regs[result];
                        builder.build_int_to_ptr(iv, ptr_type, "riserr_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_result_is_err").unwrap();
                    let call = builder.build_call(fn_ref, &[res_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_result_is_err: expected return".into())),
                    };
                    regs.insert(dest.clone(), result_val);
                }
                Instruction::ResultUnwrap { dest, result } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let res_ptr = if let Some(p) = ptrs.get(result) {
                        *p
                    } else {
                        let iv = regs[result];
                        builder.build_int_to_ptr(iv, ptr_type, "runwrap_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_result_unwrap").unwrap();
                    let call = builder.build_call(fn_ref, &[res_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_result_unwrap: expected return".into())),
                    };
                    regs.insert(dest.clone(), result_val);
                    // For Result<String>, the value is a pointer - also store in ptrs
                    let as_ptr = builder.build_int_to_ptr(result_val, ptr_type, "runwrap_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    ptrs.insert(dest.clone(), as_ptr);
                }
                Instruction::ResultUnwrapOr { dest, result, default } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let res_ptr = if let Some(p) = ptrs.get(result) {
                        *p
                    } else {
                        let iv = regs[result];
                        builder.build_int_to_ptr(iv, ptr_type, "runwrapor_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let default_val = regs[default];
                    let fn_ref = llvm_module.get_function("cy_result_unwrap_or").unwrap();
                    let call = builder.build_call(fn_ref, &[res_ptr.into(), default_val.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_result_unwrap_or: expected return".into())),
                    };
                    regs.insert(dest.clone(), result_val);
                    let as_ptr = builder.build_int_to_ptr(result_val, ptr_type, "runwrapor_ptr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    ptrs.insert(dest.clone(), as_ptr);
                }
                Instruction::ResultError { dest, result } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let res_ptr = if let Some(p) = ptrs.get(result) {
                        *p
                    } else {
                        let iv = regs[result];
                        builder.build_int_to_ptr(iv, ptr_type, "rerror_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_result_error").unwrap();
                    let call = builder.build_call(fn_ref, &[res_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_result_error: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "rerror_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::LogDebug { dest, message } | Instruction::LogInfo { dest, message } | Instruction::LogWarn { dest, message } | Instruction::LogError { dest, message } => {
                    let fn_name = match instr {
                        Instruction::LogDebug { .. } => "cy_log_debug",
                        Instruction::LogInfo { .. } => "cy_log_info",
                        Instruction::LogWarn { .. } => "cy_log_warn",
                        Instruction::LogError { .. } => "cy_log_error",
                        _ => unreachable!(),
                    };
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let msg_ptr = if let Some(p) = ptrs.get(message) {
                        *p
                    } else {
                        let iv = regs[message];
                        builder.build_int_to_ptr(iv, ptr_type, "log_mp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function(fn_name).unwrap();
                    let call = builder.build_call(fn_ref, &[msg_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError(format!("{}: expected return", fn_name))),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::LogSetLevel { dest, level } => {
                    let level_val = regs[level];
                    let fn_ref = llvm_module.get_function("cy_log_set_level").unwrap();
                    let call = builder.build_call(fn_ref, &[level_val.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_log_set_level: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::HttpGet { dest, url } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let url_ptr = if let Some(p) = ptrs.get(url) {
                        *p
                    } else {
                        let iv = regs[url];
                        builder.build_int_to_ptr(iv, ptr_type, "hget_up")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_http_get").unwrap();
                    let call = builder.build_call(fn_ref, &[url_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_get: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "hget_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::HttpPost { dest, url, body, content_type } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let url_ptr = if let Some(p) = ptrs.get(url) {
                        *p
                    } else {
                        let iv = regs[url];
                        builder.build_int_to_ptr(iv, ptr_type, "hpost_up")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let body_ptr = if let Some(p) = ptrs.get(body) {
                        *p
                    } else {
                        let iv = regs[body];
                        builder.build_int_to_ptr(iv, ptr_type, "hpost_bp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let ct_ptr = if let Some(p) = ptrs.get(content_type) {
                        *p
                    } else {
                        let iv = regs[content_type];
                        builder.build_int_to_ptr(iv, ptr_type, "hpost_cp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_http_post").unwrap();
                    let call = builder.build_call(fn_ref, &[url_ptr.into(), body_ptr.into(), ct_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_post: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "hpost_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::HttpStatus { dest, response } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let resp_ptr = if let Some(p) = ptrs.get(response) {
                        *p
                    } else {
                        let iv = regs[response];
                        builder.build_int_to_ptr(iv, ptr_type, "hstat_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_http_status").unwrap();
                    let call = builder.build_call(fn_ref, &[resp_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_status: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::HttpBody { dest, response } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let resp_ptr = if let Some(p) = ptrs.get(response) {
                        *p
                    } else {
                        let iv = regs[response];
                        builder.build_int_to_ptr(iv, ptr_type, "hbody_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_http_body").unwrap();
                    let call = builder.build_call(fn_ref, &[resp_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_body: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "hbody_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::HttpHeader { dest, response, name } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let resp_ptr = if let Some(p) = ptrs.get(response) {
                        *p
                    } else {
                        let iv = regs[response];
                        builder.build_int_to_ptr(iv, ptr_type, "hhdr_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let name_ptr = if let Some(p) = ptrs.get(name) {
                        *p
                    } else {
                        let iv = regs[name];
                        builder.build_int_to_ptr(iv, ptr_type, "hhdr_np")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_http_header").unwrap();
                    let call = builder.build_call(fn_ref, &[resp_ptr.into(), name_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ptr_val = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_header: expected return".into())),
                    };
                    let as_int = builder.build_ptr_to_int(ptr_val, i64_type, "hhdr_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), as_int);
                    ptrs.insert(dest.clone(), ptr_val);
                }
                Instruction::HttpOk { dest, response } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let resp_ptr = if let Some(p) = ptrs.get(response) {
                        *p
                    } else {
                        let iv = regs[response];
                        builder.build_int_to_ptr(iv, ptr_type, "hok_rp")
                            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
                    };
                    let fn_ref = llvm_module.get_function("cy_http_ok").unwrap();
                    let call = builder.build_call(fn_ref, &[resp_ptr.into()], dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("cy_http_ok: expected return".into())),
                    };
                    regs.insert(dest.clone(), result);
                }
            }
        }
    }

    Ok(llvm_module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sans_ir::ir::{Instruction, IrBinOp, IrCmpOp, IrFunction, Module};

    #[test]
    fn codegen_while_loop() {
        let program = sans_parser::parse(
            "fn main() Int { let mut x Int = 0 while x < 3 { x = x + 1 } x }"
        ).expect("parse failed");
        let module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let ir = compile_to_llvm_ir(&module).expect("codegen failed");

        assert!(ir.contains("alloca"), "expected alloca in:\n{}", ir);
        assert!(ir.contains("br "), "expected branch in:\n{}", ir);
        assert!(ir.contains("ret i64"), "expected ret in:\n{}", ir);
    }

    #[test]
    fn codegen_print() {
        let program = sans_parser::parse(r#"fn main() Int { print("hello") }"#).expect("parse failed");
        let module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
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
        let program = sans_parser::parse(
            "struct Point { x Int, y Int, } fn main() Int { let p = Point { x: 3, y: 4 } p.x + p.y }"
        ).expect("parse failed");
        let module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        assert!(ir.contains("alloca"), "expected alloca in:\n{}", ir);
        assert!(ir.contains("getelementptr"), "expected GEP in:\n{}", ir);
        assert!(ir.contains("ret i64"), "expected ret in:\n{}", ir);
    }

    #[test]
    fn codegen_enum_match() {
        let program = sans_parser::parse(
            "enum Color { Red, Green, Blue, } fn main() Int { let c = Color::Green match c { Color::Red => 1, Color::Green => 2, Color::Blue => 3, } }"
        ).expect("parse failed");
        let module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
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
        let program = sans_parser::parse(
            "struct Point { x Int, y Int, } impl Point { fn sum(self) Int { self.x + self.y } } fn main() Int { let p = Point { x: 3, y: 4 } p.sum() }"
        ).expect("parse failed");
        sans_typeck::check(&program, &std::collections::HashMap::new()).expect("type error");
        let module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let ir = compile_to_llvm_ir(&module).expect("codegen failed");
        assert!(ir.contains("define i64 @Point_sum"), "expected Point_sum function in:\n{}", ir);
        assert!(ir.contains("call i64 @Point_sum"), "expected call to Point_sum in:\n{}", ir);
    }

    #[test]
    fn codegen_channel_create() {
        let program = sans_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() 0 }").expect("parse");
        sans_typeck::check(&program, &std::collections::HashMap::new()).expect("typeck");
        let module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let ir = compile_to_llvm_ir(&module).expect("codegen");
        assert!(ir.contains("malloc"), "expected malloc in:\n{}", ir);
        assert!(ir.contains("pthread_mutex_init"), "expected mutex_init in:\n{}", ir);
    }

    #[test]
    fn codegen_spawn() {
        let program = sans_parser::parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() h.join() }").expect("parse");
        sans_typeck::check(&program, &std::collections::HashMap::new()).expect("typeck");
        let module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let ir = compile_to_llvm_ir(&module).expect("codegen");
        assert!(ir.contains("pthread_create"), "expected pthread_create in:\n{}", ir);
        assert!(ir.contains("pthread_join"), "expected pthread_join in:\n{}", ir);
        assert!(ir.contains("__trampoline_worker"), "expected trampoline in:\n{}", ir);
    }

    #[test]
    fn codegen_send_recv() {
        let program = sans_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }").expect("parse");
        sans_typeck::check(&program, &std::collections::HashMap::new()).expect("typeck");
        let module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let ir = compile_to_llvm_ir(&module).expect("codegen");
        assert!(ir.contains("pthread_mutex_lock"), "expected lock in:\n{}", ir);
        assert!(ir.contains("pthread_cond_signal"), "expected signal in:\n{}", ir);
        assert!(ir.contains("pthread_cond_wait"), "expected wait in:\n{}", ir);
    }

    #[test]
    fn codegen_mutex_create_lock_unlock() {
        let program = sans_parser::parse(
            "fn main() Int { let m = mutex(5) let v = m.lock() m.unlock(v) 0 }"
        ).unwrap();
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }

    #[test]
    fn codegen_bounded_channel() {
        let program = sans_parser::parse(
            "fn main() Int { let (tx, rx) = channel<Int>(4) tx.send(1) rx.recv() }"
        ).unwrap();
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }

    #[test]
    fn codegen_array_create_push_get_len() {
        let program = sans_parser::parse(
            "fn main() Int { let a = array<Int>() a.push(5) a.get(0) }"
        ).unwrap();
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }

    #[test]
    fn codegen_string_concat() {
        let program = sans_parser::parse(
            r#"fn main() Int { let s = "hello" + " world" 0 }"#
        ).unwrap();
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }

    #[test]
    fn codegen_int_to_string_and_string_to_int() {
        let program = sans_parser::parse(
            r#"fn main() Int { let s = int_to_string(42) string_to_int(s) }"#
        ).unwrap();
        sans_typeck::check(&program, &std::collections::HashMap::new()).unwrap();
        let ir = sans_ir::lower(&program, None, &std::collections::HashMap::new());
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }
}
