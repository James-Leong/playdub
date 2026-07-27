# Prototype limitations

The current vertical slice is designed to prove a local workflow, not broad compatibility.

## Playback

- Playback is an HTML5 video element in the operating system's Tauri WebView.
- A video must be reselected after restart; the native dialog grants only a session-scoped asset
  permission and Playdub deliberately stores no broad static media scope.
- The picker displays `.mp4`, `.mkv`, `.mov`, `.webm`, `.avi`, and `.m4v`, but that filter is not a
  compatibility promise.
- Container, video codec, audio codec, subtitle-track, and hardware-decoding behavior vary by OS.
- MP4/H.264/AAC is the only recommended interchange combination for prototype bug reports.
- FFmpeg extraction support is broader than WebView playback support; one succeeding does not imply
  the other will succeed.
- libmpv, HDR guarantees, DVD/Blu-ray, DRM, network streams, multi-angle video, and complete
  audio/subtitle-track selection are not implemented.

## Recognition

- The prototype extracts and processes a batch; it is not yet streaming ASR or a 30–90 second
  look-ahead scheduler.
- First captions may take significant time on long media or slow hardware.
- Accuracy and language coverage depend on the locally configured Whisper model, audio quality, speech,
  and whisper.cpp build.
- An English-only `.en` model does not support multilingual source audio.
- Speaker identification, overlapping-speech handling, terminology dictionaries, and guaranteed
  word-level timing are not provided.

## Translation

- Whisper's translation mode targets English only.
- Other target languages require optional `llama-cli` and an instruction-capable GGUF model.
- Playdub does not validate that a locally configured GGUF model supports either requested language.
- LLM output can be inaccurate or structurally inconsistent. It is not appropriate for safety-
  critical, legal, medical, or otherwise consequential reliance without expert review.

## Storage and lifecycle

- Temporary decoded audio and generated intermediates use local storage during processing.
- SRT and WebVTT can be exported next to the source video. Existing exports are not overwritten.
- Persistent, versioned subtitle caching is future work.
- Opening another video prevents stale results from replacing the current session, but it does not
  cancel an external process already in progress. Processes have long safety timeouts and may
  continue consuming CPU until completion or timeout.
- Working directories are removed on ordinary success/error returns. A hard application or machine
  termination can leave files in the application cache.
- Model download, checksum verification, update management, and official model recommendations are
  intentionally absent.

## Packaging

- GitHub Actions builds unsigned prototype artifacts for macOS, Windows, and Linux.
- A successful package build does not prove media or inference behavior on that platform.
- External tools and models are not bundled; every installation requires local configuration.
- Signing, macOS notarization, SBOM publication, and release-grade third-party notices remain open.
