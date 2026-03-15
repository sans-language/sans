use std::process::Command;

/// Helper: compile a single C runtime file, returning the path to the object file.
fn compile_runtime(manifest_dir: &str, tmp_dir: &std::path::Path, name: &str, fixture_id: &str) -> std::path::PathBuf {
    let c_path = format!("{}/../../runtime/{}.c", manifest_dir, name);
    let o_path = tmp_dir.join(format!("{}_{}.o", fixture_id, name));
    let compile = Command::new("cc")
        .args(["-c", &c_path, "-o", o_path.to_str().unwrap()])
        .status()
        .unwrap_or_else(|_| panic!("failed to compile {} runtime", name));
    assert!(compile.success(), "{} runtime compilation failed", name);
    o_path
}

/// Helper: compile a single Sans runtime file, returning the path to the object file.
fn compile_sans_runtime(manifest_dir: &str, tmp_dir: &std::path::Path, name: &str, fixture_id: &str) -> std::path::PathBuf {
    let sans_path = format!("{}/../../runtime/{}.sans", manifest_dir, name);
    let o_path = tmp_dir.join(format!("{}_{}.o", fixture_id, name));

    let source = std::fs::read_to_string(&sans_path)
        .unwrap_or_else(|e| panic!("could not read runtime {}.sans: {}", name, e));
    let program = sans_parser::parse(&source)
        .unwrap_or_else(|e| panic!("parse error in runtime {}: {}", name, e.message));
    let module_exports = std::collections::HashMap::new();
    sans_typeck::check_module(&program, &module_exports)
        .unwrap_or_else(|e| panic!("type error in runtime {}: {}", name, e.message));
    let module_fn_ret_types = std::collections::HashMap::new();
    let ir = sans_ir::lower(&program, None, &module_fn_ret_types);
    sans_codegen::compile_to_object(&ir, o_path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("codegen error in runtime {}: {}", name, e));
    o_path
}

const C_RUNTIME_NAMES: &[&str] = &[];
const SANS_RUNTIME_NAMES: &[&str] = &["log", "result", "functional", "array_ext", "string_ext", "http", "server", "json", "sock", "curl", "map", "arena"];

