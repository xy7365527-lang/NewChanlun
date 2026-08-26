#!/usr/bin/env bash
# claude-migrate.test.sh —— #1238 迁移助手沙盒对拍（无真机/无 rsync，用 fake-rsync 模拟镜像语义）。
# 覆盖：preflight 拒绝面 / config-json 白名单与 fail-closed / emit-env / verify 通过+失败 / rollback /
#       baseline/final-copy 的 excludes 传递（含 --include-history）。
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/claude-migrate.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

PASS=0; FAIL=0
ok()   { PASS=$((PASS+1)); printf 'PASS %s\n' "$1"; }
bad()  { FAIL=$((FAIL+1)); printf 'FAIL %s\n' "$1"; }
check() { # check <desc> <expected> <actual>
  if [ "$2" = "$3" ]; then ok "$1"; else bad "$1（期望=$2 实际=$3）"; fi
}
expect_rc() { # expect_rc <desc> <want_rc> <rc>
  if [ "$2" -eq "$3" ]; then ok "$1"; else bad "$1（期望退出码 $2 实际 $3）"; fi
}

# ── fake rsync：记录参数 + cp -a 镜像（excludes 只验证参数传递，语义归 rsync） ──
FAKE="$WORK/fake-rsync"
cat > "$FAKE" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >> "${FAKE_RSYNC_LOG:?}"
# 参数尾两项是 "SRC/" 与 "DST/"
set -- "$@"
if [ "$#" -ge 2 ]; then
  src="${@: -2:1}"
  dst="${@: -1:1}"
  mkdir -p "$dst"
  cp -a "$src". "$dst"/ || exit 1
fi
EOF
chmod +x "$FAKE"

# ═══════════════════════════ preflight ═══════════════════════════
mkdir -p "$WORK/src/.claude"
echo hi > "$WORK/src/.claude/f.txt"
cp "$WORK/src/.claude/f.txt" /dev/null 2>/dev/null

expect_rc "preflight 源缺失拒绝" 1 "$("$SCRIPT" preflight --src "$WORK/nope" --dst "$WORK/dst" >/dev/null 2>&1; echo $?)"
mkdir -p "$WORK/dst-inside/sub"
expect_rc "preflight SRC∈DST 拒绝" 1 "$("$SCRIPT" preflight --src "$WORK/dst-inside/sub" --dst "$WORK/dst-inside" >/dev/null 2>&1; echo $?)"
ln -s "$WORK/src" "$WORK/src-link"
expect_rc "preflight 源是 symlink 拒绝" 1 "$("$SCRIPT" preflight --src "$WORK/src-link" --dst "$WORK/dst" >/dev/null 2>&1; echo $?)"
expect_rc "preflight 正常通过" 0 "$("$SCRIPT" preflight --src "$WORK/src" --dst "$WORK/dst" >/dev/null 2>&1; echo $?)"

# ═══════════════════════════ config-json ═══════════════════════════
cat > "$WORK/cj.json" <<'EOF'
{
  "mcpServers": {"probe": {"command": "echo", "args": ["x"]}},
  "oauthAccount": "secret-account",
  "machineID": "secret-machine",
  "userID": "secret-user",
  "projects": {"/tmp/x": {"hasTrustDialogAccepted": true}}
}
EOF
"$SCRIPT" config-json --config-json "$WORK/cj.json" --out "$WORK/cj.redacted.json" >/dev/null 2>&1
check "config-json 只保留 mcpServers" "mcpServers" "$(jq -r 'keys|join(",")' "$WORK/cj.redacted.json")"
check "config-json 保留 probe 内容" "x" "$(jq -r '.mcpServers.probe.args[0]' "$WORK/cj.redacted.json")"
expect_rc "config-json out==input 拒绝" 1 "$("$SCRIPT" config-json --config-json "$WORK/cj.json" --out "$WORK/cj.json" >/dev/null 2>&1; echo $?)"

echo '{ broken json' > "$WORK/bad.json"
expect_rc "config-json 非法 JSON 拒绝" 1 "$("$SCRIPT" config-json --config-json "$WORK/bad.json" --out "$WORK/bad.out" >/dev/null 2>&1; echo $?)"
if [ ! -e "$WORK/bad.out" ]; then ok "config-json 失败时清除输出"; else bad "config-json 失败仍留输出"; fi

# 缺 jq 分支：PATH 里只放 bash 的 shim（脚本 shebang 是 /usr/bin/env bash），不放 jq
mkdir -p "$WORK/nobin"
ln -s "$(command -v bash)" "$WORK/nobin/bash"
expect_rc "config-json 缺 jq fail-closed" 1 "$(bash -c 'PATH="$1"; export PATH; "$2" config-json --config-json "$3" --out "$4"' _ "$WORK/nobin" "$SCRIPT" "$WORK/cj.json" "$WORK/nobj.out" >/dev/null 2>&1; echo $?)"
if [ ! -e "$WORK/nobj.out" ]; then ok "缺 jq 不落输出"; else bad "缺 jq 仍落输出"; fi

