<p align="center">
  English · <a href="README.zh-CN.md">中文</a>
</p>

<div align="center">
  <img src="assets/branding/fewu-logo-animated.svg" alt="fewu logo" width="120" height="120">
  <h1>fewu</h1>
  <p><strong>The lightweight, native code editor and agent workspace.</strong></p>
  <p>Written in Rust. Built for low-end machines.</p>
</div>

<p align="center">
  <a href="LICENSE-GPL"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-191919?style=flat-square&amp;labelColor=C1E96D" alt="License: GPL-3.0-or-later"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/Rust-native-191919?style=flat-square&amp;labelColor=C1E96D&amp;logo=rust&amp;logoColor=F6F3EC" alt="Rust"></a>
  <a href="#get-fewu"><img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-191919?style=flat-square&amp;labelColor=C1E96D" alt="Platforms: macOS, Linux, Windows"></a>
  <a href="https://github.com/zed-industries/zed"><img src="https://img.shields.io/badge/built%20on-Zed-191919?style=flat-square&amp;labelColor=C1E96D" alt="Built on Zed"></a>
</p>

---

**fewu** is a fast, native code editor written in Rust, with a clean, modern design and coding agents built in. It is tuned for low-spec hardware, with a small idle memory footprint, lazy language-server startup, and a native **Grid-Box** spatial file graph.

fewu is maintained by [Matthew](https://github.com/shuriv). It is a fork of [Wu](https://github.com/dstrml/wu), which is itself a fork of [Zed](https://github.com/zed-industries/zed).

## Why fewu?

- ⚡ **Native and fast.** Built on GPUI, Zed's GPU-accelerated UI framework. No Electron, no webviews. Opens instantly and stays responsive on large projects.
- 🍃 **Lightweight.** Tuned for integrated GPUs and 8 GB machines, with a small idle memory footprint and lazy language-server startup.
- 👾 **Coding agents included.** Chat with Claude Code, Codex, and OpenCode inside the editor, using the CLIs you already have installed and signed in.
- 🧭 **Grid-Box.** A native spatial file graph that lays files and their relationships out on a canvas. Switch between a structured grid and a force graph.
- 🧩 **Zed extensions work.** Install extensions from the Zed registry; they run in fewu without changes.
- 🤫 **Private by default.** No account, no telemetry, no crash reporting.

## Built for low-end machines

fewu keeps the editor core native and the resource use low:

| What | How |
| --- | --- |
| **Idle memory** | A small footprint, even with a project open. |
| **Language servers** | Start lazily, only when a language is actually used. |
| **Rendering** | GPUI draws the interface on the GPU without a browser engine. |
| **Navigation** | Grid-Box stays fast on large file trees. |

See the [upstream memory benchmark](docs/memory-benchmark.md) for the methodology and numbers.

## Coding agents, built in

Agent Chat runs Claude Code, Codex, and OpenCode next to your code. It uses the command-line tools you already have installed and signed in, so there are no API keys and no fewu account.

- Start a chat from the agent icon in the activity bar.
- Type `/` for commands, `@` to mention a file, and `$` for a skill.
- Paste or drop images to attach them.
- Pin, fork, and rename chats; subagents open in their own tabs.

## Get fewu

Grab the Windows x86_64 build from the [latest release](https://github.com/shuriv/fewu/releases/latest), or build from source:

```sh
git clone https://github.com/shuriv/fewu.git
cd fewu
cargo run -p wu
```

fewu uses Rust 1.97.1, pinned in [`rust-toolchain.toml`](rust-toolchain.toml). Install the platform dependencies from the [Zed development docs](https://zed.dev/docs/development) first.

> The main crate is still named `wu`, so the build commands use that name.

<details>
<summary><strong>Build a release binary</strong></summary>

```sh
cargo build --release -p wu
```

The binary is written to `target/release/wu` (`wu.exe` on Windows).
</details>

## Documentation

- [Zed documentation](https://zed.dev/docs) — settings, key bindings, languages, extensions, tasks, debugging, and remote development all work the same way.
- [Memory benchmark](docs/memory-benchmark.md) — how the upstream editor compares with Zed and VS Code.
- [Issues](https://github.com/shuriv/fewu/issues) — report a bug or request a feature.

## Contributing

fewu is an early-stage project. Bug reports and feature requests are welcome in [Issues](https://github.com/shuriv/fewu/issues); pull requests are welcome too.

## License

fewu is licensed under **GPL-3.0-or-later**, with **Apache-2.0** components where marked. See [LICENSE-GPL](LICENSE-GPL) and [LICENSE-APACHE](LICENSE-APACHE).

fewu is a derivative work of [Wu](https://github.com/dstrml/wu) and [Zed](https://github.com/zed-industries/zed), and shares their licenses.

## Acknowledgements

Thanks to the Zed team for building an excellent editor and releasing it as open source, and to Wu's author for the fork fewu is built on. fewu would not exist without their work.

<div align="center">
  <sub>Built with Rust · Forked from <a href="https://github.com/dstrml/wu">Wu</a> · Built on <a href="https://github.com/zed-industries/zed">Zed</a></sub>
</div>
