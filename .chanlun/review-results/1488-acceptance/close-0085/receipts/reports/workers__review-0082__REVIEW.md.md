# 收据 workers/review-0082/REVIEW.md

- 原件：`workers/review-0082/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`155b3af7bf7c4744f0db446272b16cc31f8a9cd82b297a81fe9b95cd4297cd78`
- 字节数：9762；行数：88

## 关键行（行号: 原文）

```
1: # #1488 0077 + 0079 非作者独审（review-0082）
3: - 被审：review2-checkout detached `26391ae34f3be4bda5729b87d2710cda568a6395`；范围 `git diff 3ee05a2713 26391ae34f`（12 提交：0077 a041499efc/799bff65e7/2cf884e204/4fe8a89b3d/98aa65eccd/06582330d7/b4e168b64d；0079 4dbbd57846/63ecdb9548/4686f58856/2c2d467e26/26391ae34f）。
5: - 主控裁定按 #1488 最新评论执行：P28 读法「推导式任一计入项单点改动即变红」；P57 已接受；P58 留待 squash 时 fmt。
7: ## Verdict：**通过（PASS）**，附 6 条 P3 级小项（均不阻断，见末节）
9: ## 一、逐条表（0077 P3 修复）
11: | 项 | 锁 | 负控（红证） | 本审结论 |
14: | P12 | `recover_lock_promotes_after_duplicate_ingest_and_chained_rebinds`（1810） | NA12_20：红在 1827 | 真实；但负控只在首步就红，链式部分没有单独的负控（P3-c） |
15: | P13 | effects.rs:2676-2695，改为读 journal 真实 `material_epoch`：`Δ==3`、`keys[0]==before+1`、`keys[1]==before+3` | N13：红在 effects.rs:2692（left 2 / right 3）；**基线 3ee05a2713 上同一变异只在 2695 的 `finish().unwrap()` 红** | **可证伪成立**。基线红点证明原断言恒真；新断言直接因计数值差异变红 |
16: | P15 | effects.rs:2743 注释订正 | 不适用（纯注释） | 与代码一致 |
17: | P18 | `.github/1488-uo-locks.txt` 补注释 | 不适用 | 一致 |
18: | P20 | receipt 锁 | NA12_20：红在 1793 | 真实 |
19: | P28 | 推导式锁 | R14：15 项单点变异全部红（收据在 2cf884e204 布局下，行号比 HEAD 多 18 行，映射后一致） | 按主控读法成立 |
22: | P55 | publication_degrade_verdict_tests 新增两行：(a) reject 后 `observe_for_test` 须含「工作账损坏」；(b) 过桥错误原样 Err | N55a、N55b 红在 6238（2cf884e204 布局下为 6256）；**基线上 N55a 为绿** | 成立。基线为绿说明原真值表缺语义失败覆盖，正是本条要补的。行 (b) 没有单独的负控（P3-d） |
28: ## 二、0079 P24-adj（冷根失败无票重跑后续租 Carry）
45: **探针 H 转绿（本审亲跑，悬空 80f3f5f955 = HEAD + 0078 probe.patch + 本审新增 `r82_probe_h_oracle`；ref `refs/uo1488/r82-80f3f5f955`；日志 runs/20261021-122410-80f3f5f955.log、runs/r82-probe.log）**
55: ## 三、生产二进制逐字节不变（亲建）
63: ## 四、全锁与宽对拍（亲跑 HEAD）
66: - 宽对拍：`706 passed; 23 failed; 16 ignored`（runs/r82-wide.log，sha256 d9501dbb…）。23 项失败 = financial_session 22 项（都是 `MissingDependency：会话保留量读数当前只在macOS实现`）+ `v3::state::transaction_material_qualification::actual_entry_seven_exclusions_and_fresh_drop` 1 项，与作者报告的 706/23 完全一致。
69: ## 五、fmt（P58 主控裁定留待 squash）
71: - rustfmt 1.10.0 `cargo fmt --check`：基线 3ee05a2713 与 HEAD 都是 HUNKS=138，**按文件计数完全一致**（driver 34、audit_owner.rs 19、request_audit 15 …）。
72: - HEAD 上位置变化的 hunk（driver 1980/1991/1999/2096/2316/2455/2464，aux_index 4005/4082/4090，effects 2689/2727/2784…）按本次插入的行数（+64、+11、+204、+62、+5/+6）换算后，都对应基线已有的 hunk。本票新增代码没有引入 fmt 漂移。
75: ## 六、收据入仓一致性
77: - verify_receipts.py 逐份核对 sha256、字节数、行数、头行和关键行（按前缀；收据会截断长行）。
81: ## 七、发现（全部 P3，不阻断）
85: 3. **P3-c** P12 的负控只在首步（1827）就红，链式 rebind 部分没有单独的负控。
88: 6. **P3-f** 0079 宽对拍收据 113705 与被切断的前一次运行共用同一个日志路径，存在重叠（作者已披露）。本审独立重跑结果相同，不影响结论。
```
