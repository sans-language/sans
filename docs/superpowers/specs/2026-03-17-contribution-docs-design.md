# Contribution Documentation Design Spec

**Goal:** Create contribution documentation that serves both human developers and AI agents, with CONTRIBUTING.md as the narrative entry point and CLAUDE.md as the machine-readable authority.

**Approach:** No duplication — CONTRIBUTING.md explains the "why" and links to CLAUDE.md for the "what." AI agents read both; humans start with CONTRIBUTING.md.

---

## Prerequisite

**Update CLAUDE.md stale references.** The Architecture section still references "8 C runtime files" and `.c` files. The runtime is now 100% self-hosted Sans (13 `.sans` files, zero C). The following CLAUDE.md sections must be corrected before shipping CONTRIBUTING.md:
- Architecture: change "8 C runtime files" to "13 Sans runtime modules under `runtime/`" with the actual filenames
- "Adding a New Built-in Function" step 5: change "If backed by C: add function to appropriate `runtime/*.c` file" to reference `runtime/*.sans`
- "Adding a New Type" step 7: change "If opaque: add C runtime backing" to reference Sans runtime
- Conventions: update "Opaque types ... backed by C runtime with `cy_` prefix" to reflect Sans runtime with `sans_` prefix

This is a blocking prerequisite — CONTRIBUTING.md links to CLAUDE.md as the "authoritative rule set," so CLAUDE.md must be accurate.

---

## Deliverables

### 1. CONTRIBUTING.md

Human-friendly guide with eight sections:

**1.1 Welcome**
- 2-3 sentences: Sans is AI-first, contributions from humans and AI agents welcome.
- Link to CODE_OF_CONDUCT.md.

**1.2 Quick Setup**
- Platform note: macOS is the primary development platform. Linux contributors need to adapt the LLVM path (e.g., `apt install llvm-17-dev`, set `LLVM_SYS_170_PREFIX=/usr/lib/llvm-17`). Windows is not currently supported for development.
- Prerequisites: Rust (stable), LLVM 17, Xcode CLT (macOS).
- Build: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo build`
- Test: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test`
- Copy-pasteable commands, minimal explanation.

**1.3 Architecture Overview**
- The 6-crate Rust compiler pipeline: sans-lexer -> sans-parser -> sans-typeck -> sans-ir -> sans-codegen -> sans-driver.
- One paragraph per crate explaining purpose and location.
- The self-hosted compiler in `compiler/` (~11,600 LOC, 7 modules) — what it is, how it relates to the Rust compiler. Clarify: "Feature additions target the Rust compiler pipeline. The self-hosted compiler is a separate implementation maintained in parallel — changes there are not required unless explicitly noted."
- The self-hosted runtime in `runtime/` — all `.sans` files, no C. Compiled from source on each build.
- Test structure: unit tests in each crate, E2E in `crates/sans-driver/tests/e2e.rs`, fixtures in `tests/fixtures/`.

**1.4 How to Add a Feature (Worked Example)**
- Narrative walkthrough of adding a hypothetical built-in function.
- Explains why each pipeline stage exists (typeck validates types, IR abstracts over codegen, codegen emits LLVM, driver links).
- Explains what breaks if you skip a stage.
- Links to CLAUDE.md "Adding a New Built-in Function" for the precise checklist.
- Mentions the three pipelines: built-in function, method on a type, new type.
- Notes: "The self-hosted compiler in `compiler/` is a separate codebase. Unless the feature involves the self-hosted compiler directly, you only need to modify the Rust crates."

**1.5 AI Agent Contributors**
- Directed at AI agents or humans directing AI agents at the repo.
- Points to CLAUDE.md as the authoritative rule set — read it fully before starting.
- Notes available Claude Code skills in `docs/superpowers/`: brainstorming (design exploration), writing-plans (implementation planning), subagent-driven-development (parallel task execution), requesting-code-review (self-review before PR). These are Claude Code plugin skills that structure the development workflow.
- Explains self-review requirement: run the code-review skill before requesting human review.
- Notes the AI-optimized syntax rule: "Can this be expressed in fewer tokens?"

**1.6 Pull Request Process**
- Workflow: fork -> branch -> implement -> self-review -> PR.
- Mandatory in every PR: documentation updates (all 8 items per CLAUDE.md Documentation Update Checklist), tests, no compiled binaries.
- Do not manually bump version numbers — version is managed by CI when the maintainer pushes a release tag.
- Links to CLAUDE.md "Versioning" and "Documentation Update Checklist" sections.
- Notes: the PR template checklist will remind you of all mandatory steps.

