# Playdub

Playdub 是一个开源、本地优先的视频播放器，使用端侧语音识别与机器翻译生成与播放同步的双语字幕。

> [!IMPORTANT]
> Playdub 目前是可以运行的早期原型，不是成熟的视频播放器。它可以播放系统 WebView
> 兼容的本地视频，调用用户自行提供的 whisper.cpp CLI 识别语音，并在本机翻译字幕。
> 格式兼容性、识别准确率、性能与安装包仍需跨平台验证。

## 原型能力

- 打开本地视频，播放、暂停、跳转、调节音量和全屏；
- 使用用户自行提供的 FFmpeg，把音频抽取为 16 kHz 单声道临时 WAV；
- 使用用户自行提供的 `whisper-cli` 和 GGML 模型生成带时间戳字幕；
- 使用 Whisper 的翻译模式将语音翻译为英文；
- 可选使用用户自行提供的 `llama-cli` 和指令型 GGUF 模型，将识别文本实验性地翻译为其他语言；
- 同步显示双语字幕，并在源视频旁导出 SRT 或 WebVTT；
- 视频、音频、转录、提示词和模型均保留在本机；Playdub 不下载模型，也没有在线推理路径。

播放使用 Tauri 系统 WebView 中的 HTML5 video。MP4（H.264 视频 + AAC 音频）是最稳妥的
交换格式，实际支持取决于操作系统。FFmpeg 能读取一个文件，并不表示系统 WebView 也能播放。
请阅读[原型配置指南](./docs/prototype/setup.zh-CN.md)和[已知限制](./docs/prototype/limitations.md)。

## 核心原则

- **本地优先：** 视频和音频默认不离开用户设备。
- **播放不等待 AI：** 播放、识别和翻译互相解耦。
- **构建可审计：** 桌面测试安装包统一由 GitHub Actions 生成。
- **模型授权清晰：** 模型是可选下载项，必须登记并审核许可证，不适用 Playdub 的
  Apache-2.0 许可证。
- **无障碍默认开启：** 键盘操作、字幕可读性、减少动态效果和语义化界面属于产品要求。

## 运行原型

除 Node.js 24+、pnpm 11+、Rust 和 Tauri 系统依赖外，还需自行准备 FFmpeg、
whisper.cpp CLI 和 Whisper GGML 模型；翻译为非英文时，可选准备 llama.cpp CLI 与兼容的
GGUF 翻译/指令模型。

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
```

在 Playdub 设置中分别选择可执行文件与模型路径。Playdub 不替你下载、捆绑或授予这些文件的
许可证。完整步骤见[原型配置指南](./docs/prototype/setup.zh-CN.md)。

## 本地开发

```bash
pnpm test
pnpm lint
cargo test --manifest-path src-tauri/Cargo.toml
```

本地只进行代码开发、格式检查、静态检查和单元测试。macOS、Windows、Linux 桌面打包统一由
[GitHub Actions](./.github/workflows/build-desktop.yml) 完成。当前工作流产物是未签名
的原型测试包，不是正式发行版；不要提交本地构建产物。构建矩阵和发行门槛见
[GitHub 桌面构建说明](./docs/development/github-builds.md)。

开始贡献前请阅读：

- [贡献指南](./CONTRIBUTING.md)
- [AI 开发约束](./AGENTS.md)
- [架构说明](./docs/architecture/overview.md)
- [依赖与模型授权策略](./docs/legal/dependency-and-model-policy.md)

Playdub 原创源代码使用 [Apache-2.0](./LICENSE) 许可证。该许可证**不适用于** FFmpeg、
whisper.cpp、llama.cpp、编解码器或任何模型文件；详见
[第三方声明](./THIRD_PARTY_NOTICES.md)。