/// Helper: compile a multi-file fixture directory and run main.sans, returning the exit code.
fn compile_and_run_dir(fixture_dir: &str) -> i32 {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dir_path = format!("{}/../../tests/fixtures/{}", manifest_dir, fixture_dir);
    let main_path = std::path::PathBuf::from(format!("{}/main.sans", dir_path));

    // Resolve imports
    let resolved_modules = sans::imports::resolve_imports(&main_path)
        .unwrap_or_else(|e| panic!("import resolution error: {}", e));

    // Parse main
    let main_source = std::fs::read_to_string(&main_path)
        .unwrap_or_else(|e| panic!("could not read main.sans: {}", e));
    let main_program = sans_parser::parse(&main_source)
        .unwrap_or_else(|e| panic!("parse error: {:?}", e));

    // Type-check in dependency order
    let mut module_exports = std::collections::HashMap::new();
    for module in &resolved_modules {
        let exports = sans_typeck::check_module(&module.program, &module_exports)
            .unwrap_or_else(|e| panic!("type error in module '{}': {}", module.name, e.message));
        module_exports.insert(module.name.clone(), exports);
    }

    sans_typeck::check(&main_program, &module_exports)
        .unwrap_or_else(|e| panic!("type error: {}", e.message));

    // Build module_fn_ret_types
    let mut module_fn_ret_types: std::collections::HashMap<(String, String), sans_ir::IrType> =
        std::collections::HashMap::new();
    for (mod_name, exports) in &module_exports {
        for (func_name, sig) in &exports.functions {
            let ir_type = sans_ir::ir_type_for_return(&sig.return_type);
            module_fn_ret_types.insert((mod_name.clone(), func_name.clone()), ir_type);
        }
    }

    // Build extra struct defs from module exports for cross-module field access
    let mut extra_struct_defs: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for exports in module_exports.values() {
        for (struct_name, fields) in &exports.structs {
            let field_names: Vec<String> = fields.iter().map(|(name, _)| name.clone()).collect();
            extra_struct_defs.insert(struct_name.clone(), field_names);
        }
    }

    // Lower + merge
    let mut all_ir_functions = Vec::new();
    for module in &resolved_modules {
        let ir = sans_ir::lower(&module.program, Some(&module.name), &module_fn_ret_types);
        all_ir_functions.extend(ir.functions);
    }
    let main_ir = sans_ir::lower_with_extra_structs(&main_program, None, &module_fn_ret_types, &extra_struct_defs);
    let all_globals = main_ir.globals;
    all_ir_functions.extend(main_ir.functions);

    let merged = sans_ir::ir::Module { globals: all_globals, functions: all_ir_functions };

    // Codegen, link, run
    let tmp_dir = std::env::temp_dir();
    let obj_path = tmp_dir.join(format!("{}.o", fixture_dir));
    let bin_path = tmp_dir.join(fixture_dir);

    sans_codegen::compile_to_object(&merged, obj_path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("codegen error: {}", e));

    // Compile runtime files (C and Sans)
    let mut runtime_objs: Vec<std::path::PathBuf> = C_RUNTIME_NAMES
        .iter()
        .map(|name| compile_runtime(manifest_dir, &tmp_dir, name, fixture_dir))
        .collect();
    runtime_objs.extend(SANS_RUNTIME_NAMES
        .iter()
        .map(|name| compile_sans_runtime(manifest_dir, &tmp_dir, name, fixture_dir)));

    // Link
    let mut link_args: Vec<String> = vec![obj_path.to_str().unwrap().to_string()];
    link_args.extend(runtime_objs.iter().map(|p| p.to_str().unwrap().to_string()));
    link_args.push("-lcurl".to_string());
    link_args.push("-o".to_string());
    link_args.push(bin_path.to_str().unwrap().to_string());
    let link_status = Command::new("cc")
        .args(&link_args)
        .status()
        .expect("failed to invoke linker");
    assert!(link_status.success(), "linker failed");

    let run_status = Command::new(bin_path.to_str().unwrap())
        .status()
        .expect("failed to run compiled binary");

    // Clean up
    let _ = std::fs::remove_file(&obj_path);
    let _ = std::fs::remove_file(&bin_path);
    for p in &runtime_objs {
        let _ = std::fs::remove_file(p);
    }

    run_status.code().unwrap_or(-1)
}

/// Helper: compile a .sans fixture file and run it, returning the exit code.
fn compile_and_run(fixture: &str) -> i32 {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_path = format!("{}/../../tests/fixtures/{}", manifest_dir, fixture);
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("could not read fixture '{}': {}", fixture_path, e));

    // Parse
    let program = sans_parser::parse(&source)
        .unwrap_or_else(|e| panic!("parse error: {:?}", e));

    // Type check
    sans_typeck::check(&program, &std::collections::HashMap::new())
        .unwrap_or_else(|e| panic!("type error: {}", e.message));

    // Lower to IR
    let ir_module = sans_ir::lower(&program, None, &std::collections::HashMap::new());

    // Codegen to object file
    let tmp_dir = std::env::temp_dir();
    let obj_path = tmp_dir.join(format!("{}.o", fixture));
    let bin_path = tmp_dir.join(fixture.replace(".sans", ""));

    sans_codegen::compile_to_object(&ir_module, obj_path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("codegen error: {}", e));

    // Compile runtime files (C and Sans)
    let mut runtime_objs: Vec<std::path::PathBuf> = C_RUNTIME_NAMES
        .iter()
        .map(|name| compile_runtime(manifest_dir, &tmp_dir, name, fixture))
        .collect();
    runtime_objs.extend(SANS_RUNTIME_NAMES
        .iter()
        .map(|name| compile_sans_runtime(manifest_dir, &tmp_dir, name, fixture)));

    // Link
    let mut link_args: Vec<String> = vec![obj_path.to_str().unwrap().to_string()];
    link_args.extend(runtime_objs.iter().map(|p| p.to_str().unwrap().to_string()));
    link_args.push("-lcurl".to_string());
    link_args.push("-o".to_string());
    link_args.push(bin_path.to_str().unwrap().to_string());
    let link_status = Command::new("cc")
        .args(&link_args)
        .status()
        .expect("failed to invoke linker");
    assert!(link_status.success(), "linker failed");

    // Run and get exit code
    let run_status = Command::new(bin_path.to_str().unwrap())
        .status()
        .expect("failed to run compiled binary");

    // Clean up
    let _ = std::fs::remove_file(&obj_path);
    let _ = std::fs::remove_file(&bin_path);
    for p in &runtime_objs {
        let _ = std::fs::remove_file(p);
    }

    run_status.code().unwrap_or(-1)
}

