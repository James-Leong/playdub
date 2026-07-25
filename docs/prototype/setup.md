# Prototype setup

Playdub intentionally ships without codecs, inference runtimes, or models. To use AI subtitles,
select tools and models already present on your machine. Do not copy them into this repository.

## 1. Run Playdub

Install Node.js 24+, pnpm 11+, stable Rust, and the
[Tauri prerequisites for your OS](https://v2.tauri.app/start/prerequisites/), then:

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
```

Official desktop compilation belongs in
[`build-desktop.yml`](../../.github/workflows/build-desktop.yml). Local packaging output is
unofficial and must not be committed.

## 2. Provide FFmpeg

Select an FFmpeg executable in Playdub Settings. The prototype invokes it locally to decode the
selected media into a temporary 16 kHz mono WAV. It does not link FFmpeg or copy the binary.

FFmpeg is LGPL-2.1-or-later by default, but optional configuration can enable GPL code, nonfree
components, and additional libraries with separate terms. If you build a conservative LGPL
development binary from upstream source, start with flags such as:

```text
--disable-gpl --disable-nonfree --disable-version3 --enable-shared --disable-static
```

Those flags are not a complete compliance determination: review the full configure output, linked
libraries, enabled codecs, platform packaging, and [FFmpeg's legal page](https://ffmpeg.org/legal.html).
Playdub does not redistribute your binary, and its Apache-2.0 license does not cover FFmpeg or codec
patents.

Before selecting it, run:

```bash
/absolute/path/to/ffmpeg -version
```

Check the reported configuration and confirm that it is the binary you intended. On Windows select
`ffmpeg.exe`.

## 3. Provide whisper.cpp and a Whisper model

Build `whisper-cli` from the
[official whisper.cpp repository](https://github.com/ggml-org/whisper.cpp), following that
revision's build instructions. Select the resulting `whisper-cli` executable and a compatible GGML
Whisper model in Settings.

Model weights are separate from whisper.cpp's MIT-licensed source code. Review the exact model
source, immutable revision, license, tokenizer, file digest, and intended use yourself. A model
appearing in a README or model hub does not make it Apache-2.0 or approved by Playdub.

For an English-only source, an `.en` Whisper model is generally smaller/faster than its multilingual
counterpart. It cannot recognize arbitrary source languages. For multilingual recognition, choose
a compatible multilingual Whisper model and set the source language appropriately.

Whisper's built-in translation task translates speech **to English only**. When English is the
selected target, Playdub runs that local fast path and preserves Whisper timestamps. It is not a
general English-to-any-language translator.

## 4. Optional translation to another target language

For a non-English target, build `llama-cli` from the
[official llama.cpp repository](https://github.com/ggml-org/llama.cpp) and select both the CLI and
an instruction-capable GGUF model in Settings.

Playdub passes a local model path and `--offline`; it never asks llama.cpp to fetch a model. The
selected CLI must support `--offline`, `--single-turn`, and `--reasoning-format none`, so an older
llama.cpp build may be rejected. Confirm those options with `/absolute/path/to/llama-cli --help`.

llama.cpp source is MIT-licensed; the GGUF model is not thereby MIT-licensed. Check its exact weight
license, tokenizer, provenance, acceptable-use terms, supported languages, and redistribution
rights. Never commit the model.

This is an experimental batch text-translation path. “GGUF” describes a file format, not translation
quality or language support. The model must follow translation instructions for the chosen source
and target languages. Output may omit, merge, hallucinate, or reformat captions; review it before
relying on it.

Model files are also parser inputs, not inert trusted assets. llama.cpp's
[security guidance](https://github.com/ggml-org/llama.cpp/security) recommends isolating untrusted
models and lists past parser vulnerabilities. Use a current runtime, verify the model digest, and do
not open a GGUF file from an untrusted publisher on a sensitive machine.

## 5. Use the player

1. Open Settings and select absolute paths for FFmpeg, `whisper-cli`, and the Whisper GGML model.
2. If the target is not English, also select `llama-cli` and a compatible GGUF model.
3. Open a local video. MP4 with H.264 video and AAC audio is the safest prototype input.
4. Choose the source and target languages, then start subtitle generation.
5. Playback remains available while the background batch runs. Keep the application open.
6. Use **Export SRT** or **Export VTT** to write
   `<video>.<target>.playdub.srt`/`.vtt` beside the source. Existing files are not overwritten.
7. Confirm captions against the media before using them for accessibility, publication, or
   consequential decisions.

All processing is local. Playdub has no downloader or network inference code. External tools may
have features of their own; use binaries you trust and audit their configuration independently.
Executable and model fields must resolve to absolute regular-file paths; Playdub does not search
the shell `PATH` or expand aliases.

## Troubleshooting

- **Video is blank but extraction works:** the system WebView likely lacks that codec/container.
  Transcode a test copy to MP4/H.264/AAC outside Playdub.
- **Executable rejected or cannot start:** select the actual executable, not its containing folder,
  and verify it runs from a terminal without a shell alias.
- **Whisper reports an incompatible model:** rebuild the CLI and use a GGML model supported by the
  same whisper.cpp revision.
- **English-only model fails on another language:** choose a multilingual model.
- **Non-English translation is missing:** configure both `llama-cli` and a compatible GGUF model.
- **Translation is poor:** language capability comes from the selected model; Playdub does not
  certify user-supplied models.
- **Export says the file exists:** the prototype never overwrites an existing subtitle. Rename or
  move the previous export before trying again.

See [known limitations](./limitations.md) and run the
[smoke-test checklist](../testing/prototype-smoke-test.md) before reporting a bug.
