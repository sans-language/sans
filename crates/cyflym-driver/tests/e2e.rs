use std::process::Command;

/// Helper: compile a .cy fixture file and run it, returning the exit code.
fn compile_and_run(fixture: &str) -> i32 {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_path = format!("{}/../../tests/fixtures/{}", manifest_dir, fixture);
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("could not read fixture '{}': {}", fixture_path, e));

    // Parse
    let program = cyflym_parser::parse(&source)
        .unwrap_or_else(|e| panic!("parse error: {:?}", e));

    // Type check
    cyflym_typeck::check(&program)
        .unwrap_or_else(|e| panic!("type error: {}", e));

    // Lower to IR
    let ir_module = cyflym_ir::lower(&program);

    // Codegen to object file
    let tmp_dir = std::env::temp_dir();
    let obj_path = tmp_dir.join(format!("{}.o", fixture));
    let bin_path = tmp_dir.join(fixture.replace(".cy", ""));

    cyflym_codegen::compile_to_object(&ir_module, obj_path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("codegen error: {}", e));

    // Link
    let link_status = Command::new("cc")
        .args([obj_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap()])
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
