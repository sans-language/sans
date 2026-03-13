use std::collections::HashSet;
use std::path::{Path, PathBuf};

use cyflym_parser::ast::Program;

/// A parsed module with its metadata.
pub struct ResolvedModule {
    pub name: String,      // module prefix, e.g., "user"
    pub path: PathBuf,     // absolute path to .cy file
    pub program: Program,  // parsed AST
}

/// Recursively resolves all imports starting from the entry point.
/// Returns modules in topological order (dependencies first, entry point last).
/// The entry point itself is NOT included in the returned vec — only imported modules.
pub fn resolve_imports(entry_point: &Path) -> Result<Vec<ResolvedModule>, String> {
    let base_dir = entry_point.parent()
        .ok_or_else(|| format!("cannot determine directory of '{}'", entry_point.display()))?;

    let mut resolved: Vec<ResolvedModule> = Vec::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut in_progress: HashSet<PathBuf> = HashSet::new();

    let entry_source = std::fs::read_to_string(entry_point)
        .map_err(|e| format!("could not read '{}': {}", entry_point.display(), e))?;
    let entry_program = cyflym_parser::parse(&entry_source)
        .map_err(|e| format!("parse error in '{}' at {}..{}: {}", entry_point.display(), e.span.start, e.span.end, e.message))?;

    let entry_canonical = entry_point.canonicalize()
        .map_err(|e| format!("could not canonicalize '{}': {}", entry_point.display(), e))?;
    in_progress.insert(entry_canonical.clone());

    for import in &entry_program.imports {
        resolve_import(
            &import.path,
            &import.module_name,
            base_dir,
            &mut resolved,
            &mut visited,
            &mut in_progress,
        )?;
    }

    Ok(resolved)
}

fn resolve_import(
    import_path: &str,
    module_name: &str,
    base_dir: &Path,
    resolved: &mut Vec<ResolvedModule>,
    visited: &mut HashSet<PathBuf>,
    in_progress: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    // All paths resolve relative to entry point's directory (base_dir), per spec.
    let file_path = base_dir.join(format!("{}.cy", import_path));
    let canonical = file_path.canonicalize()
        .map_err(|_| format!("module not found: {}", import_path))?;

    if visited.contains(&canonical) {
        return Ok(());
    }

    if in_progress.contains(&canonical) {
        return Err(format!("circular import detected: {}", import_path));
    }

    in_progress.insert(canonical.clone());

    let source = std::fs::read_to_string(&file_path)
        .map_err(|_| format!("module not found: {}", import_path))?;
    let program = cyflym_parser::parse(&source)
        .map_err(|e| format!(
            "parse error in '{}' at {}..{}: {}",
            file_path.display(), e.span.start, e.span.end, e.message
        ))?;

    for sub_import in &program.imports {
        resolve_import(
            &sub_import.path,
            &sub_import.module_name,
            base_dir,
            resolved,
            visited,
            in_progress,
        )?;
    }

    in_progress.remove(&canonical);
    visited.insert(canonical.clone());
    resolved.push(ResolvedModule {
        name: module_name.to_string(),
        path: canonical,
        program,
    });

    Ok(())
}
