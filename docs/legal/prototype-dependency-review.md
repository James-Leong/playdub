# Prototype dependency review

Status: engineering review complete; release notice generation still required
Reviewed: 2026-07-26

This is an engineering record, not legal advice. Exact transitive versions and checksums remain in
`pnpm-lock.yaml` and `src-tauri/Cargo.lock`; those lock files do not replace license texts or a
release SBOM.

## Native file dialog

| Field | Decision |
| --- | --- |
| Packages | `@tauri-apps/plugin-dialog` 2.7.2 and `tauri-plugin-dialog` 2.7.2 |
| Upstream | [Tauri plugins workspace](https://github.com/tauri-apps/plugins-workspace) |
| Declared license | `MIT OR Apache-2.0` |
| Purpose | Native media/executable/model selection and dynamic session scope for selected paths |
| Simpler existing option | A browser file input cannot provide a trusted absolute path to the Rust analysis process |
| Distribution | Source/runtime code is included through normal pnpm/Cargo builds; no external binary or model is added |
| Security boundary | Only `dialog:default` is granted; selected media is canonicalized before narrow asset-protocol authorization |
| Platform scope | Official Tauri plugin for desktop; compilation is exercised by the GitHub macOS/Windows/Linux matrix |

The permissive dual-license declaration is eligible for ordinary dependency review under the
project policy. The exact JavaScript package metadata and Cargo lock entry both resolve to 2.7.2.
Before a public binary release, generated notices/SBOM must include its transitive dependency graph
and actual license texts.

## External CLI tools

FFmpeg, whisper.cpp, llama.cpp, and model files are not package dependencies and are not
distributed. Their user-supplied status is recorded in
[ADR 0001](../architecture/decisions/0001-user-supplied-cli-prototype.md) and does not pre-approve
future bundling or downloading.
