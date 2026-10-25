#!/usr/bin/env bash
# #1559/#1560/#1561/#1567 落地验收：逐项照草稿验收清单实跑（NNNN 已代入 0044/0045/0046）。
# 用法：在仓库根执行 bash .chanlun/review-results/pbatch-land-20261021/acceptance.sh
# 票状态类（关票）属合入后事项，本脚本只报 PENDING，不计失败；实施票交付项同样 PENDING。
set -u
R=xy7365527-lang/NewChanlun
fail=0
chk() { local name="$1"; shift; if ( eval "$*" ) >/dev/null 2>&1; then echo "PASS  $name"; else echo "FAIL  $name"; fail=1; fi; }
pend() { echo "PENDING  $1"; }

echo "== #1559  ADR 0044"
F=docs/adr/0044-cost-stop-policy-signed-instance-v1.md
chk "1 文件存在" "test -f $F"
chk "2 13 字段齐" "for w in theta fee slippage latency_error holding profile_id symbol granularity tower_base_ref evidence_gate valid_through authorizer authorized_at; do rg -q \$w $F || exit 1; done"
chk "3 两版分节=2" "test \"\$(rg -c '^### .*(P0|P1)' $F)\" = 2"
chk "4 无 <待签>" "! rg -q '<待签>' $F"
chk "5 θ≥1 且两版各一" "test \"\$(rg -o 'theta: *[0-9]+/[0-9]+' $F | wc -l | tr -d ' ')\" -eq 2 && rg -o 'theta: *[0-9]+/[0-9]+' $F | awk -F'[: /]+' '{if (\$2+0 < \$3+0) bad=1} END {exit bad}'"
chk "6a 三项状态=6" "test \"\$(rg -o '(slippage|latency_error|holding): *(Known|ProxyLowerBound|Missing)' $F | wc -l | tr -d ' ')\" = 6"
chk "6b fee Known=2" "test \"\$(rg -c 'fee: *Known' $F)\" = 2"
chk "7 P1 官方费率页" "rg -q 'hyperliquid.gitbook.io/hyperliquid-docs/trading/fees' $F"
chk "8 无未标定常数作输入" "! rg -q '^\s*(fee|slippage|latency_error|holding):.*(2\.0|growth mode)' $F"
chk "9 CONTEXT 指针" "rg -q 'ADR 0044' CONTEXT.md"
pend "10 #1559 签署后 closed（合入归用户）"
chk "10' resolution 含 ADR 0044 路径与版本" "gh api repos/$R/issues/1559/comments --jq '.[].body' | rg -q '0044-cost-stop-policy-signed-instance-v1.md.*v1'"
pend "11 #1441 给出四态之一（下游实施）"
chk "11' #1441 评论引用 ADR 0044" "gh api repos/$R/issues/1441/comments --jq '.[].body' | rg -q '0044-cost-stop-policy-signed-instance-v1'"

echo "== #1560  ADR 0045"
F=docs/adr/0045-sr-v1-policy-signed.md
chk "1 文件存在" "test -f $F"
chk "2 四条齐=4" "test \"\$(rg -c '^- id: SR-[1-4]' $F)\" = 4"
chk "3a SR-1..3 动作记号=3" "test \"\$(rg -c '^- id: SR-[1-3] .*action: (freeze_new|exit_all_in_scope|freeze_new_send)( |\$)' $F)\" = 3"
chk "3b 无非法动作记号" "test -z \"\$(rg -o 'action: *[^ ]+' $F | rg -v 'action: *(freeze_new|exit_all_in_scope|freeze_new_send)\$')\""
chk "4 无断连类触发" "! rg -q -i '^- id: SR-.*(unreachable|disconnect|断连|不可达)' $F"
chk "5 绑 P1 且版本格式" "rg -q '^profile_id: P1-hl-testnet\$' $F && rg -q '^version: v[0-9]+\$' $F && rg -q '^supersedes: (none|v[0-9]+)\$' $F"
chk "6 每条带版本与签署人=4" "test \"\$(rg -c '^- id: SR-[1-4] .*version: v[0-9]+ .*authorizer: [^ ]' $F)\" = 4"
chk "7 签署字段非空" "rg -q '^authorized_at: [0-9]{4}-' $F && rg -q '^valid_through: [0-9]{4}-' $F"
chk "8 动作表默认" "rg -q '^action_map: ADR 0030:31-36' $F"
chk "9a SR-2 accountValue=1" "test \"\$(rg -c '^- id: SR-2 .*accountValue' $F)\" = 1"
chk "9b SR-2 无 withdrawable" "! rg -q '^- id: SR-2 .*withdrawable' $F"
chk "10 CONTEXT 指针" "rg -q 'ADR 0045' CONTEXT.md"
pend "11 #1560 CLOSED（合入归用户）"
chk "11' resolution 含 0045-sr-v1-policy-signed.md v1" "gh api repos/$R/issues/1560/comments --jq '.[].body' | rg -q '0045-sr-v1-policy-signed.md.*v1'"
chk "12 #1380/#1412 评论引用" "for n in 1380 1412; do gh api repos/$R/issues/\$n/comments --jq '.[].body' | rg -q '0045-sr-v1-policy-signed' || exit 1; done"

