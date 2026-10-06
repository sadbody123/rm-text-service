# 贡献约定

## 提交

每个提交都用你自己的 GPG 密钥签名。GitHub 必须显示为已验证的 OpenPGP 签名；SSH 签名不能通过检查。

提交说明使用 Conventional Commits，标题一行，例如：

```text
ci: 建立提交门禁与 Rust 检查
feat(client): 实现文本上传
```

类型只用这些小写名字：`feat`、`fix`、`docs`、`style`、`refactor`、`perf`、`test`、`build`、`ci`、`chore`、`revert`。`scope` 可以省略。说明用中文，不超过 100 个字符，结尾不加句号。

## 拉取请求

描述里保留模板的五个小节，并且每节都要有实质内容。只留下 HTML 注释视为没填；确实没有影响或材料时写“无”。一个拉取请求最多包含 30 个提交。

`main` 不接受直接推送，只通过 squash merge 合并。
