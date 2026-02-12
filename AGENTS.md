# AGENTS.md
Operational guide for coding agents working in this repository.
Scope: entire repo rooted at `focus-must/`.

## 1) Project Snapshot
- Stack: Tauri 2 + Vue 3 + TypeScript + Vite.
- Frontend lives in `src/`; backend lives in `src-tauri/`.
- Package manager: npm (`package-lock.json` exists).
- TypeScript strict mode is enabled (`tsconfig.json`).

## 2) Source of Truth Files
- JS/TS scripts: `package.json`
- TS config: `tsconfig.json`, `tsconfig.node.json`
- Vite config: `vite.config.ts`
- Tauri config: `src-tauri/tauri.conf.json`
- Rust crate config: `src-tauri/Cargo.toml`

## 3) Build / Lint / Test Commands
Run from repo root unless noted.

### Install
```bash
npm install
```

### Dev
```bash
npm run dev
```
Starts Vite dev server.

### Build
```bash
npm run build
```
This runs:
- `vue-tsc --noEmit`
- `vite build`

### Tauri
```bash
npm run tauri dev
npm run tauri build
```
Notes:
- `src-tauri/tauri.conf.json` sets `beforeDevCommand` to `npm run dev`.
- `beforeBuildCommand` is `npm run build`.

### Rust checks (from `src-tauri/`)
```bash
cargo check
cargo test
```

## 4) Single Test Execution (Important)
Current state:
- No JS test runner is configured (no `test` script in `package.json`).
- No Rust tests are currently present in source files.

If Rust tests are added, run one test with:
```bash
# from src-tauri/
cargo test test_name
cargo test module_path::test_name -- --exact
```

If frontend tests are added later (for example Vitest), add scripts first, then use:
```bash
npm run test -- path/to/file.test.ts
```

Do not assume frontend single-test support until a `test` script exists.

## 5) Lint / Format Reality
- No repo-level ESLint config found.
- No repo-level Prettier config found.
- No `.editorconfig` found.
- Preserve surrounding file style and keep diffs localized.
- Do not run broad format sweeps.

## 6) Cursor / Copilot Rules
Checked and not found:
- `.cursorrules`
- `.cursor/rules/`
- `.github/copilot-instructions.md`

If added later, treat them as higher-priority local instructions.

## 7) Repository Conventions (Observed)

### Architecture and layout
- `src/composables/`: reusable Vue composition logic.
- `src/components/`: focused/presentational UI.
- `src/styles/main.css`: global variables and shared classes.
- `src-tauri/src/lib.rs`: core backend logic and Tauri command handlers.

### TypeScript and Vue
- Vue SFCs use `<script setup lang="ts">`.
- Prefer explicit interfaces for payload/state shapes.
- Use typed `Ref<T>`, `computed`, and `watch`.
- Keep async UI actions as `async` functions with `await`.
- Cleanup timers/listeners in `onUnmounted`.

### Imports
- Group imports: framework/external first, local second.
- Use type imports where appropriate (`type UnlistenFn` pattern exists).
- No path aliases are configured; use relative imports.

### Naming
- TS variables/functions: `camelCase`.
- TS types/interfaces/components: `PascalCase`.
- Rust functions/modules/fields: `snake_case`.
- Rust structs/enums/traits: `PascalCase`.
- Tauri command names exposed to frontend use snake_case strings.

### Data shape and contracts
- Frontend mirrors backend snake_case field names for Tauri payloads.
- Avoid silent shape conversions unless explicitly needed.
- Keep shared contracts explicit with interfaces/types.

### Error handling
- No empty catches.
- Log with context (example: `Failed to load history`).
- Prefer early returns for guard conditions.
- Avoid broad panic-prone Rust changes; existing code uses `unwrap` in controlled paths.

## 8) Editing Rules for Agents
- Do not edit generated/build output:
  - `node_modules/`
  - `dist/`
  - `src-tauri/target/`
  - `src-tauri/gen/schemas/`
- Make focused changes; avoid unrelated refactors in bugfixes.
- Keep command and event names stable unless migration is required.
- Preserve existing UX copy unless user asks for copy changes.

## 9) Validation Checklist Before Finishing
Run the smallest relevant set:
```bash
npm run build
```

For backend-only changes, also run:
```bash
# from src-tauri/
cargo check
```

If tests exist in touched areas, run targeted tests first, then broader suites.
