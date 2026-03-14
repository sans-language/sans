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

const RUNTIME_NAMES: &[&str] = &["json", "http", "log", "result", "string_ext", "array_ext", "functional", "server"];

/// Helper: compile a multi-file fixture directory and run main.cy, returning the exit code.
fn compile_and_run_dir(fixture_dir: &str) -> i32 {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dir_path = format!("{}/../../tests/fixtures/{}", manifest_dir, fixture_dir);
    let main_path = std::path::PathBuf::from(format!("{}/main.cy", dir_path));

    // Resolve imports
    let resolved_modules = sans::imports::resolve_imports(&main_path)
        .unwrap_or_else(|e| panic!("import resolution error: {}", e));

    // Parse main
    let main_source = std::fs::read_to_string(&main_path)
        .unwrap_or_else(|e| panic!("could not read main.cy: {}", e));
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
    all_ir_functions.extend(main_ir.functions);

    let merged = sans_ir::ir::Module { functions: all_ir_functions };

    // Codegen, link, run
    let tmp_dir = std::env::temp_dir();
    let obj_path = tmp_dir.join(format!("{}.o", fixture_dir));
    let bin_path = tmp_dir.join(fixture_dir);

    sans_codegen::compile_to_object(&merged, obj_path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("codegen error: {}", e));

    // Compile all C runtime files
    let runtime_objs: Vec<std::path::PathBuf> = RUNTIME_NAMES
        .iter()
        .map(|name| compile_runtime(manifest_dir, &tmp_dir, name, fixture_dir))
        .collect();

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

/// Helper: compile a .cy fixture file and run it, returning the exit code.
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
    let bin_path = tmp_dir.join(fixture.replace(".cy", ""));

    sans_codegen::compile_to_object(&ir_module, obj_path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("codegen error: {}", e));

    // Compile all C runtime files
    let runtime_objs: Vec<std::path::PathBuf> = RUNTIME_NAMES
        .iter()
        .map(|name| compile_runtime(manifest_dir, &tmp_dir, name, fixture))
        .collect();

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
    assert_eq!(compile_and_run("struct_basic.cy"), 7);
}

#[test]
fn e2e_struct_nested_access() {
    assert_eq!(compile_and_run("struct_nested_access.cy"), 30);
}

#[test]
fn e2e_enum_basic() {
    assert_eq!(compile_and_run("enum_basic.cy"), 2);
}

#[test]
fn e2e_enum_data() {
    assert_eq!(compile_and_run("enum_data.cy"), 12);
}

#[test]
fn e2e_method_basic() {
    assert_eq!(compile_and_run("method_basic.cy"), 7);
}

#[test]
fn e2e_trait_impl() {
    assert_eq!(compile_and_run("trait_impl.cy"), 13);
}

#[test]
fn e2e_generic_identity() {
    assert_eq!(compile_and_run("generic_identity.cy"), 42);
}

#[test]
fn e2e_generic_pair() {
    assert_eq!(compile_and_run("generic_pair.cy"), 17);
}

#[test]
fn e2e_spawn_join() {
    assert_eq!(compile_and_run("spawn_join.cy"), 7);
}

#[test]
fn e2e_channel_basic() {
    assert_eq!(compile_and_run("channel_basic.cy"), 42);
}

#[test]
fn e2e_spawn_channel() {
    assert_eq!(compile_and_run("spawn_channel.cy"), 10);
}

#[test]
fn e2e_mutex_basic() {
    assert_eq!(compile_and_run("mutex_basic.cy"), 15);
}

#[test]
fn e2e_mutex_threaded() {
    assert_eq!(compile_and_run("mutex_threaded.cy"), 1);
}

#[test]
fn e2e_channel_bounded() {
    assert_eq!(compile_and_run("channel_bounded.cy"), 30);
}

#[test]
fn e2e_array_basic() {
    assert_eq!(compile_and_run("array_basic.cy"), 28);
}

