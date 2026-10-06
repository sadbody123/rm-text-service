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

默认地址是 `http://127.0.0.1:7878`。可以输入 `ping`、`register`、`login`、`logout`、`list`、`echo` 和 `q`。

`echo` 会逐行读取文本，单独一行 `.` 结束。行与行之间用换行连接，最后一行内容后面不自动加换行。直接输入 `.` 表示空字符串；在结束前多输入一个空行，可以保留末尾换行。单独一行 `..` 表示正文里的一行 `.`。

## 代码结构

| 路径 | 职责 |
| --- | --- |
| `src/main.rs` | 解析 `--url` 并运行命令循环 |
| `src/lib.rs` | 请求发送、响应解析和多行输入 |
| `tests/` | 连接与请求处理测试 |