echo "== #1561  ADR 0028 P2 处置"
F=docs/adr/0028-first-capability-verification-profile.md
chk "1 新节=1" "test \"\$(rg -c '^## P2 处置' $F)\" = 1"
chk "2 票号与 P1/TB-09" "rg -q '#1561' $F && rg -q 'P1-hl-testnet.*TB-09|TB-09.*P1-hl-testnet' $F"
chk "3 授权记录六字段" "rg -q -F 'authorization_id, profile_id@version, tb09_evidence_fingerprint_ref, scope, authorizer, authorized_at' $F"
chk "4 ADR 0021 未改" "git diff --quiet origin/main HEAD -- docs/adr/0021-venue-selection-deferred.md"
chk "5 CONTEXT 指针" "rg -q '#1561' CONTEXT.md"
chk "6 无私钥/凭据值" "! rg -q -i 'private_key|0x[0-9a-f]{64}' $F"
chk "7 #1365/#1368 评论引用 #1561" "for n in 1365 1368; do gh api repos/$R/issues/\$n/comments --jq '.[].body' | rg -q '#1561' || exit 1; done"
pend "8 #1561 closed（合入归用户）"
chk "8' resolution 含节名与结论" "gh api repos/$R/issues/1561/comments --jq '.[].body' | rg -q 'P2 处置'"

echo "== #1567  ADR 0046 + #1657"
F=docs/adr/0046-s-admission-anchor-issuance-and-takeover.md
chk "1 文件存在且最后裁定头" "test -f $F && test \"\$(rg -c '^> 最后裁定：#1567' $F)\" = 1"
chk "2 profile/evidence_class/valid_through 只在被否节" "awk '/^## /{sec=\$0} tolower(\$0) ~ /profile_id|evidence_class|valid_through/ && sec != \"## 被否方案\" {bad=1} END{exit bad}' $F"
chk "3 实施票 #1657 引 ADR" "test \"\$(gh issue view 1657 --repo $R --json body -q .body | rg -c '0046-s-admission-anchor-issuance-and-takeover')\" -ge 1"
chk "4 #1527 blocked_by 含 1567 与 1657" "gh api repos/$R/issues/1527/dependencies/blocked_by -q '.[].number' > /tmp/pbatch_b1527.txt && rg -qx 1567 /tmp/pbatch_b1527.txt && rg -qx 1657 /tmp/pbatch_b1527.txt"
pend "5 release 无 test_prepare（#1657 交付）"
pend "6 八个 admission_prepare_rejects_* 负控（#1657 交付）"
pend "7 test_tb01c_r4_anchor_lock（#1657 候选提交上复跑）"
pend "8 test_tb01c_* 77/77（#1657 候选提交上复跑）"
pend "9 资源合同 ADR 八键（#1657 交付前置）"
pend "10 #1567 按批复关闭（合入归用户）"
chk "10' resolution 首行引用户批复且含 ADR 路径" "gh api repos/$R/issues/1567/comments --jq '.[] | select(.body | contains(\"0046-s-admission-anchor-issuance-and-takeover\")) | .body | split(\"\\n\")[0]' | rg -q '用户 2026-10-21 批复.*6310847303'"

exit $fail
