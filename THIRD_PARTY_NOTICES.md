# Third-party notices

Playdub-authored source is Apache-2.0. The license in this repository does not change or replace
the licenses of third-party software or model files.

## Prototype external tools

The prototype can launch tools selected by the user. They are not included in this repository or
in the current GitHub Actions desktop artifacts:

| Component | Upstream software license | Prototype use | Important distinction |
| --- | --- | --- | --- |
| FFmpeg | LGPL-2.1-or-later by default; configuration can enable GPL or nonfree components | Decode media to a temporary WAV | Exact obligations depend on the binary's build configuration and linked libraries |
| whisper.cpp | MIT | Run local Whisper inference | Whisper model weights and tokenizers must be reviewed separately |
| llama.cpp | MIT | Optional local transcript translation | Every GGUF model has its own license, provenance, and restrictions |

Links to upstream licensing information:

- [FFmpeg legal and license information](https://ffmpeg.org/legal.html)
- [FFmpeg source license](https://git.ffmpeg.org/gitweb/ffmpeg.git/blob/HEAD:/LICENSE.md)
- [whisper.cpp repository and MIT license](https://github.com/ggml-org/whisper.cpp)
- [llama.cpp repository and MIT license](https://github.com/ggml-org/llama.cpp)

These statements describe the upstream projects at a high level and are not legal advice. Users are
responsible for the particular binaries and model files they select. A system-installed tool can
contain different features and licensing obligations from an upstream default build.

## Model files

No model weight is covered by Playdub's Apache-2.0 license merely because Playdub can open it. The
prototype model selectors accept local files and do not imply that a model is safe to redistribute,
use commercially, or use for a particular purpose. Check the exact model revision, weight license,
tokenizer license, model card, source, and any acceptable-use terms.

Playdub currently distributes no media engine, codec bundle, inference runtime, model weight, or
tokenizer. Package lock files alone are not a substitute for release notices.

The prototype does include its locked Tauri, Rust, React, and JavaScript application dependencies.
The newly admitted native dialog pair is recorded in the
[prototype dependency review](./docs/legal/prototype-dependency-review.md). A complete generated
dependency notice and SBOM remain a release gate.

Before a release, maintainers must generate and review:

- Rust and JavaScript dependency licenses;
- native dynamic libraries and their build configuration;
- codec configuration and patent considerations;
- fonts, icons, and media assets;
- inference runtime licenses; and
- every downloadable model and tokenizer license.

See [the dependency and model policy](./docs/legal/dependency-and-model-policy.md).
