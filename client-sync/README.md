# 同步客户端

交互式 HTTP 客户端，基于招新起始代码继续开发。命令都在本目录执行。

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
cargo run --locked -- --url http://127.0.0.1:7878
```

默认地址是 `http://127.0.0.1:7878`。起始命令是 `ping`、`register`、`login`、`logout`、`list` 和 `q`。

## 代码结构

| 路径 | 职责 |
| --- | --- |
| `src/main.rs` | 解析 `--url` 并运行命令循环 |
| `src/lib.rs` | 请求发送和响应解析 |
| `tests/` | 连接与请求处理测试 |
