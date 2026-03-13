# Plan 6b: Module/Import System Design Spec

## Goal

Add minimal multi-file compilation via `import "path"` syntax with module-prefixed access, enabling code organization across files. Functions, structs, and enums are importable. All top-level items are public by default.

## Scope

- `import "path"` declarations at top of file
- Module-prefixed access: `module_name.function()`, `module_name.StructName { }`, `module_name.EnumName::Variant`
- Path resolution relative to entry point file's directory
- Circular import detection (compile error)
- Name mangling in IR for cross-module functions
- Topological sort of import graph for type checking order
- Recursive imports (imported files can import other files)

**Out of scope (deferred):** `pub` keyword / visibility control, selective imports (`import { X } from "mod"`), import aliases (`import "foo" as f`), trait/impl block imports, stdlib module resolution, external dependency resolution / `cyflym.toml`, separate compilation, duplicate last-segment detection.

## Decisions

- **Path resolution:** Relative to the directory containing the file passed to `cyflym build`. `import "utils"` resolves to `utils.cy`, `import "models/user"` resolves to `models/user.cy`.
- **Module prefix:** Last path segment. `import "models/user"` → prefix `user`. Access as `user.function()`.
- **Visibility:** All top-level functions, structs, and enums are public. No `pub` keyword in this plan.
- **Importable items:** Functions, structs, enums. Traits and impl blocks are NOT importable.
- **Circular imports:** Compile error. Detected by tracking visited files during import resolution.
- **Duplicate imports:** Idempotent. Importing the same module twice is not an error.
- **Import placement:** Must appear at top of file before any other declarations. Import after a function/struct/etc is a parse error.
- **Name mangling:** Non-main module functions are mangled as `{module_name}__{function_name}` in IR. Main module functions keep original names.
- **Compilation model:** All modules are parsed, type-checked, lowered to IR, and merged into a single flat IR Module. One object file, one binary. Codegen and linker unchanged.
- **Cross-module call syntax:** `user.create()` parses as `Expr::MethodCall` on `Expr::Identifier("user")`. The type checker recognizes `user` as a module name and resolves to the imported function. No new expression variant needed.
- **Cross-module field access without call:** `utils.greet` (without parens) parses as `Expr::FieldAccess`. The type checker produces an error: `"cannot access field on module 'utils'"`. Only function calls are valid on module references.
- **Name mangling collision:** The `__` separator can technically collide with a user-defined function named e.g. `utils__greet`. This is acceptable for now — identifier names containing `__` are unusual and the language is pre-1.0. A more robust mangling scheme is deferred.
- **Reserved keywords:** `import` becomes a reserved keyword.

## Syntax

```cyflym
// utils.cy
fn greet(name String) String {
    "hello " + name
}

fn add(a Int, b Int) Int {
    a + b
}

// models/user.cy
struct User {
    name String,
    age Int,
}

fn create(name String, age Int) User {
    User { name: name, age: age }
}

// main.cy
import "utils"
import "models/user"

fn main() Int {
    let greeting = utils.greet("world")
    let sum = utils.add(1, 2)
    let u = user.create("Alice", 30)
    sum
}
```

### Keywords

- `import` — new reserved keyword

### Import Declaration

`import "path"` where path is a string literal. Path must not include the `.cy` extension. The last segment of the path becomes the module prefix.

## AST Changes

### New Token

- `Token::Import` — keyword `import`

### New AST Nodes

```rust
pub struct Import {
    pub path: String,        // e.g., "models/user"
    pub module_name: String, // last segment, e.g., "user"
    pub span: Span,
}
```

### Updated Program Struct

```rust
pub struct Program {
    pub imports: Vec<Import>,  // NEW — must be first in file
    pub functions: Vec<Function>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub traits: Vec<TraitDef>,
    pub impls: Vec<ImplBlock>,
}
```

### Cross-Module Expression Handling

Module-prefixed function calls (`utils.greet("world")`) parse as existing `Expr::MethodCall` on `Expr::Identifier("utils")`. The type checker distinguishes module references from variable method calls by checking if the identifier matches an imported module name.

Cross-module struct literals and enum variants require special handling since `user.Point { x: 1 }` would not parse as a struct literal under current rules. **Decision:** Defer direct cross-module struct/enum construction. Imported modules expose functions only for calling. Structs and enums are importable for use as types (return values, parameters) but constructed via module constructor functions (e.g., `user.create()`). This is the idiomatic pattern.

## Type System

### Module Registry

```rust
pub struct ModuleExports {
    pub functions: HashMap<String, FunctionSignature>,
    pub structs: HashMap<String, StructDef>,
    pub enums: HashMap<String, EnumDef>,
}

pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Type,
}
```

### Type Checker Interface Changes

The `check` function signature gains a module registry parameter:

```rust
pub fn check(
    program: &Program,
    module_exports: &HashMap<String, ModuleExports>,
) -> Result<(), TypeError>
```

When type-checking an `Expr::MethodCall` where the receiver is `Expr::Identifier(name)` and `name` matches a key in `module_exports`, the type checker resolves the call against that module's `FunctionSignature` instead of looking for a local method.

When a cross-module function returns a struct type (e.g., `user.create()` returns `User`), the type checker resolves the return type using the module's struct registry. The `Type::Struct { name }` returned refers to the struct definition in the module's exports, not the local scope. The type checker must check the module's `structs` map when resolving struct types that aren't found locally.

