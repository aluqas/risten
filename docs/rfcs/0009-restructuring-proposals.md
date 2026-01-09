# Risten Restructuring & Crate Split Proposals

This document outlines options for reorganizing the `risten` codebase to improve maintainability, extensibility, and clarity.

## Part 1: Directory/Module Restructuring Proposals (Inside `risten/src`)

Current State:

- `model/` (Core traits)
- `orchestrator/` (Dispatch logic)
- `source/` (Registry/Router)
- `delivery/` (Execution strategy)
- Root level files mixed in.

### Option A: Standard Layered (Classic)

Clear separation of concerns based on the architectural layer.

```
risten/src/
├── core/           # Pure traits and fundamental types (Hook, Message, Handler)
├── registry/       # Registry implementations and builders
├── dispatch/       # Dispatching logic (Standard, Static, Dynamic)
├── routing/        # Router traits and implementations (Trie, Phf)
├── delivery/       # Execution strategies (Sequential, Fanout)
└── prelude.rs      # Common exports
```

**Pros:** Easy to navigate, familiar to most Rust developers.
**Cons:** Can lead to "bucket" modules if not careful.

### Option B: Feature-Oriented

Grouping by "what it does" rather than "what it is".

```
risten/src/
├── pipeline/       # Everything related to the execution pipeline
├── router/         # Routing logic and data structures
├── handler/        # Handler definitions and adapters
├── hook/           # Hook trait and standard hooks
└── macros/         # Re-exports or internal helpers
```

**Pros:** High cohesion within features.
**Cons:** Some cross-cutting concerns (like Error) might be hard to place.

### Option C: API vs Runtime (Clean)

Separates the user-facing API from the internal engine.

```
risten/src/
├── api/            # Public facing traits and types (The "Contract")
│   ├── hook.rs
│   ├── message.rs
│   └── builder.rs
├── engine/         # Internal mechanics (The "Machine")
│   ├── dispatch/
│   ├── delivery/
│   └── registry/
└── std/            # Standard implementations provided out-of-the-box
```

**Pros:** Very clean public API surface. Good for library stability.
**Cons:** Jumping between `api` and `engine` during development.

### Option D: Static/Dynamic Split

Emphasizes the dual nature of the framework.

```
risten/src/
├── common/         # Shared traits
├── static/         # Const generics, static dispatch, compile-time optimized
├── dynamic/        # Boxed traits, runtime dispatch, flexible
└── bridge/         # Adapters to mix static and dynamic
```

**Pros:** Highlights the unique "Static First" philosophy.
**Cons:** Documentation might be split; Code duplication risk.

### Option E: Flat & Functional

Minimal nesting, promoting a flat structure for easier imports.

```
risten/src/
├── hooks.rs        # Hook traits
├── handlers.rs     # Handler traits
├── messages.rs     # Message traits
├── registry.rs     # Registry structs
├── dispatchers/    # (Submodule for complex dispatchers)
├── routers/        # (Submodule for complex routers)
└── lib.rs
```

**Pros:** Simple, very "Rust-y" for smaller crates.
**Cons:** Might get cluttered as the project grows.

---

## Part 2: Crate Splitting Proposals

Current State: `risten` (monolith-ish), `risten-macros`, `risten-utils`.

### Option 1: The "Standard" Split (Recommended)

Typical Rust ecosystem pattern.

- `risten-core`: Only the traits (`Hook`, `Router`, `Message`). Minimal dependencies.
- `risten-std`: Standard implementations (`VecRegistry`, `StandardDispatcher`).
- `risten-macros`: Proc-macros.
- `risten`: Facade crate re-exporting everything + prelude.

**Pros:** Users can depend on `risten-core` for implementing plugins without pulling in the whole engine.
**Cons:** Version synchronization management.

### Option 2: Granular Features

Splitting by major functional component.

- `risten-core`
- `risten-routing`: Advanced routing (Trie, Phf, Regex).
- `risten-dispatch`: Advanced dispatchers (Static, Fanout).
- `risten-macros`
- `risten`: Facade.

**Pros:** Users pay only for what they use (compile time).
**Cons:** Dependency graph complexity.

### Option 3: Static/Dynamic Separation

Splitting based on the execution model.

- `risten-core`
- `risten-static`: The static optimization machinery (const generics).
- `risten-dynamic`: The dynamic runtime (box, arc).
- `risten`: Facade.

**Pros:** Allows purely static, allocation-free builds for embedded/performance-critical uses.
**Cons:** Might fragment the ecosystem if not careful.

### Option 4: Unified Monolith with Features

Don't split crates, use Cargo Features.

- `risten`
  - `feature="static"`
  - `feature="dynamic"`
  - `feature="routing-trie"`
  - `feature="routing-phf"`
  - ...

**Pros:** Easiest for users to manage (single version). Easiest to develop (one repo place).
**Cons:** `cargo build` times for the whole crate; less granular visibility control.

### Option 5: The "Micro-Kernel"

Extremely minimal core.

- `risten-api`: Pure interface definitions (no implementation logic).
- `risten-runtime`: The reference implementation.
- `risten-contrib`: Community/Extra implementations.

**Pros:** strict separation of interface and implementation.
**Cons:** Overkill for the current size of the project.

---

## Recommendation

**Module Structure:** **Option A (Standard Layered)** or **Option C (API vs Runtime)** provides the best balance of organization and clarity for a framework of this complexity.

**Crate Split:** **Option 1 (Standard Split)** or **Option 4 (Monolith with Features)**. Given the current size, starting with **Option 4** and migrating to **Option 1** later might be the path of least resistance, but **Option 1** sets a better long-term foundation.

---

## Brainstorming: 20 Potential Crate Cuts

Just listing out every possible granular component that *could* be its own crate.

1. `risten-core`: The absolute minimum traits (`Hook`, `Message`, `Handler`). `no_std` compatible.
2. `risten-macros`: Procedural macros (`#[risten::main]`, `#[derive(video)]`).
3. `risten-routing`: `Router` traits and basic implementations.
4. `risten-routing-trie`: Specific implementation of Radix Trie router.
5. `risten-routing-phf`: Compile-time perfect hash function router (heavy build dependency).
6. `risten-routing-regex`: Regex-based router (heavy runtime dependency).
7. `risten-dispatch`: `Dispatcher` traits and logic.
8. `risten-dispatch-static`: `const` generic based dispatchers (`HList` logic).
9. `risten-dispatch-dynamic`: `Vec<Box<dyn Hook>>` based standard dispatchers.
10. `risten-registry`: The container logic for holding hooks/handlers.
11. `risten-delivery`: Execution strategies (Sequential, Parallel, Fanout).
12. `risten-context`: Context state management (Extractors, parsing).
13. `risten-error`: Error types and handling primitives (`IntoResponse`).
14. `risten-tower`: Integration layer with `tower::Service` and `tower::Layer`.
15. `risten-tracing`: Observability, logging, and metrics integration (`tracing` crate).
16. `risten-test`: Test utilities, mocks, and assertions for users.
17. `risten-codegen`: Internal code generation logic helper for macros.
18. `risten-utils`: Shared low-level utilities (pinning, futures helpers).
19. `risten-fs`: File-system based router/loader (like Next.js pages router but for events).
20. `risten-cli`: A CLI tool for scaffolding or debugging risten apps.