#[test]
fn e2e_struct_basic() {
    assert_eq!(compile_and_run("struct_basic.sans"), 7);
}

#[test]
fn e2e_struct_nested_access() {
    assert_eq!(compile_and_run("struct_nested_access.sans"), 30);
}

#[test]
fn e2e_struct_return_repeated() {
    assert_eq!(compile_and_run("struct_return_repeated.sans"), 3);
}

#[test]
fn e2e_enum_match_method() {
    assert_eq!(compile_and_run("enum_match_method.sans"), 5);
}

#[test]
fn e2e_enum_basic() {
    assert_eq!(compile_and_run("enum_basic.sans"), 2);
}

#[test]
fn e2e_enum_data() {
    assert_eq!(compile_and_run("enum_data.sans"), 12);
}

#[test]
fn e2e_method_basic() {
    assert_eq!(compile_and_run("method_basic.sans"), 7);
}

#[test]
fn e2e_trait_impl() {
    assert_eq!(compile_and_run("trait_impl.sans"), 13);
}

#[test]
fn e2e_generic_identity() {
    assert_eq!(compile_and_run("generic_identity.sans"), 42);
}

#[test]
fn e2e_generic_pair() {
    assert_eq!(compile_and_run("generic_pair.sans"), 17);
}

#[test]
fn e2e_spawn_join() {
    assert_eq!(compile_and_run("spawn_join.sans"), 7);
}

#[test]
fn e2e_channel_basic() {
    assert_eq!(compile_and_run("channel_basic.sans"), 42);
}

#[test]
fn e2e_spawn_channel() {
    assert_eq!(compile_and_run("spawn_channel.sans"), 10);
}

#[test]
fn e2e_mutex_basic() {
    assert_eq!(compile_and_run("mutex_basic.sans"), 15);
}

#[test]
fn e2e_mutex_threaded() {
    assert_eq!(compile_and_run("mutex_threaded.sans"), 1);
}

#[test]
fn e2e_channel_bounded() {
    assert_eq!(compile_and_run("channel_bounded.sans"), 30);
}

#[test]
fn e2e_array_basic() {
    assert_eq!(compile_and_run("array_basic.sans"), 28);
}

#[test]
fn e2e_array_literal() {
    assert_eq!(compile_and_run("array_literal.sans"), 63);
}

#[test]
fn e2e_array_param() {
    assert_eq!(compile_and_run("array_param.sans"), 70);
}

#[test]
fn e2e_array_for_in() {
    assert_eq!(compile_and_run("array_for_in.sans"), 10);
}

#[test]
fn e2e_string_ops() {
    assert_eq!(compile_and_run("string_ops.sans"), 18);
}

#[test]
fn e2e_string_conversion() {
    assert_eq!(compile_and_run("string_conversion.sans"), 42);
}

#[test]
fn e2e_import_basic() {
    assert_eq!(compile_and_run_dir("import_basic"), 7);
}

#[test]
fn e2e_import_nested() {
    assert_eq!(compile_and_run_dir("import_nested"), 15);
}

