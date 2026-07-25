# Roadmap

The roadmap describes sequence, not release promises.

## M0 — Foundation

- [x] Tauri, React, and Rust application shell
- [x] open-source governance and contributor documentation
- [x] GitHub CI and desktop build workflows
- [x] dependency and model license gates

## P0 — Usable local prototype

- [x] local file selection through the trusted Rust core
- [x] WebView video playback with play, pause, seek, volume, and fullscreen
- [x] background audio extraction through a user-supplied FFmpeg executable
- [x] local SRT transcription through user-supplied whisper.cpp CLI and GGML model
- [x] Whisper speech-to-English translation path
- [x] optional llama.cpp/GGUF text-translation path for other target languages
- [x] synchronized bilingual rendering and non-overwriting SRT/WebVTT export
- [x] external-tool prototype architecture decision and setup documentation

Prototype status means the path is runnable; it does not mean every item has passed a cross-platform
sample suite. Current packages are unsigned, tools/models are user supplied, playback format support
depends on the system WebView, inference is batch-oriented, and translation quality is not certified.

## M1 — Cross-platform playback validation

- [ ] run the same licensed smoke-test media in GitHub-hosted macOS, Windows, and Linux sessions
- [ ] validate WebView codec support and publish an exact compatibility matrix
- [ ] decide whether to retain WebView playback or integrate a reviewed native media engine
- [ ] add audio-track selection, keyboard controls, and error recovery
- [ ] configure signing/notarization before calling packages releases

## M2 — Incremental subtitle pipeline

- [ ] independent audio pre-read and bounded chunk scheduling
- [ ] VAD and versioned crash-isolated ASR worker protocol
- [ ] draft/stable caption state machine and generation-based seek cancellation
- [ ] persistent, versioned SQLite subtitle cache
- [ ] deterministic tests for corrupt media and worker crashes

## M3 — Reviewed model experience

- [ ] populate the model registry only after weight and tokenizer review
- [ ] checksummed, resumable, user-initiated model download manager
- [ ] at least one approved language path
- [ ] bilingual line breaking and quality/performance profiles

## M4 — Desktop beta

- [ ] model recovery and worker crash isolation
- [ ] accessibility and keyboard audit
- [ ] signed Windows packages
- [ ] signed and notarized macOS packages
- [ ] SBOM, complete third-party notices, and release provenance

Mobile planning begins only after desktop scheduling and model policy are stable.
