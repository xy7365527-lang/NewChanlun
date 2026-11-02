# 收据 workers/unified-ownership-implementation-0029/REPORT.md

- 原件：`workers/unified-ownership-implementation-0029/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`f255288942452410b82c77d5bcabc78ad3388c28703dbd4f887c1946d7ae11e3`
- 字节数：8619；行数：69

## 关键行（行号: 原文）

```
1: # #1488 返修 0029 报告（复审 0028 findings）
3: - 起点 `4022f12079`；终点 commit `62bad42a1b3a6b08da5a616e6d77c358d83cff70`，tree `8064dff5f546386623b07a207cc4139a082cdc8a`。
7: ## 逐条 finding
9: ### P1-α RESOLVED — discard_after_error 不再用 refresh 洗白静止点
15: ### P2-β RESOLVED — 16,640B 差额已查明
16: - **实测**（r14 输出 R14_P2B_DROP，log 034415）：P2-A 之前封存 527,048B（b6969af4cc，1950 张），现在 399,074B（1147 张），共降 127,974B，由两部分组成：
18:   - ② 16,640B：`NativeNode` 的字段由 `Option<NativeReservation>` 改为 `Option<SourceClaim>`。索引许可按“槽数 × 槽宽”计，槽宽从 241B 降到 225B。旧槽宽取自同布局镜像 `NativeNodeBeforeP2a` 的 size_of，不是手填。1040 槽 × 16B = 16,640B。
23: ### P3-α RESOLVED — node_source_physical_permits 改为实测
29: ### P3-β RESOLVED — claims==0 时也核来源合同
35: ### P3-γ RESOLVED — 重复物理 reserve 的负控补到 codec_vec_grow 和 scratch 入口
46: ### P1-1 残余 — 按主控裁定 (a) 作为本票终态接受
47: - #1488 的裁定是：以“按物理 Ledger 分组记 U + 来源不明许可一律拒绝 + E 侧保守多计”收尾，前提是 P1-α 修到位（上文已完成，并有负控）。
50: ### P3-δ — 按主控裁定 (b) 接受，裁定条件已落地
57: ## 验证
58: - **快通道全锁**（.github/1488-uo-locks.txt，54 条）：每条 finding 修完后都跑了一遍，失败的始终只有四条已知锁。按 finding 顺序依次为：P1-α 48P/4F（033420）、P2-β 48P/4F（034615）、P3-α 48P/4F（035657）、P3-β 49P/4F（040608）、P3-γ 50P/4F（042648）、P3-δ 50P/4F（043658）。log 都在 `/tmp/nc-remote/20261020-<时刻>-<sha>.log`。四条已知失败：accept_tail、incremental（Recover/
59: - **宽对拍**（`workers/unified-ownership-implementation-0027/wide.sh`）：每步的失败集都与基线 `/tmp/w0029-base.fail` 逐名相同（SAME_FAILSET），最终 209P/20F（log `20261020-043807-62bad42a1b`）。新增测试全部通过。
60: - **CI 39321742113**（uo-ci-fixed.sh）：
62:   - `uo (s_session,s_capture_native_testonly)` 组的 54 条锁为 50 PASS / 4 FAIL，失败的就是上面四条已知锁，因此 run 的结论为 failure；
64:   - 本次 CI 原件里没有单独的非测试 `cargo check` 记录，本报告不对它作结论。
66: ## 未闭合
67: - 四条已知 FAIL 锁：accept_tail、incremental（Recover/Carry）；normal_append、write_budget（journal-coexist-cap，待用户）。都不在本轮写集内，未动。
```