#[test]
fn e2e_import_chain() {
    assert_eq!(compile_and_run_dir("import_chain"), 13);
}

#[test]
fn e2e_import_struct() {
    assert_eq!(compile_and_run_dir("import_struct"), 22);
}

#[test]
fn e2e_file_write_read() {
    assert_eq!(compile_and_run("file_write_read.sans"), 11);
}

#[test]
fn e2e_file_exists_check() {
    assert_eq!(compile_and_run("file_exists_check.sans"), 1);
}

#[test]
fn e2e_read_file_alias() {
    assert_eq!(compile_and_run("read_file_alias.sans"), 9);
}

#[test]
fn e2e_args_builtin() {
    assert_eq!(compile_and_run("args_builtin.sans"), 1);
}

#[test]
fn e2e_json_object_stringify() {
    assert_eq!(compile_and_run("json_object_stringify.sans"), 2);
}

#[test]
fn e2e_json_int_roundtrip() {
    assert_eq!(compile_and_run("json_int_roundtrip.sans"), 42);
}

#[test]
fn e2e_json_build() {
    assert_eq!(compile_and_run("json_build.sans"), 50);
}

#[test]
fn e2e_json_parse_access() {
    assert_eq!(compile_and_run("json_parse_access.sans"), 42);
}

#[test]
fn e2e_json_roundtrip() {
    assert_eq!(compile_and_run("json_roundtrip.sans"), 7);
}

#[test]
fn e2e_http_error_handling() {
    assert_eq!(compile_and_run("http_error_handling.sans"), 1);
}

#[test]
fn e2e_log_levels() {
    assert_eq!(compile_and_run("log_levels.sans"), 0);
}

#[test]
fn e2e_map_basic() {
    assert_eq!(compile_and_run("map_basic.sans"), 30);
}

#[test]
fn e2e_map_has() {
    assert_eq!(compile_and_run("map_has.sans"), 42);
}

#[test]
fn e2e_map_len() {
    assert_eq!(compile_and_run("map_len.sans"), 3);
}

#[test]
fn e2e_demo_backend() {
    // Clean up any leftover output file
    let _ = std::fs::remove_file("demo_output.txt");
    let result = compile_and_run_dir("demo_backend");
    // Clean up output file created by the demo
    let _ = std::fs::remove_file("demo_output.txt");
    assert_eq!(result, 28);
}

#[test]
fn e2e_result_ok_unwrap() {
    // divide(10,2)=5 + divide(20,4)=5 = 10
    assert_eq!(compile_and_run("result_ok_unwrap.sans"), 10);
}

#[test]
fn e2e_result_error_handling() {
    // divide(10,0) -> err, unwrap_or(99) = 99
    assert_eq!(compile_and_run("result_error_handling.sans"), 99);
}

#[test]
fn e2e_float_basic() {
    // float_to_int(3.14 * 2.0 * 2.0) = float_to_int(12.56) = 12
    assert_eq!(compile_and_run("float_basic.sans"), 12);
}

#[test]
fn e2e_string_methods() {
    assert_eq!(compile_and_run("string_methods.sans"), 17);
}

#[test]
fn e2e_string_ends_with() {
    assert_eq!(compile_and_run("string_ends_with.sans"), 2);
}

#[test]
fn e2e_array_methods() {
    assert_eq!(compile_and_run("array_methods.sans"), 33);
}

#[test]
fn e2e_map_filter() {
    assert_eq!(compile_and_run("map_filter.sans"), 21);
}

#[test]
fn e2e_string_replace() {
    assert_eq!(compile_and_run("string_replace.sans"), 11);
}

#[test]
fn e2e_array_remove() {
    assert_eq!(compile_and_run("array_remove.sans"), 63);
}

#[test]
fn e2e_multiline_string() {
    assert_eq!(compile_and_run("multiline_string.sans"), 11);
}

