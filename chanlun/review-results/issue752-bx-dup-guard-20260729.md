# Issue #752 — B_x 段 raw 级重复 release 防线缺口评估（#742 残余节收口）

工位：`/private/tmp/wt-752`（分支 `ticket-752`）。背景面：`chanlun/review-results/
issue742-512-guard-order-20260729.md`「停手项」节——该票问 1 差集小节登记的残余：
「release 构建下，B_x 段的 raw 级重复不会被 `#183`/`#512` 任何硬门捕获，会被
`exit.rs:330-335` 的 `raw_ids` HashSet 静默折叠，不 panic；这类重复目前由 open 循环
自身的 `#216` 判重逻辑在更上游预防，若该逻辑本身有漏洞，release 下确实无兜底」。

代码锚：`rust/src/theta_v0/strategy/coverage/step.rs`——open 循环 `#216` 判重
（320-397 行，本票内未改行号）；`rust/src/theta_v0/strategy/coverage/held.rs`——
`restore_ancestor_chain_from_registry`（282-376 行）；`rust/src/theta_v0/strategy/exit.rs`
——`step_active_set_with_subtree_close` 的 `raw_ids` 折叠（320-358 行）。

## 结论摘要（两问）

1. **不可构造**：B_x 段（`step.rs` open 循环产出、经 `step_active_set_with_subtree_close`
   的 `opened` 参数传入）的 raw 级同 `ElementId` 重复，在 main 生产路径上**结构性不可构造**
   ——不是「目前没测到」的经验观察，是穷举 B_x 段全部推入点后得到的排他性证明（见问 1）。
2. **`#216` 判重逻辑本身足够，不需要补防线**：因为不可构造，不满足「补 release 硬门」的
   前提条件（票面：可构造才补）。已在 `exit.rs:330` 附近 `raw_ids` HashSet 处、`step.rs`
   open 循环 `#216` 注释处各补一段论证链注释，指向本报告，不改任何逻辑。

---

## 问 1 — 可构造性：静态路径推导

### B_x 段能进 `raw` 的全部途径（穷举）

`raw: Vec<usize>` 是 `coverage_step_from_buckets_sep_with_risk_seeds`（`step.rs:95`）内
唯一的候选索引累加器，held 循环（A_t 段）结束后 `a_t_end = raw.len()`（`step.rs:300`）
定住分段点，此后 open 循环（320-397 行）产出的追加部分即 B_x 段
（`raw[a_t_end..]` → `b_x_legs`，`step.rs:439-442`）。

对 `rust/src/theta_v0/strategy/coverage/{step.rs,held.rs}` 做 `raw.push`/`raw.retain`
全量 grep，落在 open 循环执行窗口内（即可能写入 B_x 段）的推入点只有三处：

| # | 位置 | 触发路径 | 推入前置检查 |
|---|---|---|---|
| ① | `step.rs:349` `raw.push(idx)` | open 桶候选自身 carrier 直接入 raw | `open_pushed.get(&cid)`（本循环内已推入的候选，按 `ElementId` 键）**或** `raw.iter().any(\|&r\| work.get(r).id == cid)`（对**当前完整** `raw`，含 A_t 段 + 本循环已推入的 B_x 段，按 `ElementId` 值扫描）——见 `step.rs:340-352` |
| ② | `held.rs:337` `raw.push(existing_idx)` | 祖先链上溯命中「已在 `work`（base 树前缀 / 本 bar overlay）但不在 `raw`」的既有元素，复用其 idx | 调用者 `restore_ancestor_chain_from_registry` 循环体顶部 `already_in_raw = raw.iter().any(\|&r\| work.get(r).id == pid)`（`held.rs:308`），为真则 `break`，**不会走到** ②/③ 的 push | 
| ③ | `held.rs:369` `raw.push(op_idx)` | 祖先链上溯命中「`work`/`raw` 均无、须从 persistent registry 恢复」的新元素 | 同上，同一顶部 `already_in_raw` 门禁 |

（`step.rs:189/226/259/279` 与 `held.rs:369` 所在函数在 held 循环中的调用点全部发生在
`a_t_end` 定住之前，属 A_t 段，不产生 B_x 段成员；`resolve_pending_parent_fixups`
（`held.rs:469-504`）只回填已推入元素的 `parent`/`attached_dir` 字段，不 `push` 索引，
对 `raw` 是只读切片 `&[usize]`——已核实签名。）

### 排他性论证

三处推入点共享两个结构性事实：

