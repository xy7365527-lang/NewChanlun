# 05c compose alloc 优化（task #40，ws-05c）

工作树 WIP（未 commit）——`rust/src/theta_v0/classifier/descend.rs`（`RMove::Compose.subs`
类型改动）、`recursive_tower.rs`（`LeveledMove::compose` + `compose_level`/`compose_level_resume`）、
`rmove_compose.rs`（`compose_move` 构造点同步）、`cand_predicate.rs`/`econ_positive.rs`（测试构造点
同步，编译强制）。

## 阶段0：05c 内部分项计时（codex 审计强制要求，profile-first）

codex 审计（`.chanlun/review-results/a3-impl-codex-audit-20260702.md` 焦点3）明确要求：**先对
05c 内部再切一刀分项计时（clone vs compose vs alloc），不要先上重抽象**。在 `LeveledMove::compose`
内部 + 调用点各插入 env-gated（`THETA_PROFILE_STAGES`）子阶段标签：

- `05c1_subs_clone`：调用点 `[subs_moves[win[0]].clone(), win[1].clone(), win[2]].clone()]` 构建
  临时数组。
- `05c2_compose_call`：`LeveledMove::compose` 整体调用，内部再分：
  - `05c2a_rmove_clone`：`subs.iter().map(|m| m.rmove.clone()).collect()`（`RMove::Compose.subs`
    构造）。
  - `05c2b_submoves_alloc`：`Rc::new(subs.to_vec())`（`LeveledMove.sub_moves` 侧车分配）。

**1M CL 实测（修前，`THETA_PROFILE_STAGES=1 A3_PROFILE_BARS=1000000`）**：

| 阶段 | 耗时 | 占 05c 比例 |
|------|------|------------|
| 05_compose_resume（总） | 32358.9 ms | — |
| 05c_tail_upper_build | 31382.8 ms | 97.0% of 05 |
| 05c1_subs_clone | 7233.9 ms | 23.0% |
| 05c2_compose_call | 15161.9 ms | 48.3%（≈05c2a+05c2b） |
| ├─ 05c2a_rmove_clone | 7392.2 ms | 23.5% |
| └─ 05c2b_submoves_alloc | 7295.9 ms | 23.2% |
| （残差：迭代器/collect/仪表开销） | ~8987 ms | 28.6% |

300K CL 复测比例一致（05c1≈22%/05c2a≈23%/05c2b≈23%），**排除单一大头假设**——三个克隆/分配点
耗时相当，无一项占绝对主导。这否定了"只优化 compose() 本体"的单点方案，确认需要**两处同修**。

## 根因（结构性，非局部低效）

`RMove::Compose { subs: Vec<RMove>, .. }`（`descend.rs`，Lean `Move` μF 镜像）**按值持有** `subs`，
递归嵌套——上级走势的 `subs` 内若含 `Compose` 变体，其自身 `subs` 又是一整棵子树。`Vec<RMove>`
的 `#[derive(Clone)]` 对 `Compose` 变体递归深拷贝整棵子树。三个耗时点是同一根因在不同调用层的
表现：

1. **05c1**：调用点 `subs_moves[win[i]].clone()` 克隆 `LeveledMove`（含其 `rmove: RMove` 深树）。
2. **05c2a**：`compose()` 内 `m.rmove.clone()` **再次**克隆同一批 `RMove` 深树（构造新 `Compose.subs`）。
3. **05c2b**：`compose()` 内 `subs.to_vec()` **再次**克隆同一批 `LeveledMove`（构造 `sub_moves` 侧车）。

同一批数据被深拷贝三次（05c1 一次、05c2a 对其 rmove 字段再一次、05c2b 对整个 LeveledMove 再
一次）——这是 05c 耗时的结构性来源，不是某个函数写得低效。

## 实施（两处同修，A1 Rc 化家族延伸）

**(A) 调用点消除临时数组 clone**：`detect_centers_windowed{,_resume}` 恒产出连续窗口索引
`[i, i+1, i+2]`（`win`）——`compose_level`/`compose_level_resume` 原先 `[a.clone(),b.clone(),c.clone()]`
构建临时数组再传给 `compose()`，实为多余：直接切片 `&subs_moves[win[0]..win[0]+3]` 即可（`compose()`
本就只需 `&[LeveledMove]`）。零风险——同一数据，只是不预先拷贝。**消灭 05c1 全部耗时。**

