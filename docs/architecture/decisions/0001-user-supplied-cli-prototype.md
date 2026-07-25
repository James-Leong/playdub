# ADR 0001: User-supplied CLI tools for the first inference prototype

- Status: accepted for prototype
- Date: 2026-07-26
- Owners: Playdub maintainers

## Context

Playdub needs a usable, local-only vertical slice before choosing and distributing a permanent
native media and inference stack. Bundling FFmpeg, whisper.cpp, llama.cpp, or model weights would
immediately add cross-platform build, update, security, notice, and license obligations. Model
licenses are independent of inference-runtime licenses.

The system WebView already provides a video element, but its formats differ by OS. FFmpeg can
reliably produce the PCM audio required by Whisper from many media formats. whisper.cpp exposes a
CLI for timestamped local recognition and speech-to-English translation. llama.cpp exposes a CLI
that can exercise an instruction-capable GGUF model for experimental text translation.

## Decision

For the prototype:

- playback uses an HTML5 video element in the Tauri system WebView;
- the Rust core launches only executable paths explicitly selected by the user;
- FFmpeg produces a private 16 kHz mono WAV working file;
- `whisper-cli` produces timestamped SRT output;
- a second Whisper pass may translate speech to English;
- optional `llama-cli` translates caption text to another requested language;
- all processes run locally without a shell or network integration;
- Playdub does not download, bundle, or endorse these tools or models; and
- late task results carry a generation ID and cannot overwrite a newer media session.

These paths are user configuration, not production dependencies approved for distribution.
Generation filtering does not currently cancel an already-running child process.

## Consequences

The prototype can demonstrate playback, transcription, and translation without adding native
binaries or model weights to the repository. Contributors can use different locally built tools.

Setup is manual. Reproducibility, performance, and translation quality vary. HTML video does not
provide broad media compatibility. External-process startup adds latency, batch SRT is not true
streaming recognition, and GGUF prompt translation cannot guarantee timestamp preservation or
language quality.

Each selected binary and model remains under its own terms. The application must display actionable
errors when a path is absent or incompatible; it cannot silently download a replacement.

## Security constraints

- Invoke executables directly with argument arrays; never interpolate a shell command.
- Canonicalize and validate user-selected executable, model, and media paths in the Rust core.
- Treat SRT and translator output as untrusted, enforce size limits, and render it as text.
- Create working files under an application-owned temporary directory with private permissions.
- Do not pass media content, transcripts, or paths to an online service.
- Bound every child process with a timeout and reap it after success, error, or timeout.

Active cancellation when the media generation changes and explicit shutdown cleanup remain required
before the prototype worker is considered release-grade.

## Revisit criteria

Replace or supersede this ADR before Playdub bundles/downloads a runtime, publishes an official
model, introduces libmpv, or claims incremental real-time transcription. That decision must record
immutable upstream revisions, exact build flags, linking/distribution obligations, notices, update
mechanism, model licenses, and a three-platform validation result.
