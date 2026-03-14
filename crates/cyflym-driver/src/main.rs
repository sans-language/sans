use std::path::PathBuf;
use std::process;

use cyflym::imports;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 || args[1] != "build" {
        eprintln!("Usage: cyflym build <file.cy>");
        process::exit(1);
    }

    let source_path = PathBuf::from(&args[2]);

    if let Err(e) = build(&source_path) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}

fn build(source_path: &PathBuf) -> Result<(), String> {
    // Validate extension
    if source_path.extension().and_then(|e| e.to_str()) != Some("cy") {
        return Err(format!(
            "expected a .cy source file, got: {}",
            source_path.display()
        ));
    }

    // Step 1: Resolve imports (recursive, topological order)
    let resolved_modules = imports::resolve_imports(source_path)?;

    // Step 2: Read and parse the entry point
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| format!("could not read '{}': {}", source_path.display(), e))?;
    let main_program = cyflym_parser::parse(&source).map_err(|e| {
        format!(
            "parse error at {}..{}: {}",
            e.span.start, e.span.end, e.message
        )
    })?;

    // Step 3: Type-check in dependency order, collecting module exports
    let mut module_exports: std::collections::HashMap<String, cyflym_typeck::ModuleExports> =
        std::collections::HashMap::new();

    for module in &resolved_modules {
        let exports = cyflym_typeck::check_module(&module.program, &module_exports)
            .map_err(|e| format!("type error in module '{}': {}", module.name, e.message))?;
        module_exports.insert(module.name.clone(), exports);
    }

    // Type-check main module
    cyflym_typeck::check(&main_program, &module_exports)
        .map_err(|e| format!("type error: {}", e.message))?;

    // Step 4: Build module_fn_ret_types for IR lowering
    let mut module_fn_ret_types: std::collections::HashMap<(String, String), cyflym_ir::IrType> =
        std::collections::HashMap::new();
    for (mod_name, exports) in &module_exports {
        for (func_name, sig) in &exports.functions {
            let ir_type = cyflym_ir::ir_type_for_return(&sig.return_type);
            module_fn_ret_types.insert((mod_name.clone(), func_name.clone()), ir_type);
        }
    }

    // Step 5: Build extra struct defs from all module exports (for cross-module field access)
    let mut extra_struct_defs: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for exports in module_exports.values() {
        for (struct_name, fields) in &exports.structs {
            let field_names: Vec<String> = fields.iter().map(|(name, _)| name.clone()).collect();
            extra_struct_defs.insert(struct_name.clone(), field_names);
        }
    }

    // Step 6: Lower to IR with name mangling, then merge
    let mut all_ir_functions = Vec::new();

    for module in &resolved_modules {
        let ir = cyflym_ir::lower(&module.program, Some(&module.name), &module_fn_ret_types);
        all_ir_functions.extend(ir.functions);
    }

    let main_ir = cyflym_ir::lower_with_extra_structs(&main_program, None, &module_fn_ret_types, &extra_struct_defs);
    all_ir_functions.extend(main_ir.functions);

    let merged_module = cyflym_ir::ir::Module {
        functions: all_ir_functions,
    };

    // Step 7: Codegen
    let obj_path = source_path.with_extension("o");
    let obj_path_str = obj_path
        .to_str()
        .ok_or_else(|| "object path contains invalid UTF-8".to_string())?;

    cyflym_codegen::compile_to_object(&merged_module, obj_path_str)
        .map_err(|e| format!("codegen error: {}", e))?;

    // Step 8: Link
    let output_path = source_path.with_extension("");
    let output_path_str = output_path
        .to_str()
        .ok_or_else(|| "output path contains invalid UTF-8".to_string())?;

    // Compile JSON runtime
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let json_c_path = format!("{}/../../runtime/json.c", manifest_dir);
    let tmp_dir = std::env::temp_dir();
    let json_o_path = tmp_dir.join("cyflym_json_runtime.o");
    let json_compile = process::Command::new("cc")
        .args(["-c", &json_c_path, "-o", json_o_path.to_str().unwrap()])
        .status()
        .map_err(|e| format!("failed to compile json runtime: {}", e))?;
    if !json_compile.success() {
        return Err("failed to compile json runtime".to_string());
    }

    let link_status = process::Command::new("cc")
        .args([obj_path_str, json_o_path.to_str().unwrap(), "-o", output_path_str])
        .status()
        .map_err(|e| format!("failed to invoke linker: {}", e))?;

    if !link_status.success() {
        return Err(format!(
            "linker exited with status {}",
            link_status.code().unwrap_or(-1)
        ));
    }

    // Step 9: Clean up .o file
    std::fs::remove_file(&obj_path)
        .map_err(|e| format!("could not remove object file '{}': {}", obj_path.display(), e))?;

    println!("Built: {}", output_path.display());

    Ok(())
}
