# Playdub

Playdub is an open-source, local-first video player that generates translated subtitles with
on-device speech recognition and machine translation.

> [!IMPORTANT]
> Playdub is an early, usable prototype—not a production media player. It can play compatible local
> files, transcribe them through a user-supplied whisper.cpp CLI, and translate subtitles on the
> local machine. Formats, accuracy, performance, and packaging still need cross-platform testing.

[简体中文](./README.zh-CN.md)

## Prototype capabilities

- Open a local video and play, pause, seek, change volume, and enter fullscreen.
- Extract a 16 kHz mono working-audio file with a user-supplied FFmpeg executable.
- Generate timestamped subtitles with a user-supplied `whisper-cli` executable and GGML model.
- Translate speech to English with Whisper's translation mode.
- Optionally translate transcript text to another language with a user-supplied `llama-cli` and
  instruction-capable GGUF model.
- Render synchronized bilingual captions and export SRT or WebVTT next to the source video.
- Keep video, audio, transcript, prompts, and models on the device. Playdub performs no download and
  has no online inference path.

The video element uses the operating system's Tauri WebView. MP4 with H.264 video and AAC audio is
the safest interchange format; actual codec/container support varies by OS. A file that FFmpeg can
read is not necessarily playable by the WebView. See [prototype setup](./docs/prototype/setup.md)
and [known limitations](./docs/prototype/limitations.md).

## Principles

- **Local first:** media and speech stay on the user's device.
- **Playback never waits for AI:** transcription and translation run independently of playback.
- **Auditable distribution:** desktop test packages are produced by GitHub Actions.
- **License-safe models:** model weights are optional downloads and require an approved registry
  entry. They are not covered by Playdub's Apache-2.0 license.
- **Accessible by default:** keyboard use, readable captions, reduced motion, and screen-reader
  semantics are product requirements.

## Prototype architecture

| Layer | Responsibility |
| --- | --- |
| React + TypeScript | Product UI and accessible interaction |
| Tauri + Rust | App lifecycle, orchestration, IPC, cache, and security |
| HTML5 video in the Tauri WebView | Prototype playback and canonical media clock |
| User-supplied FFmpeg | Decode a private temporary 16 kHz mono WAV file |
| User-supplied whisper.cpp CLI | Local speech recognition and English translation |
| Optional user-supplied llama.cpp CLI | Experimental transcript translation to other languages |

Read [the architecture overview](./docs/architecture/overview.md) before implementing a new
subsystem.

## Run the prototype

Prerequisites:

- Node.js 24 or newer
- pnpm 11 or newer
- stable Rust with `rustfmt` and `clippy`
- platform prerequisites from the
  [Tauri documentation](https://v2.tauri.app/start/prerequisites/)
- local FFmpeg, whisper.cpp CLI, and Whisper GGML model files that you obtained under terms
  acceptable to you
- optionally, llama.cpp CLI and a compatible GGUF translation/instruction model

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
```

In Playdub Settings, select the executable and model paths. Playdub does not download, bundle, or
license those files for you. Follow the complete [prototype setup guide](./docs/prototype/setup.md)
before opening an issue.

## Development

```bash
pnpm test
pnpm lint
cargo test --manifest-path src-tauri/Cargo.toml
```

Local development is for code, formatting, linting, and tests. Desktop application bundles are
built exclusively by [GitHub Actions](./.github/workflows/build-desktop.yml); current artifacts are
unsigned prototype builds, not releases. Do not commit locally generated installers. See
[GitHub desktop builds](./docs/development/github-builds.md) for the matrix and release gates.

## Contributing

Start with [CONTRIBUTING.md](./CONTRIBUTING.md). AI-assisted contributions must also follow
[AGENTS.md](./AGENTS.md) and the
[AI contributor guide](./docs/development/ai-contributor-guide.md).

Participation is governed by the [Code of Conduct](./CODE_OF_CONDUCT.md).
Security reports should follow [SECURITY.md](./SECURITY.md), not a public issue.

## License

Playdub-authored source code is licensed under [Apache License 2.0](./LICENSE). This does **not**
license FFmpeg, whisper.cpp, llama.cpp, any codec, or any model file.

Media codecs, native libraries, fonts, datasets, and model weights have independent licenses.
See the [dependency and model policy](./docs/legal/dependency-and-model-policy.md) and
[THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md).
