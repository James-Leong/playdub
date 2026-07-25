# Contributing to Playdub

Thank you for helping build Playdub. Contributions of code, design, testing, documentation,
translation, and license research are welcome.

## Before opening a pull request

1. Search existing issues and discussions.
2. Open an issue before large architecture changes, new native dependencies, new model families,
   network features, or user-data collection.
3. Read [AGENTS.md](./AGENTS.md), even if you are not using an AI coding tool.
4. Keep one pull request focused on one outcome.

## Development

Create a branch from `main`, install the locked dependencies, and run:

```bash
pnpm install --frozen-lockfile
pnpm lint
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

Use Conventional Commit subjects where practical, for example `feat(subtitles): stabilize partial
segments`. Pull requests are squash-merged, so a clean PR title matters more than perfect local
history.

## Pull request requirements

- Explain the user-visible outcome and important design choices.
- Add tests or explain why no test is appropriate.
- Update public, architecture, and legal documentation when affected.
- Declare AI assistance and identify any generated assets or code.
- Complete the dependency/model license checklist when applicable.
- Do not include local installers, model weights, copyrighted media, or secrets.

All required GitHub Actions checks must pass. A build artifact from an untrusted fork is for testing
only and must not be treated as an official release.

## Sign-off

Contributions must be signed off using the
[Developer Certificate of Origin](https://developercertificate.org/):

```text
Signed-off-by: Your Name <your.email@example.com>
```

Use `git commit -s` to add the line. By signing off, you certify that you have the right to submit
the contribution under the project's license.

## Community

Be respectful, assume good intent, and critique ideas rather than people. Harassment, discrimination,
and publication of another person's private information are not accepted. Maintainers may remove
content or participation that makes the project unsafe or unproductive.