# ═══════════════════════════ emit-env ═══════════════════════════
ENV_OUT="$("$SCRIPT" emit-env --dst /srv/agents/claude)"
case "$ENV_OUT" in
  *"export CLAUDE_CONFIG_DIR=/srv/agents/claude"*) ok "emit-env 输出 CLAUDE_CONFIG_DIR" ;;
  *) bad "emit-env 缺 CLAUDE_CONFIG_DIR" ;;
esac
"$SCRIPT" emit-env --dst /srv/agents/claude --env-file "$WORK/claude.env" >/dev/null 2>&1
if [ -f "$WORK/claude.env" ] && grep -q 'CLAUDE_CONFIG_DIR=/srv/agents/claude' "$WORK/claude.env"; then
  ok "emit-env 落盘 env 文件"
else
  bad "emit-env env 文件缺失/内容错"
fi

# ═══════════════════════════ verify ═══════════════════════════
mkdir -p "$WORK/good"
echo '{ "mcpServers": {} }' > "$WORK/good/.claude.json"
echo x > "$WORK/good/somefile"
expect_rc "verify 正常通过" 0 "$("$SCRIPT" verify --dst "$WORK/good" >/dev/null 2>&1; echo $?)"

mkdir -p "$WORK/badkeys"
echo '{ "mcpServers": {}, "oauthAccount": "s" }' > "$WORK/badkeys/.claude.json"
echo x > "$WORK/badkeys/somefile"
expect_rc "verify 多键拒绝" 1 "$("$SCRIPT" verify --dst "$WORK/badkeys" >/dev/null 2>&1; echo $?)"

mkdir -p "$WORK/badsess/.claude.json.d" 2>/dev/null
mkdir -p "$WORK/badsess/sessions"
echo '{ "mcpServers": {} }' > "$WORK/badsess/.claude.json"
echo x > "$WORK/badsess/somefile"
expect_rc "verify sessions/ 拒绝" 1 "$("$SCRIPT" verify --dst "$WORK/badsess" >/dev/null 2>&1; echo $?)"

mkdir -p "$WORK/empty"
expect_rc "verify 空目标拒绝" 1 "$("$SCRIPT" verify --dst "$WORK/empty" >/dev/null 2>&1; echo $?)"

# ═══════════════════════════ rollback ═══════════════════════════
echo x > "$WORK/rb.env"
"$SCRIPT" rollback --env-file "$WORK/rb.env" >/dev/null 2>&1
if [ ! -e "$WORK/rb.env" ]; then ok "rollback 移除 env 文件"; else bad "rollback 未移除 env 文件"; fi

# ═══════════════════════════ baseline / final-copy（fake rsync） ═══════════════════════════
mkdir -p "$WORK/csrc/.claude" "$WORK/csrc/.claude/sub"
echo a > "$WORK/csrc/.claude/a.txt"
echo b > "$WORK/csrc/.claude/sub/b.txt"
export FAKE_RSYNC_LOG="$WORK/rsync.log"
: > "$FAKE_RSYNC_LOG"

expect_rc "baseline 成功" 0 "$("$SCRIPT" baseline --src "$WORK/csrc/.claude" --dst "$WORK/cdst" --rsync "$FAKE" >/dev/null 2>&1; echo $?)"
if [ -f "$WORK/cdst/a.txt" ] && [ -f "$WORK/cdst/sub/b.txt" ]; then ok "baseline 镜像了内容"; else bad "baseline 未镜像内容"; fi
if grep -q -- '--exclude=sessions/' "$FAKE_RSYNC_LOG" && grep -q -- '--exclude=history.jsonl' "$FAKE_RSYNC_LOG"; then
  ok "baseline 默认 excludes 含 sessions/ 与 history.jsonl"
else
  bad "baseline 默认 excludes 缺失"
fi

: > "$FAKE_RSYNC_LOG"
"$SCRIPT" baseline --src "$WORK/csrc/.claude" --dst "$WORK/cdst2" --rsync "$FAKE" --include-history >/dev/null 2>&1
if grep -q -- '--exclude=history.jsonl' "$FAKE_RSYNC_LOG"; then
  bad "--include-history 仍传 history.jsonl"
else
  ok "--include-history 移除 history.jsonl"
fi

expect_rc "final-copy 成功" 0 "$("$SCRIPT" final-copy --src "$WORK/csrc/.claude" --dst "$WORK/cdst3" --rsync "$FAKE" >/dev/null 2>&1; echo $?)"

# ═══════════════════════════ 汇总 ═══════════════════════════
printf '\n通过 %d / 失败 %d\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
