#!/usr/bin/env python3
"""按 PR 模板检查描述是否写全必要小节。"""

import os
import re
import sys
from pathlib import Path

HEADING = re.compile(r"^\s{0,3}##\s+(.+?)(?:\s+#+)?\s*$")
FENCE = re.compile(r"^\s{0,3}(`{3,}|~{3,})")
COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)


def strip_comments(text: str) -> str:
    return COMMENT.sub("", text)


def extract_sections(markdown: str) -> dict[str, str]:
    sections: dict[str, list[str]] = {}
    current: str | None = None
    in_code = False
    fence_char = ""
    fence_len = 0

    for line in markdown.splitlines():
        fence_match = FENCE.match(line)
        if fence_match:
            marker = fence_match.group(1)
            if not in_code:
                in_code = True
                fence_char = marker[0]
                fence_len = len(marker)
            elif marker[0] == fence_char and len(marker) >= fence_len:
                in_code = False
                fence_char = ""
                fence_len = 0
            if current is not None:
                sections[current].append(line)
            continue

        if not in_code:
            match = HEADING.match(line)
            if match:
                current = match.group(1).strip()
                sections.setdefault(current, [])
                continue

        if current is not None:
            sections[current].append(line)

    return {title: "\n".join(lines) for title, lines in sections.items()}


def meaningful(content: str) -> str:
    content = strip_comments(content)
    content = re.sub(r"(?m)^\s*[-*]\s*$", "", content)
    content = re.sub(r"(?m)^\s*[-*]\s*\[\s?\]\s*$", "", content)
    return content.strip()


def main() -> int:
    body = os.environ.get("PR_BODY") or ""
    template_path = Path(
        os.environ.get("TEMPLATE_PATH")
        or Path(__file__).resolve().parents[1] / "pull_request_template.md"
    )
    try:
        template = template_path.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"::error::读取 PR 模板失败：{template_path}，{exc}")
        return 1

    required = list(extract_sections(strip_comments(template)))
    if not required:
        print(f"::error::PR 模板未定义任何二级标题：{template_path}")
        return 1

    sections = extract_sections(strip_comments(body))
    errors: list[str] = []
    for title in required:
        if title not in sections:
            errors.append(f"缺少 `## {title}` 小节")
            continue
        if not meaningful(sections[title]):
            errors.append(f"`## {title}` 小节缺少有效内容")

    if errors:
        print("::error::PR 描述未满足模板要求。")
        print("当前模板要求填写以下小节：")
        for title in required:
            print(f"- ## {title}")
        print("发现的问题：")
        for error in errors:
            print(f"- {error}")
        return 1

    print("PR 描述检查通过。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
