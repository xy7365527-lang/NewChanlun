# #1627 PR #1655 独审第 1 轮

评审模型：claude-opus-5-5（经本机 CLIProxyLocal 代理，非作者）；被审提交：`e5f472f25a`；评审只读运行于 worktree。

**结论：BLOCK。** 有一处误改：§14.1 `selector_keys` 的默认值行被换成了 `tower_base` 的内容。其余各项均通过。

### R1 验收 1–8（逐条实跑）
| # | 结果 |
|---|---|
| 1 `延伸到(\*\*)?笔` | exit=1 ✅ |
| 2 beichi 非 changelog 的「1 分钟 K 线构成的笔」 | exit=1 ✅ |
| 3 `tower_base ∈` | exit=1 ✅ |
| 4 `a₀ 构件值域` | 只输出 `.chanlun/definitions/zhongshu.md` 一行 ✅ |
| 5 level_recursion「合法备选」 | exit=1 ✅ |
| 6 「两处正本间不一致\|v1.4 冲突登记」 | exit=1 ✅ |
| 7 `^### 14.3` | 只命中 `1567:` 一行 ✅ |
| 8 四个文件的「最后裁定」都含 1627 | 输出为空 ✅ |

`git diff HEAD~1 HEAD` 与 `PR diff（git diff origin/main...HEAD）` 逐字一致。

### R2 值域单点 ⚠️ 有一处阻断
- **阻断：** `qujiantao.md:1544` 在 §14.1 `selector_keys` 表里，原默认值行 `[end_time 降序, start_time 降序, idx 升序]……` 被整行替换成「**`segment`**（见 `zhongshu.md` §2.1.1）。代价：无，即现行生产塔 L0……」。
  - diff hunk `@@ -1540,7 +1541,7 @@` 可以确认。
  - 全文件 `rg "end_time 降序, start_time 降序"` 已 0 命中，`selector_keys` 的默认值彻底丢失。
  - 结果是 §14.1 把 `segment` 当成键序默认值，同表「可选值 (b)」里的「可与默认或 (a) 组合」也就失去了所指。
  - 看起来本该改 §14.3 默认值行，却落到了 §14.1。§14.3 的默认值行（`:1574`）实际未改，与 v1.5 沿革里「§14.3……默认值行……改为指针」的说法也不符。
- 值域表只在 `zhongshu.md:163-169` 列了一份。§14.3 的 `kline3`／`trade3` 两行只留代价，出处改成指向 zhongshu；排除行也改成了指针。
- `beichi.md:461` 括号里复述了 `segment`／`kline3`／`trade3`，与正本一致，也符合主控的追加要求。不阻断。
- 全 `.chanlun/definitions/` 里，没有残留把缠论笔当作 a₀／塔底的现行口径：
  - `qujiantao.md:478` 和 `:1483`（stroke 可配置）已加「stroke 不在 a₀ 值域」注。
  - `:449` 和 `:1251` 是代码引文，`:453` 有 #1627 订正注。
  - `:1251`、`:1393`、`:1514` 已改为「延伸到 K 线／成交」。

### R3 §4.5.2 ✅
- 改动只有方向句 `:526`，加上冲突注换成订正注 `:528`。
- 状态标题 `:524`、编排者原话 `:530`、§4.5.3 `:532`、§4.5.4 `:539` 都不在 diff hunk 里。
- 方向句「把塔底沿 `zhongshu.md` §2.1.1 的合法基底往下延伸，乃至**每笔成交**」符合要求。§2.1.1 是 §2.1 的子节。

### R4 beichi ✅
- `:459-462` 拆成两条：①递归基构件 a₀，指向 zhongshu §2.1.1，写明默认值与笔排除；②喂给塔底的最低数据周期是 1 分钟 K 线（现行取值、体系层选择），可换 5 分钟 K 线或逐笔成交，最细到盘口订单。
- #1465 指针、「按 ADR 0041 另增可选值、须重开 zhongshu 票」「当前不在 `tower_base` 值域内」这几句都在。
- 030:14 原文保留。v1.4 changelog 的同一句已加订正注。

### R5 §9.6 代码行号 ✅（抽查约 30 处）
**一致的：**
- `econ_positive.rs:1142-1143`、`cand_delta.rs:85-87`
- `types.rs:70-87`、`:92`、`:95`、`:110`、`:130`、`:353`
- `stream.rs:73`、`:85`、`:156-158`、`:232`、`:266`、`:338-351`
- `rec_stream.rs:124`、`:229`、`:1270-1273`、`:1556-1578`
- `ffi.rs:39-46`、`mod.rs:22`、`:81`、`divergence.rs:508`
- C3 的 8 处「笔=a0」注释全部逐字命中，`trading/types.rs:30` `FIRST_BSP_LADDER = 2` 也对
- `pipeline.rs:140`、`a_nested_divergence.py:408`
- Lean 侧 `rg "towerBase|tower_base|A0Source" formal` 结果为 0

**不阻断的问题：**
- C6 `a_zhongshu_v1.py:181` 是空行，`def zhongshu_from_strokes` 实际在 `:183`。这个旧行号是照搬既有的 §9.3 P5，但 §9.6 自称「逐条核实」。
- C2 漏列：
  - `stream.rs` 的测试段 `:466`、`:554`、`:565-600`（a0=笔 的 A/B 断言）；
  - `ffi.rs:229-232`（pyo3 把 `a0` 参数暴露给 Python）。

### R6 原文引用 ✅
以下引文在 `docs/chanlun/text/blog/` 下逐字存在，且都位于「↑正文」界线之内：
- 057:26、017:40（含后文「1分钟或5分钟线……1秒种线」）、035:16
- 083:14、083:16、064:34、060:42、084:54、030:14

### R7 格式 ✅
- 四个文件的三行头格式都正确。
- 表格按 GFM 管道数校验全部通过（`\|` 已转义）。
- ADR 0013、0035、0041 的相对链接都存在，ADR 0041 第四条与先例段也在。
- 没有本地回环地址字符串。
- 不阻断：zhongshu、beichi、level_recursion 的「**最后更新**」日期没有改成 2026-10-21。

{"verdict":"BLOCK","blocking":["qujiantao.md:1544 §14.1 selector_keys 的默认值行被误替换成 tower_base 的 `segment` 内容（diff hunk @@ -1540），原默认键序 [end_time 降序, start_time 降序, idx 升序] 在文件中已彻底丢失；§14.3 默认值行（:1574）反而未改，与 v1.5 沿革所述不符。须恢复 §14.1 原行，若要改为指针则改到 §14.3 默认值行"],"nonblocking":["zhongshu.md §9.6 C6 引 a_zhongshu_v1.py:181，实际 def 在 :183（旧行号照搬自 §9.3 P5）","§9.6 C2 漏列 recursive_t/stream.rs 测试段 :466/:554/:565-600（a0=笔 A/B 断言）与 ffi.rs:229-232（pyo3 暴露 a0 参数）","beichi.md:461 括号里复述了 segment/kline3/trade3，与正本一致，符合主控要求，仅记为复述","zhongshu/beichi/level_recursion 的「最后更新」字段未改为 2026-10-21"]}
