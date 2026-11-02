# 收据 workers/wide16-0058/REPORT.md

- 原件：`workers/wide16-0058/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`71a4d9e5307cf0931f407388eca278fbe71801376be5ebee8a523dd7b795f7b3`
- 字节数：19488；行数：121

## 关键行（行号: 原文）

```
1: # #1488 宽对拍 16 条失败清零：wide16-0058 报告
3: - 工位：`budget-handoff-fix-v2-checkout`，分支 `codex/1488-budget-full-chain-fix-v2-20261018`。起点 HEAD `bba547492d`，终点 HEAD `df0b98f0f7`。只追加提交，没有 push，也没有合 main。
10:   - 并发窗口（15:38 之前）取得的二分证据（`153132/153303/153416/153538/153653/153807`）已在私有 target 上**全部复跑**（`1705xx–1709xx` 各条，见下），结论与原先一致。
12:   - 起点 HEAD 的 16 条基线 `20261020-152356-bba547492d` 也落在并发窗口内，现以终点 HEAD 的复跑 `165033-df0b98f0f7` 取代。
14: ## 终点验证（HEAD `df0b98f0f7`，私有 target-w16a）
18: | 16 条逐条 `--exact` | 5 PASS（(a)/(c) 全部）、11 FAIL（全是待裁的 (b)，断言未动） | `/tmp/nc-remote/20261020-165033-df0b98f0f7.log` |
19: | 全锁 `.github/1488-uo-locks.txt`，共 84 条 | **84/84 PASS**（cold_root_probe 等相关锁都在其中） | `/tmp/nc-remote/20261020-165151-df0b98f0f7.log` |
20: | 宽对拍 `v3::` | 241 PASS / 11 FAIL / 5 ignored。失败集**恰好等于** 11 条 (b)，没有新增失败 | `/tmp/nc-remote/20261020-165500-df0b98f0f7.log` |
35: ## 16 行总表
39: | 1 | `v3::driver::tests::source_keeper_driver_append_repost_restart_preserves_results_and_request_reuse`（**main 绿**） | `e6abdecd33`（末绿 `5c84e6f009`） | 冷根首跑在 cold_root.codec_work() 内一律报“冷审计父预算不足”，回落到无票重跑。首跑已经 `take_source_pool`，Err 时把旧来源池随之丢弃，于是重跑时 trial prior=none，跨请求 hits 恒为 0 | **(a)** | 已修，`1263
40: | 2 | `v3::audit_owner::construction_tests::owner_set_real_publish_commit_failure_revokes_everything` | `5cb7253d46`（测试由 `3f2b4fd924` 引入） | 夹具 `set_publish_case` 用 publication:None 自建 PublishPhase；plan_from_candidates 克隆 native 旧 Entry 时没有 WorkScope，于是被拒（“Native文本复制需要当前WorkScope”） | **(c)** | 已修，`69
43: | 5 | `v3::audit_owner::q::actual_nonempty_native_q_accept_preserves_full_raw_without_manual_double_charge` | `5cb7253d46`（`ea41d4dfd1` 引入，从未绿过） | 夹具基线把 slot 的双份历史费计了进去；`detach_slot`（自 #1394 `c45d39be52` 起）在每个固定操作入口都会真实释放 slot 份，因此实测值是基线的一半（82133 对 164266） | **(c)** | 已修，`df0b98f0f7`：基线改为 `historica
44: | 6 | `v3::audit_owner::effects::construction_coverage_tests::owner_set_exact_work_cap_lives_through_effects_then_releases_only_its_charge` | `5cb7253d46`（`9d03c773ae` 引入） | 断言“原 codec retain 仍在同一父账”；v4 下许可随对象析构即返还，codec retain 已退役，父账只剩 cap 和根 M | **(b)** | 见 (b) 表 B1 | `170724`；拟改写 `163119-9f8267fa
58: ## (b) 待裁表（11 条）
60: ### 公共材料
77: **票评论里的相关先例**：#1488 主控裁定（`gh issue view 1488 --comments` 第 1018 行，逐字节选）：
80: 这条裁定针对的是 audit_owner 里的另一条 r14。本表的 B11 与它同型。该票此前已有四到五处裁改断言的先例（评论第 1385 行、第 1463 行）。
82: **拟改写的总补丁**：`workers/wide16-0058/b-proposal.patch`，未提交。游离提交 `9f8267fa93` 上 11/11 PASS，日志 `/tmp/nc-remote/20261020-163119-9f8267fa93.log`。
96: ### 逐条
98: | ID | 测试 | 原断言（HEAD 行号） | 原断言引入 / 意图原文 | v4 条款 | 拟改写的新断言 | 负控：新断言行精确变红 |
112: ### B11 的额外说明，请负责人特别裁定
115: 2. 按 #1488 第 1018 行先例，主控可能要求把 `live() <= before` 加强成账目恒等式：封存后 live = 审计前残余 + 封存池许可之和，并配三条负控（重复计费、事后追认、挂错父账）。本报告只给最小改写；裁定后可按恒等式重写。
117: ## 风险与观察（16 条之外）
```