1. **门禁键是 `ElementId` 值，不是 `work` 下标**。①的 `open_pushed` 以 `cid: ElementId`
   为键；①的 `id_in_raw`、②③共用的 `already_in_raw` 都用 `work.get(r).id == 目标id` 逐条
   比较，而非比较下标本身。这意味着即便假设上游 `work` 里出现两个不同物理下标携带同一
   `ElementId`（更深一层、超出本票范围的树构造不变量——`#183` 的 `raw` 全量唯一性硬门
   本身就是防这类情形的，属另一层已有防线），本层的判重仍会按 id 值正确折叠/让位/湮灭，
   不产生同 id 两条目。
2. **门禁检查的是「当前完整 `raw`」，不是各推入点各自维护的局部状态**。`raw` 自
   `step.rs:110` 声明起是单个 `Vec<usize>`，held 循环、open 循环共享同一份，`a_t_end`
   只是记录长度的分段点、不是两份独立向量。open 循环内任一次 `already_in_raw`/`id_in_raw`
   扫描看到的都是「A_t 段全部 + 本循环目前为止已推入的 B_x 段全部」，包括同一循环内更早
   iteration 的候选推入（①）与更早的祖先恢复推入（②③）——不存在「A 组检查时看不到 B 组
   已推入内容」的时序缺口。

推论：设想任意两次推入操作试图为同一 `ElementId` 都成功落地到 `raw`——按代码结构，
第二次操作发生时，`raw` 中已经存在第一次操作留下的同 id 条目（前置条件 2），而第二次
操作的门禁检查必然扫描到它（前置条件 1 保证按值而非按下标匹配），从而使第二次操作
在 push 语句之前分流（①：让位或湮灭；②③：`already_in_raw` 为真直接 `break`，函数在
到达对应 push 语句之前就返回）。三处推入点穷尽了 B_x 段的全部生成途径（上表），故不存在
遗漏的第四条无门禁推入路径。因此 `raw[a_t_end..]` 内部、以及 `raw[a_t_end..]` 与
`raw[..a_t_end]`（A_t 段）之间，均不可能出现同 `ElementId` 的两个条目——**这是由三处
推入点的门禁纪律共同保证的结构性不变量，不依赖对生产数据分布的经验假设**，单线程顺序
执行（无并发/重入）是该论证成立的前提，与本函数实际调用方式（`step.rs:452` 单次同步
调用）一致。

### 与 `#183`/`#512` 检测域的关系（对照 #742 报告）

`#742` 报告问 1 已指出 `#183` 检测「raw 全量」、`#512` 检测「`next_idx`（存活子集）」，
两者对 B_x 段内部重复均不覆盖（`#183` release 下不生效，`#512` 检测域是 A_t 子集不含
B_x）。本票补的是第三层：**B_x 段内部重复本身在 main 路径上不会发生**——不是「有硬门
测不到」的缺口，是「构造该形态所需的代码路径不存在」。三层证据互不重复：`#742` 证明
「若发生，两道硬门测不到」；`#752`（本票）证明「不会发生」。

## 问 2 — 防线充足性

因问 1 结论为不可构造，不满足票面「可构造才补 release 硬门」的条件分支。`#216` 判重
逻辑（`open_pushed` 键值判重 + `already_in_raw` 门禁）本身足够，理由即问 1 的排他性
论证。已在以下两处补论证链注释（纯注释，无逻辑改动，指向本报告）：

- `rust/src/theta_v0/strategy/exit.rs`——`raw_ids` HashSet 折叠处（约 330 行）：说明
  该折叠对 main 路径的合法 B_x 输入是恒定空操作（never-actually-folds），仅对测试/未来
  误用输入起兜底作用。
- `rust/src/theta_v0/strategy/coverage/step.rs`——open 循环 `#216` 注释块（约 320 行）：
  补充指向本报告的排他性证明指针。

不新增 `debug_assert`/`panic!`：证明已确立该折叠在 main 路径上恒为 no-op，额外断言不会
增加可执行的新信息（若违反，意味着违反的是更上游的三处推入门禁本身，应在门禁处而非折叠
消费端捕获——门禁处本就有 `already_in_raw`/`open_pushed` 的显式检查，无需重复）。

## 测试记录

本票只新增注释，无逻辑改动，无新增测试。

```
cargo test --lib
```

结果：2587 passed, 0 failed, 138 ignored（与 `#742` 报告基线一致，零新增红）。

## 停手项

无新增。`#742` 报告「停手项」节至此收口——三层证据（两道硬门检测域互补 + B_x 段自身
不可构造）拼合完整，无残留悬空面。
