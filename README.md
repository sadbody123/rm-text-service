# RM 文本服务

Rust 实现的 HTTP 用户文本服务。业务代码会按任务逐步加入这个仓库。

## 检查

推送到 `main` 或打开拉取请求时会运行：

- 发现仓库里的 Cargo 工程后，执行格式检查、`cargo check`、Clippy、测试和 release 构建。还没有 crate 时，这一项通过，并记录“尚无 crate”。
- 每个提交都要有 GitHub 已验证的 GPG 签名，提交说明符合 Conventional Commits。
- 拉取请求要按模板写全背景、改动、影响、验证和材料，并且不超过 30 个提交。

提交约定见 [CONTRIBUTING.md](CONTRIBUTING.md)。
