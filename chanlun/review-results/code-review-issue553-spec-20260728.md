# #553 Spec 轴评审（新上下文独立复算，2026-07-28）

范围 `git diff e8496fcaf0...HEAD`（单文件 `rust/src/bin/p123_fast_replay.rs` +196/−1 + 验收报告）。
规格源 = #553 票体 + SPEC #547 第 17 条 + 验收报告 `issue553-t4-acceptance-20260728.md`。

## 结论：**PASS**（2 条中级登记修正，0 条实装阻断）

核心验收面全部独立复算通过，无一处数字造假。

## 已复算通过（抽验四项）

| 抽验点 | 独立复算结果 |
|---|---|
| env-gated 真门控 | `observe` 首行 `let Some(writer) … else { return Ok(()) }`（`:429`）；关灯侧 `/tmp/wt553_800k_newoff_events.txt` 不存在；单测 `event_dump_disabled_writes_nothing` 绿 |
| 只写不判 | 全仓 grep 生产读点仅 3 处（`:1134` 构造 / `:1415` 写 / `:1582` flush）；`revisions`/`seq` 读写全在 `observe` 内（`:434/:440/:461`），无第二读点；`&NestCandidateEvent` 不可变借用 |
| 既有行未扰（四态） | `38039a6d…396e8` 在改动前 binary（`wt553_p123_base`，SHA `a3aeb071…`）/ 关灯 / 开灯 / 开灯二跑四侧逐字节相同，本机复算成立 |
| 双跑 SHA 可复算 | 2M：`c26904ba…`（既有 8,106 行）/ `627a1227…`（事件 8,076 行）/ `4ecc346b…`（stdout）三面双侧相同、`diff` 全 0；800k：`38039a6d…`/`e2834221…` 同。§5.4 全部分布（rev 8073/1/1/1、div 3971、kind pan 7281、level 6357/1401/278/40、门行 candidates=8073）逐位复现 |
| 其他 | `cargo test --bin p123_fast_replay` = 4 passed / 0 failed；`rustfmt --check` 干净；binary SHA `cbf5e4d5…` 三副本一致 |

## (a) 规格要求但缺失/部分

**A1【中】宽免条款归属错标**（`issue553-t4-acceptance-20260728.md:206`）。
报告写「本票遂按票体『跑不动就取最大可行窗口并照实登记』条款」——`gh issue view 553/547` 正文均无此文字；#547 第 17 条原文只有「重型验证窗口**全量**双跑 diff=0 + 双侧 SHA 封印（#69 协议）」。该宽免来自执行令，不在票据。归属错标令一个字面未达成项（§一子句 2、§二项 5）获得票据外观的 "PASS（窗口缩小）"。修正 = 改标条款来源，PASS 字样降为「按执行令宽免」。

**A2【中】全量尺度证据可得而未取**（`:230`、`:307`）。
报告称全量中止产物「不作对拍面」、run3 独有价值由 200k/800k「替代承担」。实测：`wt553_full_run1_{dump,events}.txt` 与 run2 **逐字节相同**（139,219 B / 1,239,091 B，`diff -q` 通过）；run3（关灯）既有 dump 是 run1 的**严格前缀**（`cmp` 通过）。即全量已跑段的「双跑 diff=0」与「开灯≡关灯」现成可封，恰好补的正是唯一未达成项。

## (b) scope creep

无。判据（dirty (i)–(iv)、trigger 语义、TERM 反查）与 `rust/src/theta_v0/**` 零改动，既有控制流唯一改写为 `pending.contains` 提出局部变量（纯查询，等价）。

## (c) 看似实装但错了的

**C1【低】stdout 行数两处失真**：§5.2 表记「21 行」、§5.3/§四.3 记「13 行」，实测两窗 `.out` 各 **19 行**（全为 `P123_` 门行）。SHA 与 diff=0 正确，仅行数字段错。

**C2【低】写失败的侵入面未声明**：`observe` 的 IO 错误经 `?` 上抛（`:1415`），会中止 prefix pass 并跳过既有 `dump_flush` 与 P421 flush。与 P421（`:1596` 同款 `?`）一致、与 `dump_line`（`let _ =` 吞错）不一致。模块头「只写不判」宣称未含此边界；关灯路径不受影响，故非阻断。