**1.7 Common Gotchas**
- Do not manually bump version numbers — CI handles this on tag push. The most common mistake was forgetting files; automation eliminates it.
- Documentation updates span 8 places — missing any one will be caught in review.
- `!` is bitwise NOT, not logical NOT. Use `== 0` for logical negation.
- No GC — heap allocations are leaked. Use `arena_begin()`/`arena_alloc(n)`/`arena_end()` for bulk deallocation.
- E2E test fixtures must use unique temp filenames to prevent parallel test races.

**1.8 Getting Help**
- Open a GitHub issue using the bug report or feature request template.
- For questions about the codebase, open a discussion or issue.

### 2. CODE_OF_CONDUCT.md

- Contributor Covenant v2.1 (full standard text).
- Enforcement contact: open a GitHub issue on the sans-language/sans repository with the label "conduct". (Note: the Contributor Covenant recommends a private contact method. For now, GitHub issues with a specific label is acceptable for a project at this scale. If a private reporting channel is needed later, add a maintainer email.)

### 3. .github/PULL_REQUEST_TEMPLATE.md

```markdown
## Summary
<!-- What does this PR do and why? -->

## Checklist
- [ ] Tests pass (`LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test`)
- [ ] Version bumped in all required files (see CLAUDE.md Versioning)
- [ ] Docs updated per CLAUDE.md Documentation Update Checklist (reference.md, ai-reference.md, docs.html, HOVER_DATA, syntax highlighting, test fixtures, examples, README)
- [ ] Code self-reviewed (AI contributors: use superpowers:requesting-code-review)
- [ ] No compiled binaries committed

## Test plan
<!-- How did you verify this works? -->
```

### 4. .github/ISSUE_TEMPLATE/bug_report.md

YAML frontmatter with `name: Bug Report`, `about: Report a bug`, `labels: bug`.

Fields:
- **Sans version** (`sans --version` output)
- **OS / architecture** (e.g., macOS 15, ARM64)
- **Code to reproduce** (minimal `.sans` file)
- **Expected behavior**
- **Actual behavior** (include compiler output / error message)

### 5. .github/ISSUE_TEMPLATE/feature_request.md

YAML frontmatter with `name: Feature Request`, `about: Suggest a new feature`, `labels: enhancement`.

Fields:
- **What the feature does** (one sentence)
- **Proposed syntax** (with prompt: "Can this be expressed in fewer tokens?")
- **Example code** (`.sans` showing usage)
- **Pipeline stages affected** (optional: typeck / IR / codegen / runtime)
- **Documentation impact** (which of the 8 doc targets would this affect?)

### 6. README.md modification

Add a "Contributing" section before "Known Limitations":
- One sentence: "See [CONTRIBUTING.md](CONTRIBUTING.md) for how to set up, add features, and submit pull requests."
- One sentence: "AI agents: read [CLAUDE.md](CLAUDE.md) for the complete rule set."

---

## Design Decisions

1. **No duplication** — CONTRIBUTING.md links to CLAUDE.md for checklists and pipelines. CLAUDE.md is the single source of truth for machine-checkable rules.
2. **AI-first, human-readable** — CONTRIBUTING.md provides the narrative context that CLAUDE.md lacks. AI agents benefit from both.
3. **PR template as safety net** — the checklist catches the most common mistakes (forgotten version bump, missing docs) at PR creation time. Enumerates all 8 doc targets explicitly.
4. **Issue templates are lightweight** — just structured fields, no boilerplate essays. Feature request includes "documentation impact" field to prime contributors early.
5. **Contributor Covenant** — standard, widely recognized, low maintenance. Contact via GitHub issues for now.
6. **Self-hosted compiler clarified** — CONTRIBUTING.md explicitly states feature additions target the Rust pipeline only unless noted. Prevents confusion about whether both compilers need updating.
7. **Platform explicitly scoped** — macOS primary, Linux possible with adaptation, Windows unsupported. Prevents contributors from hitting undocumented walls.

## Out of Scope

- CI workflow for automated testing (separate concern, not part of contribution docs)
- DCO/CLA sign-off (not needed for MIT-licensed project at this stage)
- Governance model (single maintainer for now)
- Private reporting channel for Code of Conduct (can be added later if needed)
