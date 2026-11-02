# 收据 workers/unified-ownership-implementation-0027/REPORT.md

- 原件：`workers/unified-ownership-implementation-0027/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`f0cbc371db0d488aad672df0c47b832e9265f544f55c7f4f642bf0417c0a43cd`
- 字节数：13612；行数：147

## 关键行（行号: 原文）

```
1: # #1488 续轮 0027 REPORT（按复审 0026 返修）
3: 起点 `b6969af4cc`（42 PASS / 4 已知 FAIL），分支 `codex/1488-budget-full-chain-fix-v2-20261018`，只做了追加提交，没有推送工作分支，也没有合入 main。
4: 终点 HEAD：`4022f12079`（tree 见文末）。本地没跑 cargo；锁和宽对拍都走快通道（`../control/uo-remote.sh`、`wide.sh`），CI 用 `uo-ci-fixed.sh` 跑。
6: ## 提交链（b6969af4cc..HEAD）
10: | 8e9af92d06 / 075016aedd | P1 诊断读数及其 revert（结论：套件内“池与候选同父”不发生） |
11: | 6475737795 | P1-1/P1-2：old_pool 按物理 Ledger 分组，逐账对拍收尾静止点读数；池与候选同父时拒绝保留池；keeper 池按物理 live 计入 |
12: | c0cf2be5a8 / d4f7a81582 / 788b8523f2 | P1-2：锁移到真实准入封存路径；无工作域封存的池以 capture 出生时的冷根为物理父账 |
13: | 4b88c64e0a / 5246bc790d | P2-A（主控裁定）：source_permit 降为合同角色项 SourceClaim；指向锁和重复 reserve 负控锁 |
20: ## 逐条 finding
22: ### P1-1：U 不再作为聚合残差吸收 —— 已闭合
36: ### P1-2：keeper 池按所在父账的物理 live 计入 —— 已闭合
52: ### P2-A：source_permit 降为合同角色项（主控裁定）—— 已闭合
75: ### P3-a：生产 reserve_native_work 拒绝路径 —— 已闭合
88: ### P3-b：Recording::start / start_on 语义 —— 已闭合
90: - `start_on` 的结果类别恢复为 P2-2 之前生产 `start` + `attach_parent` 的口径（`effects.rs:468-497`）：
95: - **锁**：`v3::audit_owner::effects::construction_coverage_tests::p3b_start_on_keeps_pre_p2_2_result_categories`（`effects.rs:2314`），覆盖上面三类，以及父账恰好够 inline+首块时成功、析构后全额返还。原有 p2_2 两条锁仍然 PASS。
97: ### P3-c：C5 改为确定判据 —— 已闭合
101:   - 列入范围：
111: ### P3-d：clear_handles 后复核根残留 —— 已闭合
117:   - 原 p3_1 锁仍然 PASS。
119: ## 锁与宽对拍
121: | 节点 | 锁（全清单） | 宽对拍（0025 范围） |
123: | 788b8523f2（P1） | 43P/4F（log 20261020-021021） | 202P/20F，失败集 = 基线（log 20261020-021134） |
124: | 5246bc790d（P2-A） | 44P/4F（log 20261020-022707） | 203P/20F，失败集 = 基线（log 20261020-022856） |
125: | 4022f12079（终点） | 47P/4F（log 20261020-030040），panic 文案 md5 与 022707 相同 | 206P/20F，失败集逐行 = /tmp/20261020-005513-b6969af4cc.log.fail（log 20261020-030310） |
127: - 4 个已知 FAIL：accept_tail、incremental、normal_append、write_budget（StaleWriter：Carry租约已撤销 / stage缺材料cut / publication暂存资源已失效），与起点相同。
136: ## CI
139:   - `uo (s_session)`：success。锁清单中只有 fixed_radix 这 1 条在本 feature 下可见，结果 PASS；其余 50 条为 NOT_LISTED（属于 feature-gated 模块）。形态与上一轮 39290457765 一致（那一轮为 1 PASS / 45 NOT_LISTED）。
140:   - `uo (s_session,s_capture_native_testonly)`：结论 failure，**47 PASS / 4 FAIL(101)**。4 个 FAIL 即已知的 accept_tail、incremental、normal_append、write_budget，与本地快通道一致。上一轮 39290457765 为 42P/4F，增加的 5 条是本轮新增的锁，全部 PASS。
142: ## 未闭合 / 说明
143: - s_content 库测（不在 CI，也不在锁清单内）：起点 b6969af4cc 本来就有 5 个 FAIL（b2_owner_codec_*、codec_original_*）。P2-A 后失败集不变，只是 live 数值变小（log 023508 对比 023403）。不属本轮引入。
144: - P1 的设计限制见 P1-1 节：组粒度的静止点归属；无 Ledger 的池按合同额计。
147: 终点 tree：`04132b2d23e0255eca5e1506b9bb41853db96e3e`
```
