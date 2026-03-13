use std::path::PathBuf;
use std::process;

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

    // Step 1: Read source file
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| format!("could not read '{}': {}", source_path.display(), e))?;

    // Step 2: Parse
    let program = cyflym_parser::parse(&source).map_err(|e| {
        format!(
            "parse error at {}..{}: {}",
            e.span.start, e.span.end, e.message
        )
    })?;

    // Step 3: Type check
    cyflym_typeck::check(&program, &std::collections::HashMap::new()).map_err(|e| format!("type error: {}", e.message))?;

    // Step 4: Lower to IR
    let ir_module = cyflym_ir::lower(&program);

    // Step 5: Codegen to object file — replace .cy with .o
    let obj_path = source_path.with_extension("o");
    let obj_path_str = obj_path
        .to_str()
        .ok_or_else(|| "object path contains invalid UTF-8".to_string())?;

    cyflym_codegen::compile_to_object(&ir_module, obj_path_str)
        .map_err(|e| format!("codegen error: {}", e))?;

    // Step 6: Link — output path is source path with .cy removed (no extension)
    let output_path = source_path.with_extension("");
    let output_path_str = output_path
        .to_str()
        .ok_or_else(|| "output path contains invalid UTF-8".to_string())?;

    let link_status = process::Command::new("cc")
        .args([obj_path_str, "-o", output_path_str])
        .status()
        .map_err(|e| format!("failed to invoke linker: {}", e))?;

    if !link_status.success() {
        return Err(format!(
            "linker exited with status {}",
            link_status.code().unwrap_or(-1)
        ));
    }

    // Step 7: Clean up .o file
    std::fs::remove_file(&obj_path)
        .map_err(|e| format!("could not remove object file '{}': {}", obj_path.display(), e))?;

    // Step 8: Report success
    println!("Built: {}", output_path.display());

    Ok(())
}