For `Expr::FieldAccess` where the receiver is a module name, the type checker produces an error: `"cannot access field on module 'utils' — did you mean to call a function?"`.

### Type Checking Rules

| Expression | Rule |
|---|---|
| `import "path"` | Resolve file, parse, check. Build ModuleExports. |
| `mod.func(args)` | Look up `func` in module `mod`'s exports. Check arg types match signature. Return type is function's return type. |
| Return type `mod.Struct` | Resolve struct definition from module's exports. Type checker checks module struct registry for types not found locally. |
| `for x in mod.func()` | If `mod.func()` returns `Array<T>`, `x` binds as `T`. |

### Type Checking Order

1. Build import graph from all files.
2. Detect cycles → error if found.
3. Topological sort: check leaf modules first.
4. Check each module, building its `ModuleExports`. Pass `module_exports` (populated so far) to `check`.
5. When checking a module, its dependencies' exports are available for cross-module resolution.

### Type Errors

- `import "foo"` but `foo.cy` not found → `"module not found: foo"`
- `import "foo.cy"` (with extension) → `"module not found: foo.cy"` (no special handling — the path simply won't resolve)
- `foo.bar()` but `bar` not in foo's exports → `"function 'bar' not found in module 'foo'"`
- `foo.bar` (field access on module) → `"cannot access field on module 'foo' — did you mean to call a function?"`
- `foo.bar(wrong_args)` → normal argument type mismatch error
- Circular import → `"circular import detected: a → b → a"`
- Import after declaration → parse error `"imports must appear before all declarations"`

## IR Changes

### Name Mangling

Functions in non-main modules are mangled: `{module_name}__{function_name}`.

- `utils.cy`'s `fn greet()` → IR function named `utils__greet`
- `main.cy`'s `fn main()` → IR function named `main` (no mangling)

### Cross-Module Call Lowering

`mod.func(args)` lowers to `Instruction::Call { dest, function: "{mod}__{func}", args }`.

### IR Merging

The driver lowers each module to IR separately, then concatenates all `IrFunction` vectors into one `Module.functions`. Order: dependencies first, main last (matches topological sort).

### No Struct/Enum Mangling Needed

Struct field operations use numeric indices in IR. Enum operations use numeric tags. Names are only relevant during type checking. No IR-level changes needed for cross-module struct/enum types.

## Codegen Changes

None. Codegen receives a flat list of IR functions with mangled names and compiles them. No awareness of modules needed.

## Driver Changes

The driver (`crates/cyflym-driver/src/main.rs`) is updated to:

1. Parse the entry point file.
2. Recursively discover and parse all imported files.
3. Build the import graph, detect cycles.
4. Topological sort.
5. Type-check in dependency order.
6. Lower each module to IR with name mangling.
7. Merge all IR into one Module.
8. Codegen and link as before.

### Import Resolution Interface

Import resolution lives in a new module in the driver crate: `crates/cyflym-driver/src/imports.rs`.

```rust
pub struct ResolvedModule {
    pub name: String,           // module prefix, e.g., "user"
    pub path: PathBuf,          // absolute path to .cy file
    pub program: Program,       // parsed AST
}

/// Recursively resolves all imports starting from the entry point.
/// Returns modules in topological order (dependencies first, entry point last).
/// Errors on circular imports or missing files.
pub fn resolve_imports(
    entry_point: &Path,
) -> Result<Vec<ResolvedModule>, CompileError>
```

The driver calls `resolve_imports`, then iterates the result in order, type-checking each module and building `ModuleExports`, then lowering each to IR with name mangling, then merging all IR into one `Module`.

## Testing

### Unit Tests (~14 new)

**Lexer (1):**
- Tokenize `import` keyword

**Parser (~3):**
- Parse `import "utils"` declaration
- Parse multiple imports at top of file
- Error: import after function declaration

**Type Checker (~7):**
- Cross-module function call type checks correctly
- Cross-module function with struct return type resolves
- Error: unknown function in module
- Error: unknown module name used as prefix
- Error: circular import detected
- Error: field access on module name (without call parens)
- Duplicate import of same module is not an error

**IR (~2):**
- Cross-module call lowers to mangled function name
- Functions from imported module present in merged IR

**Driver (~1):**
- `resolve_imports` returns modules in topological order

### E2E Tests (~4 new)

**`import_basic/main.cy` + `import_basic/utils.cy`** — Main imports utils, calls a function, exits with return value.

**`import_nested/main.cy` + `import_nested/models/user.cy`** — Main imports `"models/user"`, calls `user.create()`, exits with return value. Tests nested path resolution.

**`import_chain/main.cy` + `import_chain/a.cy` + `import_chain/b.cy`** — Main imports a, a imports b, main calls a.func() which internally calls b.func(). Tests transitive imports.

**`import_struct/main.cy` + `import_struct/models.cy`** — Main imports models, calls a constructor function that returns a struct, accesses fields. Tests cross-module struct types.

### Estimated Total: ~243 tests (225 existing + ~18 new)

## Deferred

- `pub` keyword / private-by-default visibility
- Selective imports (`import { X, Y } from "mod"`)
- Import aliases (`import "foo" as f`)
- Trait and impl block imports
- Standard library module resolution
- External dependency resolution / `cyflym.toml`
- Separate compilation / incremental builds
- Direct cross-module struct literal construction (`mod.Struct { ... }`)
- Direct cross-module enum variant construction (`mod.Enum::Variant`)
- Duplicate last-segment detection (`import "a/utils"` + `import "b/utils"`)
