<p align="center">English | <a href="README.zh-CN.md">中文</a></p>

<div align="center">
  <img src="crates/wu/resources/app-icon.png" alt="fewu" width="128">
  <h1>fewu</h1>
  <p>The lightweight, native code editor and agent workspace.</p>
  <p>Written in Rust. Built for low-end machines.</p>
</div>

---

fewu is a fast, native code editor written in Rust, with a clean, modern design and coding agents built in. It is tuned for low-spec hardware: a small idle memory footprint, lazy language-server startup, and a native **Grid-Box** spatial file graph.

fewu is maintained by [Matthew](https://github.com/shuriv). It is a fork of [Wu](https://github.com/dstrml/wu), which is itself a fork of [Zed](https://github.com/zed-industries/zed).

## Features

- ⚡ **Native and fast.** No Electron or webviews. Opens instantly and stays responsive on large views.
- 🍃 **Lightweight.** Tuned for low memory use on integrated GPUs and 8 GB machines.
- 👾 **Coding agents included.** Built-in support for Claude Code, Codex, and OpenCode.
- ✨ **Beautiful by default.** Clean, modern look with a delightful UX.
- 🤫 **No account or telemetry.** Nothing to sign in to.

## Download

There are no packaged releases yet. Build from source (see [Building](#building)).

## Building

fewu builds the same way Zed does. See the [Zed development docs](https://zed.dev/docs/development).

## Licensing

fewu is licensed under GPL-3.0-or-later with Apache-2.0 components where marked. See [LICENSE-GPL](./LICENSE-GPL) and [LICENSE-APACHE](./LICENSE-APACHE).

fewu is a derivative work of [Wu](https://github.com/dstrml/wu) and [Zed](https://github.com/zed-industries/zed), and shares their licenses.

## Acknowledgements

Thanks to the Zed team for building an excellent editor and releasing it as open source, and to Wu's author for the fork fewu is built on. fewu would not exist without their work.
