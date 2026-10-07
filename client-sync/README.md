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

默认地址是 `http://127.0.0.1:7878`。可以输入 `ping`、`register`、`login`、`logout`、`list`、`echo`、`put`、`get`、`delete`、`delete-user` 和 `q`。

`delete-user` 在注销成功后清除本地令牌。收到 401 时会提示重新登录并清除令牌。

`list` 按服务端返回的顺序显示文本名称。`get` 或 `delete` 在文本不存在时显示 404。

`put` 和 `get` 会先询问文本名称。`put` 再按与 `echo` 相同的方式读取文本，并在已登录时带上令牌。

`echo` 会逐行读取文本，单独一行 `.` 结束。行与行之间用换行连接，最后一行内容后面不自动加换行。直接输入 `.` 表示空字符串；在结束前多输入一个空行，可以保留末尾换行。以 `..` 开头的行会去掉一个点再保存，所以 `..` 表示 `.`，`...` 表示 `..`。

## 代码结构

| 路径 | 职责 |
| --- | --- |
| `src/main.rs` | 解析 `--url` 并运行命令循环 |
| `src/lib.rs` | 请求发送、响应解析和多行输入 |
| `tests/` | 连接与请求处理测试 |