**(B) `RMove::Compose.subs` 由 `Vec<RMove>` 改 `Rc<Vec<RMove>>`**（`descend.rs`，task #104 之后
A1 Rc 化家族第二处）：`Compose` 变体的派生 `Clone` 对 `subs` 字段变为 `Rc::clone`（O(1) 引用计数），
不再递归深拷贝子树。**直接消灭 05c2a**；因 `LeveledMove.rmove` 字段克隆代价随之坍缩为 O(1)（对
`Compose` 变体）或本就 O(1)（对 `Segment` 变体），**间接消灭 05c2b**（`subs.to_vec()` 现在克隆
的是廉价的 `LeveledMove`）。

不变量保持：`Rc<Vec<T>>` 的 `PartialEq`/`Eq`/`Debug` 均按内容（deref 后逐元素）比较/打印，
`descend()` 的 `subs.as_slice()`/`.iter()`/`.first()`/`.last()`（`descend.rs`/`cand_predicate.rs`/
`coverage.rs`/`interp.rs`/`mod.rs` 各读取点）经自动解引用透明工作，**无需改写任何读取点**——
只有 6 处构造点需要 `Rc::new(...)` 包装（`recursive_tower.rs::compose`、`rmove_compose.rs::
compose_move`、`descend.rs` 3 处测试 fixture、`cand_predicate.rs` 1 处测试 fixture、
`econ_positive.rs` 5 处测试 fixture）。这不是给 Lean μF 镜像加字段（结构层保持纯净），只是
同一逻辑字段的 Rust 侧共享表示优化，与 `LeveledMove.sub_moves`（task #104）同一处理原则。

## 计时对照（1M CL，同一套插桩代码，修前 vs 修后）

`THETA_PROFILE_STAGES=1 A3_PROFILE_BARS=1000000 cargo test --release -p newchan_rust
profile_stage_a3_cl -- --ignored --nocapture`：

| 指标 | 修前 | 修后 | 降幅 | 加速比 |
|------|------|------|------|--------|
| 1M bar 墙钟 | 68.02 s | 16.77 s | 75.3% | 4.06x |
| 05_compose_resume | 32358.9 ms | 2193.7 ms | 93.2% | 14.75x |
| 05c_tail_upper_build | 31382.8 ms | 1308.0 ms | 95.8% | 24.00x |
| 05c1_subs_clone | 7233.9 ms | 0（切片消灭，不再单独计时） | 100% | — |
| 05c2_compose_call | 15161.9 ms | 1025.8 ms | 93.2% | 14.78x |
| 05c2a_rmove_clone | 7392.2 ms | 235.2 ms | 96.8% | 31.4x |
| 05c2b_submoves_alloc | 7295.9 ms | 301.5 ms | 95.9% | 24.2x |
| 05_span avg（续扫跨度） | 5.34 | 5.34（不变） | — | 零行为变化确认 |

`05_span` avg 修前修后逐位相同（同一 bar 序列驱动同一扫描游标语义），确认本次优化**纯性能**，
未触碰 `detect_centers_windowed_resume` 的扫描逻辑/frontier 语义。

## 护航验证（全绿）

- `cargo test --release --lib bit_exact`：54 passed，0 failed，8 ignored。
- `cargo test --release --lib theta_v0::classifier::`：232 passed，0 failed，4 ignored。
- `cargo test --release --lib a3_oracle`：2 passed，0 failed，1 ignored（需 CL 数据）。
- `cargo test --release --lib cascade_reset_on_frontier_interior_rewrite`：1 passed。
- `cargo test --release --lib golden`：6 passed，0 failed。
- `cargo test --release --lib`（全量）：1394 passed，0 failed，100 ignored（CL 数据门控）。
- `cargo test --release --tests`（全部集成测试二进制）：全部 `ok`，0 failed。

（过程中一度撞见 `perm_test.rs`/`wverify_run.rs` 编译错误——`git status` 核实这两个文件由另一
并发工位（task #51，σ^H 桶键+分层维度 walk-forward LCB）实时编辑中，与本工位改动无关，也从未
被本工位 Edit/Write 触碰；对方稳定后复测即绿，非本次改动引入。）

## 结果包（result-package.md 六要素）

1. **结论**：05c（tail_upper_build，1M CL 占墙钟 60%+ 的最大靶）的三个等量级克隆/分配点
   （05c1 调用点数组 clone / 05c2a compose 内 rmove 深拷贝 / 05c2b compose 内 sub_moves 深拷贝）
   经"切片消灭冗余拷贝"+"`RMove::Compose.subs` Rc 化"两处同修，1M CL 全量增量塔构造墙钟从
   68.02s 降至 16.77s（4.06x），`05_compose_resume` 从 32.36s 降至 2.19s（14.75x）。护航测试
   （golden×6/bit_exact×54/classifier×232/oracle×2/cascade_reset×1/全量 lib×1394）全绿，`05_span`
   跨度指标逐位不变确认零行为变化，纯性能优化。
