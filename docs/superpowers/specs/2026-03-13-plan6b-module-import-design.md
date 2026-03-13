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
- **Cross-module struct/enum syntax:** `user.Point { x: 1 }` — the type checker recognizes the module prefix and resolves to the struct/enum from that module. This requires the parser to handle `module.StructName { fields }` — which currently parses as field access followed by a block. A new `Expr::ModuleAccess { module, name, span }` may be needed, or the type checker can handle resolution from existing AST nodes. The simplest approach: struct literals already parse `Identifier { fields }`. For cross-module structs, the parser sees `user.Point` as a field access expression, NOT a struct literal. So cross-module struct construction needs a new parsing path: `Identifier.Identifier { fields }` → `Expr::QualifiedStructLiteral { module, name, fields, span }`. Similarly for enums: `user.Color::Red` needs `Expr::QualifiedEnumVariant`.
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

Cross-module struct literals and enum variants require special handling since `user.Point { x: 1 }` would not parse as a struct literal under current rules. Two approaches:

**Approach A (recommended):** Defer cross-module struct literals and enum variants to a future plan. For now, imported modules can only expose functions. Structs and enums are importable for use as types in function signatures (the type checker resolves `user.User` as a type), but cannot be directly constructed with `user.User { ... }` syntax. Instead, modules provide constructor functions (e.g., `user.create()`). This is the idiomatic pattern anyway.

**Approach B:** Add qualified syntax for struct literals and enum variants. More parser work, more complex.

**Decision: Approach A.** Cross-module structs/enums are usable as types (for return values and parameters) but constructed via module functions. Direct cross-module struct literal construction deferred.

## Type System

### Module Registry

```rust
pub struct ModuleExports {
    pub functions: HashMap<String, FunctionSignature>,
    pub structs: HashMap<String, StructDef>,
    pub enums: HashMap<String, EnumDef>,
}

pub struct FunctionSignature {
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
}
```

### Type Checking Rules

| Expression | Rule |
|---|---|
| `import "path"` | Resolve file, parse, check. Build ModuleExports. |
| `mod.func(args)` | Look up `func` in module `mod`'s exports. Check arg types match signature. Return type is function's return type. |
| Return type `mod.Struct` | Resolve struct definition from module's exports. |
| `for x in mod.func()` | If `mod.func()` returns `Array<T>`, `x` binds as `T`. |

### Type Checking Order

1. Build import graph from all files.
2. Detect cycles → error if found.
3. Topological sort: check leaf modules first.
4. Check each module, building its `ModuleExports`.
5. When checking a module, its dependencies' exports are available for cross-module resolution.

### Type Errors

- `import "foo"` but `foo.cy` not found → `"module not found: foo"`
- `foo.bar()` but `bar` not in foo's exports → `"function 'bar' not found in module 'foo'"`
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

Import resolution logic can live in a new module in the driver crate, or in a shared utility. The driver is the natural home since it owns the compilation pipeline.

## Testing

### Unit Tests (~11 new)

**Lexer (1):**
- Tokenize `import` keyword

**Parser (~3):**
- Parse `import "utils"` declaration
- Parse multiple imports at top of file
- Error: import after function declaration

**Type Checker (~5):**
- Cross-module function call type checks correctly
- Cross-module function with struct return type resolves
- Error: unknown function in module
- Error: unknown module name used as prefix
- Error: circular import detected

**IR (~2):**
- Cross-module call lowers to mangled function name
- Functions from imported module present in merged IR

### E2E Tests (~3 new)

**`import_basic/main.cy` + `import_basic/utils.cy`** — Main imports utils, calls a function, exits with return value.

**`import_chain/main.cy` + `import_chain/a.cy` + `import_chain/b.cy`** — Main imports a, a imports b, main calls a.func() which internally calls b.func(). Tests transitive imports.

**`import_struct/main.cy` + `import_struct/models.cy`** — Main imports models, calls a constructor function that returns a struct, accesses fields. Tests cross-module struct types.

### Estimated Total: ~239 tests (225 existing + ~14 new)

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
