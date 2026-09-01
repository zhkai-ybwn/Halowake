# Contributing to Halowake

Thanks for helping improve Halowake. The most useful contributions at this stage are reproducible bug reports, workflow feedback from real multi-project development, and small focused pull requests.

## Before opening an issue

- Search existing issues first.
- For a bug, include the Halowake version, operating system, project package manager, exact steps, expected behavior, and actual behavior.
- Do not include API keys, private repository URLs, internal hosts, or source code you cannot share.

## Local development

Requirements: Node.js 20+, Rust 1.77.2+, Git, and the Tauri 2 system prerequisites.

```bash
git clone https://github.com/zhkai-ybwn/Halowake.git
cd Halowake
npm install
npm run tauri:dev
```

Run these checks before opening a pull request:

```bash
npm run lint
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

## Pull requests

- Keep one pull request focused on one user-visible change or bug fix.
- Add or update targeted tests when behavior changes.
- Do not include generated build artifacts, credentials, or unrelated formatting changes.
- Explain how you tested the change, including any manual desktop verification.

By contributing, you agree that your contributions are licensed under the repository's [MIT License](LICENSE).
