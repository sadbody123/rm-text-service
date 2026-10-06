# 同步服务端

同步 HTTP 服务端，基于招新起始代码继续开发。命令都在本目录执行。

## 检查

```bash
cargo fmt -- --check
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

## 运行

```bash
cargo run --locked -- --address 127.0.0.1:7878
```

默认地址是 `127.0.0.1:7878`。接口包括 `GET /ping`、`POST /echo`、`POST /users`、`POST /sessions`、`DELETE /sessions/current` 和 `GET /texts`。

## 代码结构

| 路径 | 职责 |
| --- | --- |
| `src/main.rs` | 命令行参数和启动入口 |
| `src/lib.rs` | 路由、业务规则、用户状态和文本状态 |
| `src/http.rs` | 同步应用组装和 HTTP 测试接口 |
| `src/infrastructure.rs` | Rocket 适配、请求体读取和日志 |
| `tests/` | 业务测试和 HTTP 测试 |
