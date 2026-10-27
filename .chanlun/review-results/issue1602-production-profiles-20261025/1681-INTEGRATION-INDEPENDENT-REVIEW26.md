# #1602 / PR #1681 集成候选独审（2026-10-26）

- 评审者：review1681_integration26（非作者；仅 git 对象读取）。
- 精确候选：`14ec6e9b9d9c244b2f3eb49f669ac94f5bb05d05`。
- 集成基线：`99df2ed762201236b89f0e81c0276cb3aa847f0a`；旧候选：`14de447c8361f1da7cb2157503100fdc60ab2387`。
- 结论：**BLOCK（新增交叉路径验收缺口）**。未发现须另作产品裁定的事项；返原 owner 补一条窄测试并提供精确 head 远端证据。

## 已核定的合并与尾差

1. `generation.rs` 保留 main 的 NativeValue、CodecVec、FixedRadix、delta_lane、BiLedgerView::Delta 与 prepare_raw_history_delta；配置注入通过 with_config，full/delta 共用 generate_impl 的 `self.theta_config`（约 761 行），Generated 由同一实例生成 record_labels（1181 行）。没有退回全历史旧分支。
2. `prepared.rs` 三条路径（delta、full lease、full）均从 Generated 取得 policy_records，三种 publication 调用均传到 commit；`commit.rs:288–297` 同事务 INSERT structure_deltas.policy_records。stage.flush_cut、实际 material scope 和条件 staging 均保留。
3. `20202e5 → 14ec6e9` 唯一代码差异为 `prepared.rs:860` 将 clone 移到 Prepared 初始化之前，结束对 metadata 的借用后再移动 metadata。表达式、来源和政策值未改；修正 owner 已发现的 E0505，不放宽生命周期或降级。
4. `policy_records.rs:195–310` 的 signed_value/read_u32 与 main 原逻辑对 missing/null/AwaitingPolicy/非对象保持同一分类和首错；main read_f64/read_str 保留，read_f64 只是解除 feature 条件，有限数值检查仍在。
5. 历史迁移与恢复沿用旧独审；当前 driver 测试仍逐一覆盖旧 unknown、pending recover、换版本重开不覆盖。commit 的 default 标签入口仅 cfg(test)，生产取显式 Generated。
6. 原独审归档用 `git show <候选>:.../FINAL-REVIEW-14de447.md | cmp - <旧独审原件>` 比较，exit 0；`git diff --check 99df2ed 14ec6e9` 无输出。

## 唯一返修项：Native delta 的显式政策戳没有交叉测试

`generation.rs:2645` 的 bound_full_generation_parity 两臂都通过 new 使用默认配置，比较六根和规范字节；policy_records 明确不进这些字节，因此这些比较不能发现 delta 发布戳遗漏/回落默认值。新增的 `generation.rs:4252` 断言位于普通 generate helper，也不经过 delta Prepared 和 SQL 发布。
`driver.rs:618–635` 的 open_with_config 固定 Ordinary；约 782/831/863 行的三组政策历史/恢复测试由此均不走 Native delta。Native S4 使用默认配置，不提供非默认政策值的交叉断言。

验收需原 owner 在现有小夹具补一条 Native delta + 显式非默认政策记录的行为锁：证明真实 delta 路径，检查 structure_deltas 与外部回执使用该记录；一次真实发布失败/重试后仍使用同一配置、失败不产生成功记录。无需新增重型 28 代矩阵或扩大资源帽。

## 验证责任

本叶未占 nc-remote、未在 macOS 编译。旧 CI 40180393627 只支持未变部分。owner 正在验证 14ec6e9 的 native 编译/generation；主控须消费新 head 的实际日志和 CI。补测提交形成新 head 后，只复核上述测试与实际尾差，本报告已核定部分可复用。