2. **定义依据**：`.chanlun/review-results/a3-impl-codex-audit-20260702.md` 焦点3（"先对 05c
   内部再切一刀分项计时，不要先上重抽象"的强制方法论要求 + H-clone 定性裁决）；task #104
   （A1 Rc 化家族首例，`LeveledMove.sub_moves: Rc<Vec<LeveledMove>>`——本次是同一原则在
   `RMove::Compose.subs` 字段上的第二次应用）；`descend.rs::RMove` 结构层"Lean Move μF 逐字段
   镜像"设计约束（Rc 化不添加字段，只改变现有字段的 Rust 侧表示，结构层保持纯净）。
3. **边界条件**：本优化在"`RMove::Compose.subs` 的 `PartialEq`/`Eq`/`Debug` 按内容比较"这一
   `Rc<Vec<T>>` 标准库保证下成立——若未来任何代码依赖 `Rc` 指针身份（`Rc::ptr_eq`）而非内容
   相等来比较两个 `RMove::Compose`，其语义会与旧版（值语义）不同（当前代码库无此依赖，已通过
   全量测试验证）。若未来某处需要修改已构造的 `RMove::Compose.subs`（原地写），`Rc::make_mut`
   会在 `strong_count > 1` 时写时复制（退化为旧版深拷贝行为，非破坏，仅退化）——当前塔构造是
   一次性组装（compose 后不再原地改写 subs），不触发此路径。
4. **下游推论**：泳道 A（05 塔构造增量路径）当前最大靶已从 30636ms/60% 降至 ~1.3s 量级，
   1M CL 全量增量分类总墙钟大幅下降，为 #13（W-VERIFY alpha 全量重测）和 #51（a3-实装①
   walk-forward LCB，同样跑全历史 CL 增量塔）的运行时预算腾出空间。旁证 07b（extract_second
   frontier 门控，task #23，4004ms 第二大靶）优先级相对上升（05 已不再是压倒性瓶颈，07b 现为
   下一个待验证的相对占比）。
5. **谱系引用**：`.chanlun/review-results/a3-impl-codex-audit-20260702.md`（焦点3 H-clone 定性
   裁决 + 本任务方向来源）；task #104 谱系（A1 Rc 化家族首例，`LeveledMove.sub_moves`）；090号
   严格性语法规则（"两个方案都有效，取更严格/更少残留"——本次选择两处同修而非单点 patch，
   因阶段0数据显示三点等量级，单点修不能解决整体瓶颈）；formalization-validity-domain 231号
   （本节计时数据为 L2：真实 CL 数据、可能否证"某单点主导"假设的相对占比测量，未做 L3 跨标的
   交叉验证——但性能优化非経验假设检验，bit-exact 护航测试已充分验证正确性）。
6. **影响声明**：改动 `descend.rs`（`RMove::Compose.subs` 类型 `Vec<RMove>`→`Rc<Vec<RMove>>`，
   影响其派生 `Clone`/`PartialEq`/`Eq`/`Debug` 的内部实现但不改变外部语义）、
   `recursive_tower.rs`（`LeveledMove::compose` 内部实现 + `compose_level`/`compose_level_resume`
   调用点切片化，均为非公开签名变化）、`rmove_compose.rs::compose_move`（内部包装
   `Rc::new`，函数签名不变）、`cand_predicate.rs`/`econ_positive.rs`（6 处测试 fixture 构造点
   同步 `Rc::new` 包装，编译强制，非行为变化）。**未修改**任何公开函数签名、任何业务逻辑分支、
   任何测试断言的期望值。零 git 操作（遵照工位约束）。未触碰 `econ_positive.rs`/`signal.rs`
   业务逻辑（团队并发域声明覆盖范围）——`econ_positive.rs` 的 5 处改动**仅限**测试 fixture 内
   `RMove::Compose` 构造语法（`subs: sub_rmoves` → `subs: Rc::new(sub_rmoves)`），系类型改动
   传导的编译强制修正，不改变任何测试的业务语义或期望值；若与 ws-07b 的并发域声明仍有交叉，
   请团队协调复核（本工位判断：纯语法层修正，无语义冲突，但如实披露供裁决）。
