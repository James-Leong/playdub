# GitHub desktop builds

Desktop packaging is centralized in
[`build-desktop.yml`](../../.github/workflows/build-desktop.yml). Local machines are for editing,
interactive debugging, and tests; a local package is never an official Playdub artifact.

## Current matrix

| Artifact | GitHub-hosted runner | Rust target | Status |
| --- | --- | --- | --- |
| `playdub-macos-arm64` | macOS 15 | `aarch64-apple-darwin` | unsigned, not notarized |
| `playdub-macos-x64` | macOS 15 Intel | `x86_64-apple-darwin` | unsigned, not notarized |
| `playdub-windows-x64` | Windows 2025 | `x86_64-pc-windows-msvc` | unsigned |
| `playdub-linux-x64` | Ubuntu 24.04 | `x86_64-unknown-linux-gnu` | unsigned |

The workflow runs manually through **Actions → Build unsigned desktop prototypes → Run workflow**
and on a `v*` tag. A tag currently produces retained workflow artifacts only; it does not create a
GitHub Release or make the package signed/release-grade.

## What the artifacts contain

The workflow installs locked JavaScript/Rust dependencies and Tauri's platform build prerequisites,
then builds the application source. It does not download or bundle FFmpeg, whisper.cpp, llama.cpp,
codecs, model weights, tokenizer files, or test media. Users must configure those local paths after
installing an artifact.

Artifacts are retained for 14 days and are intended for prototype testing. Do not describe a green
matrix as proof of runtime playback/ASR behavior: GitHub's package job compiles the app but has no
licensed media/model smoke-test fixture and no configured inference tools.

## Validation workflow

[`ci.yml`](../../.github/workflows/ci.yml) runs on pull requests and `main` pushes:

- pnpm locked install, lint/format, unit tests, and production frontend build;
- Rust formatting, Clippy with warnings denied, and unit tests; and
- the Linux Tauri prerequisites needed to compile the Rust application.

Branch protection should require both CI jobs before merge.

## Release gate

Before creating an actual public binary release:

1. pin and review the release workflow/actions and generate an SBOM;
2. complete third-party notices for everything actually distributed;
3. preserve corresponding source/build configuration for applicable native dependencies;
4. configure Windows signing and macOS signing/notarization using protected GitHub environments;
5. run the licensed cross-platform smoke-test matrix;
6. publish checksums and provenance; and
7. confirm no tool, model, temporary audio, generated subtitle, test media, or signing secret entered
   the package unexpectedly.

Until those gates pass, GitHub artifacts remain unsigned prototypes even when built from a version
tag.
