# Local development

Local machines are used for editing, formatting, static analysis, unit tests, and interactive
debugging. Official installers and release artifacts are built by GitHub Actions.

## Toolchains

- Node.js 24+
- pnpm 11+
- stable Rust, with `rustfmt` and `clippy`
- OS packages required by Tauri

To exercise the AI prototype, also provide local paths to FFmpeg, `whisper-cli`, a Whisper GGML
model, and optionally `llama-cli` plus a compatible GGUF model. These are user tools, not repository
dependencies. See [prototype setup](../prototype/setup.md).

Install JavaScript dependencies using the committed lock file:

```bash
pnpm install --frozen-lockfile
```

## Commands

```bash
# Web UI
pnpm dev
pnpm lint
pnpm test
pnpm build

# Rust
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features

# Desktop development window
pnpm tauri dev
```

`pnpm tauri build` may be used locally only to diagnose packaging. Its output is unofficial,
unsigned, ignored by Git, and must not be published.

## Platform validation

Use the desktop build workflow for macOS Apple Silicon, macOS Intel, Windows x64, and Linux x64. A
successful Rust or frontend test on one platform is not evidence that native media or inference
integration works on another.

The workflow is named **Build unsigned desktop prototypes** in the Actions UI. See
[GitHub desktop builds](./github-builds.md) for the exact matrix and release gates.

Current workflow artifacts deliberately omit FFmpeg, whisper.cpp, llama.cpp, and model files. They
require the same user configuration as a development build.

Releases require signing and notarization configuration that is deliberately absent from the
prototype. Do not label unsigned workflow artifacts as official releases.

## Prototype verification

Run the deterministic automated checks first, then follow the
[prototype smoke-test checklist](../testing/prototype-smoke-test.md). Never use private media in a
public CI fixture or issue attachment.

## Test fixtures

Prefer programmatically generated signals and captions. Any media fixture must have:

- an explicit redistribution license;
- a source URL and author;
- the exact license text or SPDX identifier; and
- the smallest practical file size.

Record fixture provenance next to the fixture.
