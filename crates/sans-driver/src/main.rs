use std::path::PathBuf;
use std::process;

use sans::imports;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("sans {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if args.len() < 3 {
        eprintln!("Usage: sans <build|run> <file.sans>");
        eprintln!("       sans --version");
        process::exit(1);
    }

    let command = &args[1];
    let source_path = PathBuf::from(&args[2]);

    match command.as_str() {
        "build" => {
            if let Err(e) = build(&source_path) {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        }
        "run" => {
            match run(&source_path) {
                Ok(exit_code) => process::exit(exit_code),
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            }
        }
        other => {
            eprintln!("unknown command '{}'. Usage: sans <build|run> <file.sans>", other);
            process::exit(1);
        }
    }
}

fn run(source_path: &PathBuf) -> Result<i32, String> {
    // Build the binary
    build(source_path)?;

    // Run the binary (canonicalize to absolute path so it's found without ./)
    let output_path = source_path.with_extension("");
    let output_path = std::fs::canonicalize(&output_path)
        .unwrap_or(output_path);
    let output_path_str = output_path
        .to_str()
        .ok_or_else(|| "output path contains invalid UTF-8".to_string())?;

    let run_status = process::Command::new(output_path_str)
        .status()
        .map_err(|e| format!("failed to run '{}': {}", output_path.display(), e))?;

    // Clean up the binary
    let _ = std::fs::remove_file(&output_path);

    Ok(run_status.code().unwrap_or(-1))
}

fn build(source_path: &PathBuf) -> Result<(), String> {
    // Validate extension
    if source_path.extension().and_then(|e| e.to_str()) != Some("sans") {
        return Err(format!(
            "expected a .sans source file, got: {}",
            source_path.display()
        ));
    }

    // Step 1: Resolve imports (recursive, topological order)
    let resolved_modules = imports::resolve_imports(source_path)?;

    // Step 2: Read and parse the entry point
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| format!("could not read '{}': {}", source_path.display(), e))?;
    let main_program = sans_parser::parse(&source).map_err(|e| {
        format!(
            "parse error at {}..{}: {}",
            e.span.start, e.span.end, e.message
        )
    })?;

    // Step 3: Type-check in dependency order, collecting module exports
    let mut module_exports: std::collections::HashMap<String, sans_typeck::ModuleExports> =
        std::collections::HashMap::new();

    for module in &resolved_modules {
        let exports = sans_typeck::check_module(&module.program, &module_exports)
            .map_err(|e| format!("type error in module '{}': {}", module.name, e.message))?;
        module_exports.insert(module.name.clone(), exports);
    }

    // Type-check main module
    sans_typeck::check(&main_program, &module_exports)
        .map_err(|e| format!("type error: {}", e.message))?;

    // Step 4: Build module_fn_ret_types for IR lowering
    let mut module_fn_ret_types: std::collections::HashMap<(String, String), sans_ir::IrType> =
        std::collections::HashMap::new();
    for (mod_name, exports) in &module_exports {
        for (func_name, sig) in &exports.functions {
            let ir_type = sans_ir::ir_type_for_return(&sig.return_type);
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
        let ir = sans_ir::lower(&module.program, Some(&module.name), &module_fn_ret_types);
        all_ir_functions.extend(ir.functions);
    }

    let main_ir = sans_ir::lower_with_extra_structs(&main_program, None, &module_fn_ret_types, &extra_struct_defs);
    let all_globals = main_ir.globals;
    all_ir_functions.extend(main_ir.functions);

    let merged_module = sans_ir::ir::Module {
        globals: all_globals,
        functions: all_ir_functions,
    };

    // Step 7: Codegen
    let obj_path = source_path.with_extension("o");
    let obj_path_str = obj_path
        .to_str()
        .ok_or_else(|| "object path contains invalid UTF-8".to_string())?;

    sans_codegen::compile_to_object(&merged_module, obj_path_str)
        .map_err(|e| format!("codegen error: {}", e))?;

    // Step 8: Link
    let output_path = source_path.with_extension("");
    let output_path_str = output_path
        .to_str()
        .ok_or_else(|| "output path contains invalid UTF-8".to_string())?;

    // Compile runtime modules (C and Sans)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let tmp_dir = std::env::temp_dir();
    let c_runtime_modules = [
        "server", "sock", "curl_helpers",
    ];
    let sans_runtime_modules = [
        "log", "result", "functional", "array_ext", "string_ext", "http", "json",
    ];
    let mut runtime_o_paths: Vec<PathBuf> = Vec::new();
    for name in &c_runtime_modules {
        let o_path = compile_runtime(manifest_dir, &tmp_dir, name)?;
        runtime_o_paths.push(o_path);
    }
    for name in &sans_runtime_modules {
        let o_path = compile_sans_runtime(manifest_dir, &tmp_dir, name)?;
        runtime_o_paths.push(o_path);
    }

    let mut link_args: Vec<String> = vec![obj_path_str.to_string()];
    for o_path in &runtime_o_paths {
        link_args.push(o_path.to_str().unwrap().to_string());
    }
    link_args.extend(["-lcurl".to_string(), "-o".to_string(), output_path_str.to_string()]);

    let link_status = process::Command::new("cc")
        .args(&link_args)
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

fn compile_runtime(
    manifest_dir: &str,
    tmp_dir: &std::path::Path,
    name: &str,
) -> Result<PathBuf, String> {
    let c_path = format!("{}/../../runtime/{}.c", manifest_dir, name);
    let o_path = tmp_dir.join(format!("sans_{}_runtime.o", name));
    let status = process::Command::new("cc")
        .args(["-c", &c_path, "-o", o_path.to_str().unwrap()])
        .status()
        .map_err(|e| format!("failed to compile {} runtime: {}", name, e))?;
    if !status.success() {
        return Err(format!("failed to compile {} runtime", name));
    }
    Ok(o_path)
}

fn compile_sans_runtime(
    manifest_dir: &str,
    tmp_dir: &std::path::Path,
    name: &str,
) -> Result<PathBuf, String> {
    let sans_path = format!("{}/../../runtime/{}.sans", manifest_dir, name);
    let o_path = tmp_dir.join(format!("sans_{}_runtime.o", name));

    // Read and parse the .sans runtime file
    let source = std::fs::read_to_string(&sans_path)
        .map_err(|e| format!("could not read runtime '{}': {}", sans_path, e))?;
    let program = sans_parser::parse(&source)
        .map_err(|e| format!("parse error in runtime {}: {}", name, e.message))?;

    // Type-check as a module (no main required)
    let module_exports = std::collections::HashMap::new();
    sans_typeck::check_module(&program, &module_exports)
        .map_err(|e| format!("type error in runtime {}: {}", name, e.message))?;

    // Lower to IR (no module prefix — we need raw cy_* symbol names)
    let module_fn_ret_types = std::collections::HashMap::new();
    let ir = sans_ir::lower(&program, None, &module_fn_ret_types);

    // Compile to object file
    let o_path_str = o_path.to_str()
        .ok_or_else(|| "runtime object path contains invalid UTF-8".to_string())?;
    sans_codegen::compile_to_object(&ir, o_path_str)
        .map_err(|e| format!("codegen error in runtime {}: {}", name, e))?;

    Ok(o_path)
}
