#!/usr/bin/env bash
# 检查当前 HEAD 历史中的每个提交：说明符合 commitlint，且为已验证的 OpenPGP 签名。
set -uo pipefail

export LC_ALL=C.UTF-8
export LANG=C.UTF-8

if [ -z "${GITHUB_REPOSITORY:-}" ]; then
  echo "::error::缺少 GITHUB_REPOSITORY"
  exit 1
fi

mapfile -t shas < <(git rev-list --reverse HEAD)
if [ "${#shas[@]}" -eq 0 ]; then
  echo "::error::没有可检查的提交"
  exit 1
fi

failed=0
for sha in "${shas[@]}"; do
  echo "检查 ${sha}"
  if ! git log -1 --format=%B "${sha}" | commitlint --verbose; then
    echo "::error::提交 ${sha} 的说明不符合 Conventional Commits"
    failed=1
  fi

  if ! meta=$(gh api "repos/${GITHUB_REPOSITORY}/commits/${sha}"); then
    echo "::error::无法读取提交 ${sha} 的 GitHub 验证信息"
    failed=1
    continue
  fi

  verified=$(printf '%s' "${meta}" | jq -r '.commit.verification.verified')
  reason=$(printf '%s' "${meta}" | jq -r '.commit.verification.reason // "unknown"')
  signature=$(printf '%s' "${meta}" | jq -r '.commit.verification.signature // ""')

  if [ "${verified}" != "true" ]; then
    echo "::error::提交 ${sha} 未经 GitHub 验证（reason=${reason}）"
    failed=1
    continue
  fi

  if ! printf '%s' "${signature}" | grep -q "BEGIN PGP SIGNATURE"; then
    echo "::error::提交 ${sha} 不是 OpenPGP 签名"
    failed=1
  fi
done

if [ "${failed}" -ne 0 ]; then
  exit 1
fi

echo "提交说明与 GPG 检查通过。"
