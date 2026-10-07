# RM 文本服务

2026 秋季 DGP 方向的 Rust 实现：一个 HTTP 用户文本服务，包含客户端和两个服务端。

## 已完成

- `client-sync`：同步交互客户端
- `server-sync`：同步服务端
- `server-async`：异步服务端。请求体异步读取，密码哈希放在 `spawn_blocking` 中

## 能力

- 连通性检查和文本回显
- 注册、登录、退出和令牌有效期
- 按用户隔离的文本上传、读取、列表和删除
- 账号注销
- 严格的 JSON 字段校验，以及文本和请求体长度上限
- 共享状态保护，登录写回前核对账号是否仍是原来的那一个

## 检查

每个 crate 都在自己的目录里执行：

```bash
cargo fmt -- --check
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

推送和拉取请求也会跑这些检查。提交必须带已验证的 GPG 签名，说明使用 Conventional Commits。约定见 [CONTRIBUTING.md](CONTRIBUTING.md)。

已用官方参考程序 `reference-v0.2.1` 做过真实交互。

## 结构

```text
rm-text-service/
├── client-sync/
├── server-sync/
├── server-async/
├── .github/
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```
