# 原型配置指南

Playdub 不捆绑编解码器、推理运行时或模型。要使用 AI 字幕，请选择已经存在于本机且来源可信的
工具和模型；不要把它们复制进仓库。

## 1. 运行 Playdub

安装 Node.js 24+、pnpm 11+、稳定版 Rust 和
[当前系统对应的 Tauri 依赖](https://v2.tauri.app/start/prerequisites/)，然后执行：

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
```

正式桌面编译应使用
[`build-desktop.yml`](../../.github/workflows/build-desktop.yml)。本地打包产物不是正式发行包，
不得提交到仓库。

## 2. 自行提供 FFmpeg

在 Playdub 设置中选择 FFmpeg 可执行文件。原型只在本机调用它，把媒体解码为临时的
16 kHz 单声道 WAV；Playdub 不链接或复制该二进制。

FFmpeg 默认是 LGPL-2.1-or-later，但可选配置可能启用 GPL、nonfree 组件或使用其他许可证的
库。如果从上游源码构建一个保守的 LGPL 开发用二进制，可以从以下参数开始：

```text
--disable-gpl --disable-nonfree --disable-version3 --enable-shared --disable-static
```

这些参数不能替代完整的合规判断。仍需检查全部 configure 输出、链接库、启用的编解码器和平台
打包方式，并阅读 [FFmpeg 法律说明](https://ffmpeg.org/legal.html)。Playdub 不分发你的
FFmpeg，其 Apache-2.0 许可证也不涵盖 FFmpeg 或编解码专利。

选择前先执行：

```bash
/绝对路径/ffmpeg -version
```

检查输出的配置是否符合预期。Windows 应选择 `ffmpeg.exe`。

## 3. 自行提供 whisper.cpp 和 Whisper 模型

按照[官方 whisper.cpp 仓库](https://github.com/ggml-org/whisper.cpp)对应版本的说明构建
`whisper-cli`，然后在设置中选择该可执行文件和兼容的 GGML Whisper 模型。

模型权重与 whisper.cpp 的 MIT 源代码许可证是两回事。请自行核查模型的准确来源、不可变版本、
权重许可证、tokenizer、文件摘要和适用范围。模型出现在 README 或模型网站上，不代表它是
Apache-2.0 或已经获得 Playdub 审核。

如果音频来源只有英文，`.en` Whisper 模型通常比多语言版本更小、更快，但不能识别任意源语言。
多语言识别必须选择兼容的多语言模型，并正确设置源语言。

Whisper 内置的翻译任务只把语音翻译成**英文**。目标语言为英文时，Playdub 使用这个本地快速
路径并保留 Whisper 时间戳；它不是任意语言之间的通用翻译器。

## 4. 可选：翻译为其他语言

当目标语言不是英文时，可从[官方 llama.cpp 仓库](https://github.com/ggml-org/llama.cpp)
构建 `llama-cli`，并在设置中同时选择该 CLI 和具有对应语言能力的指令型 GGUF 模型。

Playdub 只传入本地模型路径并启用 `--offline`，不会要求 llama.cpp 在线拉取模型。所选 CLI
必须支持 `--offline`、`--single-turn` 和 `--reasoning-format none`；较老的 llama.cpp
可能不兼容。可执行 `/绝对路径/llama-cli --help` 核对这些参数。

llama.cpp 源码使用 MIT 许可证，并不代表 GGUF 模型也使用 MIT。请核查模型权重许可证、
tokenizer、来源、可接受使用条款、支持语言和再分发权利；不要把模型提交到仓库。

这是一条实验性的批量文本翻译路径。“GGUF”只是文件格式，不代表翻译质量或语言覆盖。
模型必须能遵循源语言到目标语言的翻译指令；输出可能遗漏、合并、虚构或重新排版字幕，使用前
必须核对。

模型文件也是解析器输入，不应当作天然安全的静态资源。llama.cpp 的
[安全指南](https://github.com/ggml-org/llama.cpp/security)建议隔离不可信模型，并列出了历史
解析器漏洞。请使用最新安全版本、核对模型摘要，不要在存有敏感数据的机器上打开来源不可信的
GGUF。

## 5. 使用播放器

1. 在设置中选择 FFmpeg、`whisper-cli` 和 Whisper GGML 模型的绝对路径。
2. 非英文目标语言还需选择 `llama-cli` 和兼容的 GGUF 模型。
3. 打开本地视频；MP4（H.264 + AAC）是最稳妥的原型输入。
4. 选择源语言和目标语言，开始生成字幕。
5. 后台批处理期间仍可播放视频，但需要保持应用运行。
6. 点击 **Export SRT** 或 **Export VTT**，在源视频旁生成
   `<视频名>.<目标语言>.playdub.srt`/`.vtt`；已有文件不会被覆盖。
7. 将字幕用于无障碍、发布或重要决策前，必须与原视频核对。

所有处理都在本机完成。Playdub 没有模型下载器或在线推理代码。外部工具自身可能包含网络等
功能，请只使用可信二进制，并独立审查其配置。
可执行文件与模型字段必须是可解析的绝对普通文件路径；Playdub 不搜索 shell `PATH`，也不展开
命令别名。

常见问题和原型限制见[英文限制说明](./limitations.md)及
[冒烟测试清单](../testing/prototype-smoke-test.md)。
