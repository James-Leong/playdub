# AI contributor guide

AI tools can assist Playdub development, but a human contributor remains responsible for every
submitted line, dependency, asset, benchmark, and license claim.

## Before asking an AI to change code

Give it:

- the issue and acceptance criteria;
- the repository-level `AGENTS.md`;
- relevant ADRs and architecture documents;
- the exact platform scope; and
- explicit non-goals.

Ask for clarification when a choice affects privacy, licensing, compatibility, public API, or
stored user data.

## Allowed uses

- scoped implementation and refactoring;
- deterministic tests;
- documentation drafts;
- investigation of upstream primary documentation;
- review of error handling, accessibility, and cancellation paths; and
- mechanical maintenance with human-reviewed output.

## Prohibited shortcuts

- copying code from search results without provenance;
- fabricating model licenses, benchmark numbers, or supported platforms;
- adding model weights or media copied from the internet;
- disabling CSP, signature checks, checksum checks, or tests;
- sending private media or repository secrets to a remote service;
- using an online API while describing the feature as on-device; and
- accepting generated native/unsafe code without focused review and tests.

## Dependency and model research

Use primary sources: upstream repositories, official documentation, SPDX records, model cards, and
the complete license file. Record an exact revision, not just a mutable project name.

For a proposed model, collect all fields required by `models/registry.schema.json`. Dataset or
training-code licenses do not automatically establish the model-weight license. If the model card
is ambiguous, the model is blocked.

## Review checklist

- Does the change match the issue without unrelated expansion?
- Are failure, cancellation, seeking, and stale results handled?
- Is playback isolated from expensive work?
- Are all paths and worker messages validated?
- Are tests deterministic and independent of downloaded models?
- Are UI controls keyboard reachable and clearly labelled?
- Are license and third-party notice changes included?
- Were all reported commands actually run?

Pull requests must disclose meaningful AI assistance in the PR template.
