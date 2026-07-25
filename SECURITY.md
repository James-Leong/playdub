# Security policy

## Reporting a vulnerability

Do not report security or privacy vulnerabilities in a public issue.

Use GitHub's private vulnerability reporting feature for this repository. Include affected
versions, reproduction steps, impact, and any suggested mitigation. If private reporting has not
yet been enabled, contact the maintainers through the address listed in the repository's GitHub
security settings.

Please do not include real private media, transcripts, credentials, or model-provider tokens in a
report.

## Supported versions

Until the first stable release, only the latest commit on `main` receives security fixes. A support
matrix will be published before version 1.0.

## Security boundaries

Media files, subtitle files, model archives, and inference-worker output are untrusted input.
Playdub validates paths and archive contents, verifies model checksums, constrains Tauri
capabilities, and isolates native inference from the UI process.