#[test]
fn e2e_array_literal() {
    assert_eq!(compile_and_run("array_literal.cy"), 63);
}

#[test]
fn e2e_array_for_in() {
    assert_eq!(compile_and_run("array_for_in.cy"), 10);
}

#[test]
fn e2e_string_ops() {
    assert_eq!(compile_and_run("string_ops.cy"), 18);
}

#[test]
fn e2e_string_conversion() {
    assert_eq!(compile_and_run("string_conversion.cy"), 42);
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
    assert_eq!(compile_and_run("file_write_read.cy"), 11);
}

#[test]
fn e2e_file_exists_check() {
    assert_eq!(compile_and_run("file_exists_check.cy"), 1);
}

#[test]
fn e2e_json_object_stringify() {
    assert_eq!(compile_and_run("json_object_stringify.cy"), 2);
}

#[test]
fn e2e_json_int_roundtrip() {
    assert_eq!(compile_and_run("json_int_roundtrip.cy"), 42);
}

#[test]
fn e2e_json_build() {
    assert_eq!(compile_and_run("json_build.cy"), 52);
}

#[test]
fn e2e_json_parse_access() {
    assert_eq!(compile_and_run("json_parse_access.cy"), 42);
}

#[test]
fn e2e_json_roundtrip() {
    assert_eq!(compile_and_run("json_roundtrip.cy"), 7);
}

#[test]
fn e2e_http_error_handling() {
    assert_eq!(compile_and_run("http_error_handling.cy"), 1);
}

#[test]
fn e2e_log_levels() {
    assert_eq!(compile_and_run("log_levels.cy"), 0);
}

#[test]
fn e2e_demo_backend() {
    // Clean up any leftover output file
    let _ = std::fs::remove_file("demo_output.txt");
    let result = compile_and_run_dir("demo_backend");
    // Clean up output file created by the demo
    let _ = std::fs::remove_file("demo_output.txt");
    assert_eq!(result, 30);
}

#[test]
fn e2e_result_ok_unwrap() {
    // divide(10,2)=5 + divide(20,4)=5 = 10
    assert_eq!(compile_and_run("result_ok_unwrap.cy"), 10);
}

#[test]
fn e2e_result_error_handling() {
    // divide(10,0) -> err, unwrap_or(99) = 99
    assert_eq!(compile_and_run("result_error_handling.cy"), 99);
}

#[test]
fn e2e_float_basic() {
    // float_to_int(3.14 * 2.0 * 2.0) = float_to_int(12.56) = 12
    assert_eq!(compile_and_run("float_basic.cy"), 12);
}

#[test]
fn e2e_string_methods() {
    assert_eq!(compile_and_run("string_methods.cy"), 17);
}

#[test]
fn e2e_array_methods() {
    assert_eq!(compile_and_run("array_methods.cy"), 33);
}

#[test]
fn e2e_map_filter() {
    assert_eq!(compile_and_run("map_filter.cy"), 21);
}

#[test]
fn e2e_string_replace() {
    assert_eq!(compile_and_run("string_replace.cy"), 11);
}

#[test]
fn e2e_array_remove() {
    assert_eq!(compile_and_run("array_remove.cy"), 63);
}

#[test]
fn e2e_multiline_string() {
    assert_eq!(compile_and_run("multiline_string.cy"), 11);
}

#[test]
fn e2e_modulo_neg() {
    assert_eq!(compile_and_run("modulo_neg.cy"), 9);
}

#[test]
fn e2e_string_interp() {
    assert_eq!(compile_and_run("string_interp.cy"), 13);
}

#[test]
fn e2e_ai_syntax() {
    assert_eq!(compile_and_run("ai_syntax.cy"), 126);
}

#[test]
fn e2e_ai_syntax2() {
    assert_eq!(compile_and_run("ai_syntax2.cy"), 17);
}
