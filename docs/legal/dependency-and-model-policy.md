# Dependency and model policy

This document is an engineering policy, not legal advice.

## Source-code license

Playdub-authored source code is Apache-2.0. That license does not apply to third-party dependencies,
native libraries, codecs, model weights, tokenizers, datasets, fonts, or test media.

## Dependency admission

Every production dependency must have:

1. an upstream project and immutable version;
2. a recognized license and accessible license text;
3. a documented purpose with no materially simpler existing dependency;
4. maintenance and security assessment;
5. compatibility with supported platforms; and
6. review of static/dynamic linking and distribution obligations.

Permissive licenses such as Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, and ISC are generally
reviewable through a normal pull request.

Copyleft, weak-copyleft, source-available, non-commercial, custom, or unclear terms require a
dedicated legal-review issue. This includes GPL, AGPL, LGPL, SSPL, Commons Clause, and any model
license containing use restrictions. Approval must be explicit and documented; the category is not
an automatic rejection.

## Media stack

FFmpeg and mpv build configuration changes their effective distribution obligations. Official
builds must:

- use a reproducible, documented configuration;
- inventory linked libraries and enabled codecs;
- generate applicable notices and corresponding-source instructions;
- avoid assuming codec patent rights are granted by an open-source license; and
- pass a release legal review before binaries are published.

Do not download an arbitrary prebuilt codec or mpv binary in CI.

The prototype does not distribute or link FFmpeg. It launches a user-selected FFmpeg executable to
extract working audio. This lowers Playdub's distribution surface but does not grant the user a
license to that binary or its enabled codecs. For the documented conservative configuration, see
[prototype setup](../prototype/setup.md).

## User-supplied inference tools

The prototype can launch user-selected `whisper-cli` and, optionally, `llama-cli`. The upstream
whisper.cpp and llama.cpp source projects state MIT licenses, but that says nothing about a selected
model file. Project documentation must never describe the CLI's software license as the model's
license.

Paths selected in the prototype are local, unsupported configuration. They are not registry
approval. A future official build that bundles an executable, downloads it, recommends a specific
model, or provides a one-click model installation changes the distribution and endorsement surface
and therefore requires a fresh dependency/model review.

## Model admission

Model files are not committed to Git and are not bundled by default. An official downloadable model
requires a reviewed entry in `models/registry.json` containing:

- stable identifier and version;
- upstream model and immutable revision;
- exact files, sizes, and SHA-256 hashes;
- model-weight and tokenizer licenses;
- upstream license and model-card URLs;
- supported languages and intended function;
- distribution mode;
- known use restrictions;
- provenance review date and reviewer; and
- optional runtime and hardware requirements.

The following are blockers:

- absent or ambiguous weight license;
- non-commercial restriction for a generally distributed official package;
- no-redistribution clause when Playdub would mirror the file;
- missing tokenizer license;
- mutable download without a digest;
- attribution requirements the application cannot satisfy; or
- uncertainty about whether the uploader had rights to publish the weights.

Users may configure unregistered local models, but the UI must label them unsupported and must not
imply endorsement or license safety.

Current reviewed prototype additions are recorded in
[the prototype dependency review](./prototype-dependency-review.md).

## Model delivery

- Download only after informed user action.
- Show download size, source, license, and supported languages.
- Verify SHA-256 before extraction.
- Reject absolute paths, parent traversal, links, and unexpected files in archives.
- Store each model version immutably and switch atomically.
- Allow complete deletion.
- Never execute code shipped inside a model archive.

## Release evidence

Each release should retain:

- dependency lock files;
- native dependency build scripts and source revisions;
- software bill of materials;
- third-party notices;
- model registry revision;
- GitHub Actions provenance and checksums; and
- signing/notarization records.