#[test]
fn e2e_modulo_neg() {
    assert_eq!(compile_and_run("modulo_neg.sans"), 9);
}

#[test]
fn e2e_string_interp() {
    assert_eq!(compile_and_run("string_interp.sans"), 11);
}

#[test]
fn e2e_ai_syntax() {
    assert_eq!(compile_and_run("ai_syntax.sans"), 126);
}

#[test]
fn e2e_ai_syntax2() {
    assert_eq!(compile_and_run("ai_syntax2.sans"), 17);
}

#[test]
fn e2e_ai_syntax3() {
    // first=2, last=10, total=15 => 2+10+15=27
    assert_eq!(compile_and_run("ai_syntax3.sans"), 27);
}

#[test]
fn e2e_ai_syntax4() {
    // divide(10,2)=5
    assert_eq!(compile_and_run("ai_syntax4.sans"), 5);
}

#[test]
fn e2e_ai_syntax5() {
    // a.len without parens = 3
    assert_eq!(compile_and_run("ai_syntax5.sans"), 3);
}

#[test]
fn e2e_global_var() {
    // g counter = 0; inc 3 times; counter = 3
    assert_eq!(compile_and_run("global_var.sans"), 3);
}

#[test]
fn e2e_tuple_basic() {
    assert_eq!(compile_and_run("tuple_basic.sans"), 5);
}

#[test]
fn e2e_tuple_return() {
    assert_eq!(compile_and_run("tuple_return.sans"), 30);
}

#[test]
fn e2e_tuple_three() {
    assert_eq!(compile_and_run("tuple_three.sans"), 42);
}

#[test]
fn e2e_tuple_nested() {
    assert_eq!(compile_and_run("tuple_nested.sans"), 3);
}

#[test]
fn e2e_lambda_basic() {
    assert_eq!(compile_and_run("lambda_basic.sans"), 15);
}

#[test]
fn e2e_lambda_map() {
    assert_eq!(compile_and_run("lambda_map.sans"), 9);
}

#[test]
fn e2e_lambda_capture() {
    assert_eq!(compile_and_run("lambda_capture.sans"), 15);
}

#[test]
fn e2e_nested_lambda() {
    assert_eq!(compile_and_run("nested_lambda.sans"), 15);
}

#[test]
fn e2e_array_any() {
    assert_eq!(compile_and_run("array_any.sans"), 1);
}

#[test]
fn e2e_array_find() {
    assert_eq!(compile_and_run("array_find.sans"), 30);
}

#[test]
fn e2e_array_enumerate() {
    assert_eq!(compile_and_run("array_enumerate.sans"), 32);
}

#[test]
fn e2e_array_zip() {
    assert_eq!(compile_and_run("array_zip.sans"), 22);
}

#[test]
fn e2e_string_slice() {
    assert_eq!(compile_and_run("string_slice.sans"), 10);
}

#[test]
fn e2e_string_interp_expr() {
    assert_eq!(compile_and_run("string_interp_expr.sans"), 6);
}

#[test]
fn e2e_try_operator() {
    assert_eq!(compile_and_run("try_operator.sans"), 6);
}

#[test]
fn e2e_try_operator_err() {
    assert_eq!(compile_and_run("try_operator_err.sans"), 99);
}

#[test]
fn e2e_break_basic() {
    assert_eq!(compile_and_run("break_basic.sans"), 10);
}

#[test]
fn e2e_continue_basic() {
    assert_eq!(compile_and_run("continue_basic.sans"), 25);
}

#[test]
fn e2e_tuple_return_typed() {
    assert_eq!(compile_and_run("tuple_return_typed.sans"), 7);
}

#[test]
fn e2e_tuple_array() {
    assert_eq!(compile_and_run("tuple_array.sans"), 3);
}

#[test]
fn e2e_arena_basic() {
    assert_eq!(compile_and_run("arena_basic.sans"), 100);
}

#[test]
fn e2e_arena_nested() {
    assert_eq!(compile_and_run("arena_nested.sans"), 141);
}
