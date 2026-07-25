# Prototype smoke-test checklist

Use this checklist after automated tests. Record OS, architecture, Playdub commit, tool versions,
model identifiers, and media fixture provenance. Never attach private media or model files to a
public issue.

## Test input

Use a short, clearly licensed or locally generated MP4 containing H.264 video, AAC audio, visible
motion, silence, and several spoken sentences. For translation testing, use speech whose expected
meaning you can independently verify.

## Setup

- [ ] `pnpm lint`, `pnpm test`, `pnpm build`, Rust formatting, Clippy, and Rust tests pass.
- [ ] FFmpeg, whisper.cpp CLI, and model paths are selected with absolute paths.
- [ ] Each tool runs directly in a terminal and its exact revision/configuration is recorded.
- [ ] Model source, revision, weight/tokenizer license, and digest are recorded locally.
- [ ] Optional llama.cpp CLI and GGUF model metadata are recorded when testing non-English targets.

## Playback

- [ ] The file dialog opens and cancellation changes no state.
- [ ] A compatible MP4 opens without exposing an unrestricted filesystem scope to the WebView.
- [ ] Play, pause, seek, volume, mute, end-of-media, and fullscreen behave as labelled.
- [ ] Seeking updates the visible time and captions follow the media clock.
- [ ] An unsupported/corrupt file produces an actionable error without crashing.

## Recognition and English translation

- [ ] Starting generation does not pause or block playback controls.
- [ ] FFmpeg failure reports an error and does not leave a caption job marked successful.
- [ ] A missing/incompatible Whisper model reports an actionable local error.
- [ ] Source captions appear at approximately the spoken media timestamps.
- [ ] Selecting English target uses Whisper translation and produces English captions.
- [ ] Starting a new file/job prevents late captions from the old generation appearing.
- [ ] A completed or failed job removes its working directory on an ordinary return.
- [ ] Record that opening a new file does not yet cancel an old native process; this is a known
      prototype limitation, not a successful cancellation test.

## Optional arbitrary-language translation

- [ ] Missing llama.cpp configuration is explained before a non-English translation starts.
- [ ] The configured model receives only local transcript text.
- [ ] Caption count/order/timestamps remain usable after translation.
- [ ] A malformed or partial model response is handled without script/HTML injection or crash.
- [ ] A bilingual reviewer checks meaning; model output is not assumed correct.
- [ ] SRT and WebVTT export next to the video, and neither silently overwrites an existing file.

## Privacy and packaging

- [ ] No media, audio, transcript, prompt, model path, or model input appears in network traffic.
- [ ] Logs avoid media content and expose no more local-path detail than needed for diagnosis.
- [ ] The repository and produced package contain no FFmpeg, whisper.cpp, llama.cpp, model, generated
      subtitle, temporary WAV, signing material, or private test media.
- [ ] GitHub artifact is labelled unsigned prototype software and is not presented as a release.

## Bug report evidence

Include reproducible steps, sanitized logs, platform/tool versions, and whether the same licensed
fixture works outside Playdub. Do not upload the user's video, transcript, absolute home-directory
paths, or model unless they explicitly choose and have rights to share it.
