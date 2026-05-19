# vox — 多模型 AI 多媒体命令行工具

基于 Rust 的统一 AI 接口 CLI，支持阶跃星辰（StepFun）和 MiniMax 两大提供商，覆盖文本、图像、语音、视频、音乐、搜索、视觉理解七项能力。

## 特性

- **多提供商**：StepFun 与 MiniMax，共享 OpenAI 兼容基础适配器
- **七项能力**：文本对话/补全、图像生成、语音合成、视频生成、音乐生成、网络搜索、视觉理解
- **交互式 REPL**：`vox text repl` 支持多轮对话与上下文记忆
- **提供商管理**：`vox provider add/remove/list/status`
- **模型管理**：`vox models list/set` 按能力、按提供商配置
- **诊断工具**：`vox doctor` 检查配置、网络、认证状态
- **自动重试**：瞬态故障指数退避重试（3 次）
- **配置迁移**：自动升级旧模型名称和 API 地址
- **JSON 输出**：`--format json` 便于脚本集成
- **Shell 补全**：`vox completion bash|zsh|fish|elvish`

## 安装

```bash
git clone https://github.com/huangcheng/vox.git
cd vox
cargo build --release
# 二进制文件位于 target/release/vox
```

## 快速开始

```bash
# 配置 API Key
vox provider add stepfun YOUR_API_KEY
vox provider add minimax YOUR_API_KEY

# 对话
vox text chat --message "解释 Rust 所有权机制"
vox --provider minimax text chat --message "你好"

# 生成图像
vox image generate "太空中的猫" --output cat.png

# 语音合成
vox speech generate --text "你好世界" --voice cixingnansheng --out hello.mp3

# 网络搜索
vox search query "Rust 编程语言"

# 视觉理解
vox vision analyze photo.jpg --prompt "描述这张图片"

# 启动 TUI 模式
vox --tui
```

## 配置

配置文件路径：
- macOS/Linux：`~/.config/vox/config.toml`
- Windows：`%APPDATA%\vox\config.toml`

示例（`config.example.toml`）：

```toml
provider = "stepfun"

[stepfun]
api_key = "sk-your-api-key-here"

[minimax]
api_key = "your-minimax-api-key-here"
```

### 提供商信息

| 提供商 | API 地址 | 对话模型 | 语音模型 |
|--------|----------|----------|----------|
| StepFun（阶跃星辰） | `https://api.stepfun.com/v1` | step-1-8k | stepaudio-2.5-tts |
| MiniMax | `https://api.minimaxi.com/v1` | MiniMax-M2.7 | speech-2.8-hd |

### 模型覆盖

```toml
[minimax]
api_key = "..."
model = "MiniMax-M2.7-highspeed"  # 覆盖默认对话模型
```

或按能力设置：

```bash
vox models set speech speech-2.8-hd
vox models list --provider stepfun
```

## 命令参考

```
vox [OPTIONS] [COMMAND]

命令：
  text        文本生成与对话
  image       图像生成
  speech      语音合成（TTS）
  video       视频生成
  music       音乐生成
  search      网络搜索
  vision      图像理解（视觉）
  doctor      运行诊断
  provider    管理提供商
  models      管理模型
  config      管理配置
  completion  生成 Shell 补全脚本

选项：
  --provider <PROVIDER>      提供商（minimax, stepfun）
  --model <MODEL>            模型名称覆盖
  --format <FORMAT>          输出格式（text, json）[默认：text]
  --output-dir <DIR>         默认输出目录
  --config <PATH>            配置文件路径
  --quiet                    静默模式
  --verbose                  调试输出
```

## 架构

```
src/
  providers/
    mod.rs       AIProvider trait、RetryProvider、工厂方法
    openai.rs    共享 OpenAI 兼容 HTTP 客户端
    stepfun.rs   StepFun 适配器（~200 行）
    minimax.rs   MiniMax 适配器（~230 行）
  config.rs      配置管理、迁移、提供商/模型管理
  cli.rs         Clap CLI 定义
  app.rs         命令分发
  capabilities.rs  按提供商的能力标志
  models.rs      静态模型注册表
```

`AIProvider` trait 定义各项能力（`chat`、`image_generate`、`speech_synthesize` 等）。共享的 `OpenAIClient` 提供 OpenAI 兼容端点的默认实现——各提供商只需覆写独有的 API。

## 许可证

MIT
