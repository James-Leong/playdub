# AGENTS.md

This file is the repository contract for AI coding agents. It applies to the entire repository.
More specific `AGENTS.md` files may add constraints but must not weaken these rules.

## Mission

Build Playdub as a local-first, cross-platform video player with on-device speech recognition and
subtitle translation. Protect playback stability, user privacy, accessibility, and open-source
license compliance.

## Non-negotiable rules

1. Never upload media, audio, transcripts, file paths, or model inputs unless the user explicitly
   enables a separately documented online feature.
2. Never add model weights, test media of uncertain origin, proprietary SDKs, or copied codec
   binaries to the repository.
3. Do not add a model or production dependency until its source, exact version, license, and
   distribution implications are documented.
4. Code under a non-commercial, source-available, GPL, AGPL, LGPL, or unknown license requires a
   dedicated legal review issue before integration. Do not assume compatibility.
5. Playback must not block on ASR, translation, model download, cache writes, or telemetry.
6. Do not weaken Tauri capabilities, CSP, path validation, checksum verification, or process
   isolation to make a feature easier.
7. Official installers are built in GitHub Actions. Never commit `dist/`, `target/`, installers,
   downloaded models, generated subtitles, signing material, or secrets.

## Architecture boundaries

- `src/`: presentation and typed calls into the Rust core. No native process spawning or direct
  filesystem access.
- `src-tauri/src/`: trusted orchestration, state machines, validated filesystem access, and IPC.
- `crates/`: reusable Rust domain components with no UI assumptions.
- `workers/`: isolated native inference processes and their versioned protocols.
- `models/registry.json`: metadata only; never weights.
- Playback time is the canonical clock. All subtitle results carry media timestamps and a task
  generation ID so stale work can be discarded after seeking.

New subsystem boundaries or dependency families require an ADR in `docs/architecture/decisions/`.

## Required workflow

Before editing:

1. Read the relevant architecture and legal documents.
2. Inspect the working tree and preserve unrelated user changes.
3. State assumptions when the issue does not define behavior.

Before declaring work complete:

```bash
pnpm lint
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

Run only the checks relevant to documentation-only changes. Never claim a platform package works
because it compiled on another OS; rely on the desktop build workflow.

## Testing expectations

- Add deterministic unit tests for state transitions, timestamp math, cache keys, parsers, and IPC.
- Add integration tests at process and storage boundaries.
- Keep model-dependent benchmarks out of ordinary unit tests.
- Test seeking, cancellation, corrupted input, missing models, and worker crashes.
- Use synthetic or clearly licensed test fixtures and record their provenance.

## AI contribution hygiene

- Do not invent benchmark results, licenses, APIs, or platform support.
- Cite upstream documentation in design documents for unstable technical claims.
- Keep changes small enough to review and explain generated or mechanically updated files.
- Do not add speculative abstractions without a current call site.
- Update documentation and tests in the same change as behavior.
- Record unresolved license or security questions as blockers, not TODOs hidden in code.
