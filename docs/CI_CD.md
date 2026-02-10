# CI/CD 指南

## 本地构建前置

1. 安装 `rustup`（Rust 官方工具链管理器）。
2. 进入仓库根目录后，`rustup` 会按 `rust-toolchain.toml` 自动拉取并切换到指定 Rust 版本（当前为 `1.88.0`）。

## 常用检查命令

```bash
# 检查 rustup 是否可用
rustup --version

# 在仓库根目录执行，若本机缺少指定版本会自动下载安装
cargo check
```

## GitHub Actions 约定

- CI 与 Release workflow 均通过 `actions-rust-lang/setup-rust-toolchain@v1` 安装 Rust。
- 未显式指定 `toolchain` 参数时，action 会读取仓库根目录 `rust-toolchain.toml`，与本地保持一致。
