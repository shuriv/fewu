<p align="center">
  <a href="README.md">English</a> · 中文
</p>

<div align="center">
  <img src="assets/branding/fewu-logo-animated.svg" alt="fewu 标志" width="120" height="120">
  <h1>fewu</h1>
  <p><strong>轻量、原生的代码编辑器与智能体工作区。</strong></p>
  <p>用 Rust 编写，为低配机器打造。</p>
</div>

<p align="center">
  <a href="LICENSE-GPL"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-191919?style=flat-square&amp;labelColor=C1E96D" alt="许可证：GPL-3.0-or-later"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/Rust-native-191919?style=flat-square&amp;labelColor=C1E96D&amp;logo=rust&amp;logoColor=F6F3EC" alt="Rust"></a>
  <a href="#获取-fewu"><img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-191919?style=flat-square&amp;labelColor=C1E96D" alt="平台：macOS、Linux、Windows"></a>
  <a href="https://github.com/zed-industries/zed"><img src="https://img.shields.io/badge/built%20on-Zed-191919?style=flat-square&amp;labelColor=C1E96D" alt="基于 Zed"></a>
</p>

---

**fewu** 是一款用 Rust 编写的快速原生代码编辑器，界面简洁现代，并内置编程智能体。它针对低配硬件做了优化：空闲内存占用小、语言服务器按需启动，并配有原生的 **Grid-Box** 空间文件图。

fewu 由 [Matthew](https://github.com/shuriv) 维护。它是 [Wu](https://github.com/dstrml/wu) 的分支，而 Wu 又是 [Zed](https://github.com/zed-industries/zed) 的分支。

## 为什么选择 fewu？

- ⚡ **原生、快速。** 基于 Zed 的 GPU 加速 UI 框架 GPUI，不用 Electron，也没有网页视图。打开即用，大项目下依然流畅。
- 🍃 **轻量。** 针对集成显卡和 8 GB 内存机器调优，空闲占用小，语言服务器按需启动。
- 👾 **内置编程智能体。** 在编辑器里直接与 Claude Code、Codex 和 OpenCode 对话，使用你已经安装并登录的命令行工具。
- 🧭 **Grid-Box。** 原生的空间文件图，把文件和它们的关系铺在画布上。可以在结构化网格和力导向图之间切换。
- 🧩 **兼容 Zed 扩展。** 从 Zed 扩展商店安装扩展，无需修改即可在 fewu 中运行。
- 🤫 **默认注重隐私。** 无需账号，没有遥测，没有崩溃报告。

## 为低配机器而生

fewu 保持编辑器内核原生，并尽量降低资源占用：

| 方面 | 做法 |
| --- | --- |
| **空闲内存** | 打开项目后依然保持轻量。 |
| **语言服务器** | 只有真正用到某种语言时才启动。 |
| **渲染** | GPUI 直接用 GPU 绘制界面，不需要浏览器引擎。 |
| **导航** | Grid-Box 在大文件树上依然流畅。 |

内存测试方法和数据见[上游 Wu 的测试](docs/memory-benchmark.md)。

## 内置编程智能体

Agent Chat 让你在代码旁边运行 Claude Code、Codex 和 OpenCode。它使用你已经安装并登录的命令行工具，不需要 API key，也不需要 fewu 账号。

- 从活动栏的智能体图标开始对话。
- 输入 `/` 查看命令，`@` 提及文件，`$` 使用技能。
- 粘贴或拖入图片即可作为附件。
- 对话可以固定、分叉和重命名；子智能体会在独立标签页中打开。

## 获取 fewu

目前还没有打包好的发行版。请从源码构建：

```sh
git clone https://github.com/shuriv/fewu.git
cd fewu
cargo run -p wu
```

fewu 使用 [`rust-toolchain.toml`](rust-toolchain.toml) 中固定的 Rust 1.97.1。请先按照 [Zed 开发文档](https://zed.dev/docs/development)安装平台依赖。

> 主 crate 目前仍叫 `wu`，所以构建命令里用的是这个名字。

<details>
<summary><strong>构建 release 版本</strong></summary>

```sh
cargo build --release -p wu
```

生成的可执行文件位于 `target/release/wu`（Windows 上是 `wu.exe`）。
</details>

## 文档

- [Zed 文档](https://zed.dev/docs) — 设置、快捷键、语言、扩展、任务、调试和远程开发的使用方式都相同。
- [内存测试](docs/memory-benchmark.md) — 上游编辑器与 Zed、VS Code 的对比。
- [问题反馈](https://github.com/shuriv/fewu/issues) — 报告 bug 或提出功能建议。

## 参与贡献

fewu 还处于早期阶段。欢迎在 [Issues](https://github.com/shuriv/fewu/issues) 中报告 bug、提出功能建议，也欢迎提交 pull request。

## 许可证

fewu 采用 **GPL-3.0-or-later** 许可证，标注的部分采用 **Apache-2.0**。请看 [LICENSE-GPL](LICENSE-GPL) 和 [LICENSE-APACHE](LICENSE-APACHE)。

fewu 是 [Wu](https://github.com/dstrml/wu) 和 [Zed](https://github.com/zed-industries/zed) 的衍生作品，使用相同的许可证。

## 致谢

感谢 Zed 团队打造了这样出色的编辑器，并把它开源；也感谢 Wu 的作者，fewu 建立在他们的工作之上。没有他们，就不会有 fewu。

<div align="center">
  <sub>用 Rust 构建 · 源自 <a href="https://github.com/dstrml/wu">Wu</a> · 基于 <a href="https://github.com/zed-industries/zed">Zed</a></sub>
</div>
