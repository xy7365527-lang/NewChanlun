# enum2-gating：find_second_type_structure O(subs²) 候选枚举——NO-SHIP（task #60 premise 否证）

工位 ws-enum2 / 认识论：阶段0 diagnostic profile = L1（管线度量，231号，零信息增量）；否证判断 = L1（数
学上界 + 实测双重坐实）/ 代码影响：**零**（诊断插桩加入又移除，`git diff` 净 0，`rust/src/theta_v0/
classifier/{mod.rs,rmove_compose.rs}` 相对 HEAD 无改动）

## 结论

**NO-SHIP**：task #60 的前提——「07b 残余 O(n²) 由 `find_second_type_structure` 的 i1×i2 双重嵌套候选枚
举 + `sublevel_diverges` 的 `position()`/prev 线性重定位构成 O(subs²)」——被阶段0 profile-first 实测**直接
否证**。`subs.len()`（每个 frontier parent 的次级别走势数）在 CL 300K 与 1M 两个窗口下**恒等于 3**
（`diag_position_scan`：avg=3.00 / max=3，300K 与 1M 完全一致），`find_second_type_structure` 的 i1×i2 枚
举与 `sublevel_diverges` 的 position/prev 线性扫因此是 **O(1)（严格上界 O(3²)=9 的小常数）**，不随 n 增长
——这个分量对 07b 残余 O(n²) 的贡献是 **0**，不是 area-memo（task #57）报告推测的「未解正交根因」。

真正驱动 07b `07b_extract_second` 阶段调用次数 300K→1M 呈 n^1.97 近二次增长（area-memo 报告已测：
821,098→8,766,346 次 `segments_diverge`，本工位独立测得 `sublevel_diverges` 调用 1,172,762→12,542,709
次，同一数量级同一指数）的，是**一个完全不同的量**：`extract_second_resume` 每次 memo-miss 处理的
**frontier tail 宽度**（`upper_moves.len() - prefix_count`，即尚未 confirm 的上级走势数）本身随 n **线性
增长**（300K→1M：avg 31.14→94.46，比值 3.03；max 763→2532，比值 3.32——均与 n 比值 3.33 吻合），而
memo-miss **次数**同样随 n 线性增长（3474→12095，比值 3.48）。两个线性因子相乘 ⟹ 总调用次数
`miss_count × avg_tail_width` 呈 **O(n²)**——这才是 07b 残余标度指数≈2.0 的真实来源。

## 定义依据

- `find_second_type_structure`（`rmove_compose.rs:144-169`）：`for i1 in 0..subs.len() { for i2 in
  (i1+1)..subs.len() { ... } }`，port `Origin.RMoveCompose.SecondTypeStructure` 存在性判定——契约锚不变，
  本工位只诊断其**调用规模**，未触及其语义。
- `sublevel_diverges`（`mod.rs:1499-1529`）：`subs.iter().position(...)` + `subs[..idx].iter().rev()
  .find(...)`——本工位诊断两个线性扫的实测跨度（`subs.len()` 上界），非语义改动。
- `extract_second_resume`（`mod.rs:1457-1488`）：07b frontier 门控（task #23），`prefix_count`=confirmed
  前缀数，`upper_moves[prefix_count..]`=frontier tail——本工位诊断该 tail 的宽度分布。
- 231号 formalization-validity-domain：有效域 ≠ 定义域——task #60 把"O(subs²)"的**代数定义域**（closure
  接口允许 subs 任意大）误当成"实际有效域"（真实数据下 subs 的经验分布）。本工位是该规则的第4个实例
  （前3例：222/223/230号）。

## 边界条件（会翻转本结论的条件）

1. **`subs.len()` 恒为 3 的结构性原因未查明但已实测锁定**：若上游 `compose_level`/windowed 组装逻辑改变
   窗口宽度（当前疑似固定 3 段窗口），使 `subs.len()` 不再有界，则本工位的 O(1) 论断失效，需重新评估
   candidate 枚举的渐近代价。当前 300K 与 1M 两个窗口下 `max=3` 完全一致，未观察到任何超过 3 的样本
   （count 分别 1,172,762 与 12,542,709，样本量充分），有界性证据充分但非形式化证明（L1 经验坐实，非
   L0 代数证明——若需要 L0 保证需读 `compose_level` 窗口宽度是否为编译期常量）。
2. **frontier tail 宽度线性增长的根因未定位**：本工位只测量到「avg_tail_width 随 n 线性增长」这一现象，
   未深入到「为什么 `prefix_count`（confirmed 上级走势数）追不上 `upper_moves.len()`（累积创建的上级走
   势数）」的具体机制（是否结构性——如某类上级走势永不 confirm——或参数性——如 confirm 阈值/窗口设置）。
   若后续工位查明 tail 宽度实为有界（本工位 300K/1M 两点观测到的线性增长是过渡态而非渐近行为），则
   07b 残余 O(n²) 的归因需再次修订。
3. **本工位诊断范围限于 CL 单标的**：两个窗口（300K/1M）均取自同一 CL 历史前缀，未跨标的/跨时段验证
   （231号 L2/L3 交叉验证空缺）——tail 宽度线性增长在 CL 上坐实，未断言对 BTC 等其它标的同样成立。

## 下游推论

1. **task #60 原定方案域全部不适用**：「i1×i2 枚举早退/单调剪枝」「position→partition_point/索引 map」
   ——两者都是优化一个已经 O(1)（≤9 次比较）的常数级操作，实施后**不会改变**07b_extract_second 的标度
   指数（仍会是 n^1.97），只会节省若干纳秒级别的比较开销，且需要改动 `find_second_type_structure`/
   `extract_second_signals`/`second_type_imp_broken_center` 三处公开签名 + 上溯 rmove_compose.rs 4处 +
   signal.rs 5处 + theta_v0_classifier_parity.rs 4处共 ~13 个测试调用点——为零渐近收益承担这样的改动面
   是**声明膨胀的反面**（no-patch-mentality：不应为了"看起来在优化"而制造无意义的大改动）。
2. **★真正的下一靶已定位但超出本工位文件域（275号局部依赖，本工位不越界接入）**：`extract_second_resume`
   的 frontier tail 宽度线性增长——需要新工位调查 `prefix_count` 的推进速率为何显著慢于
   `upper_moves.len()` 的增长速率（是否与 07b 门控本身的"单调性守卫"`*cached_count > prefix_count` 触发
   频率有关，或与上级走势 confirm 判据的结构性质有关）。这是 mod.rs 主循环（约 line 1094-1322，
   `dirty_from`/`prefix_count` 计算）而非 `rmove_compose.rs`/`sublevel_diverges` 的问题——不在本工位
   （task #60 文件域：rmove_compose.rs/mod.rs 的候选枚举）范围内顺手修。
3. **area-memo（task #57）报告的归因需二次订正**：其"下游推论2"提出的"(a) `find_second_type_structure`
   改传 i1 下标避免重新 position()"方案经本工位阶段0 profile-first 验证为**无效**（subs.len()≤3，改传
   下标不改变渐近复杂度）；"(b) frontier parent 的 subs 增量缓存"同理不适用（subs 已经是 O(1) 大小，无
   缓存必要）。07b 残余 O(n²) 的**唯一**待解正交分量是 frontier tail 宽度增长，非候选枚举。
4. 信号集完全不变（本工位零代码改动，仅诊断插桩加入又移除）：不影响 W-VERIFY(#13) alpha 有效性判定。

## 谱系引用

- area-memo（`area-memo-20260702.md`，task #57）：提出"find_second_type_structure 候选枚举 O(subs²)"
  假说作为 07b 残余 O(n²) 的正交第二根因——本工位阶段0 profile-first 直接测量该假说的关键变量
  （`subs.len()`）并**否证**其增长性，订正归因至 frontier tail 宽度。
- 07b（`07b-frontier-gating-20260702.md`，task #23）：门控消除 confirmed 前缀重扫 O(U²)，本工位诊断表明
  残余 O(n²) 与门控本身的 tail 宽度增长相关，而非门控遗留的候选枚举成本。
- 231号：形式化有效域规则——本工位是"定义域（closure 接口允许任意 subs 大小）≠ 有效域（真实数据 subs
  恒为3）"的第4实例（继222/223/230号），且是本规则中**否证一个具体优化方案有效性**（而非分类/守恒律/
  直积分解）的首个应用。
- 090号：严格性——不因"任务标题写了 O(subs²)"就实施一个已被数据否证的优化；诚实记录 NO-SHIP + 订正
  归因，而非在无效目标上做表演性重构。
- 275号：局部依赖——frontier tail 宽度增长的根因定位属 mod.rs 主循环 owner 域，本工位（rmove_compose.rs
  候选枚举）不越界接入，留独立工位。
- no-patch-mentality：为"看起来在解决任务标题"而对已经 O(1) 的代码做 13 处签名改动 = 补丁思维的反面
  （制造无意义改动而非追问"严格的形式是什么"）——本工位选择诚实 NO-SHIP。

## 诊断数据表（CL，release，THETA_PROFILE_STAGES=1，插桩已加入又完整移除，`git diff` 净 0）

| 指标 | 300K | 1M | 比值（1M/300K，n比值=3.33）|
|------|------|-----|---------------------------|
| `sublevel_diverges` 调用数（=`diag_position_scan`.count）| 1,172,762 | 12,542,709 | 10.70（指数≈1.97，与 area-memo `segments_diverge` 指数1.97 一致）|
| `subs.len()` per 调用（avg / max）| 3.00 / 3 | 3.00 / 3 | **恒定，无增长**——O(subs²) 假说否证关键证据 |
| prev-scan 跨度（avg / max）| 0.99 / 2 | 0.99 / 2 | 恒定，无增长 |
| `extract_second_resume` memo-miss 次数（=`diag_tail_width`.count）| 3,474 | 12,095 | 3.48（近线性，与 n 比值 3.33 接近）|
| frontier tail 宽度 avg（=`diag_tail_width`.avg）| 31.14 | 94.46 | **3.03（线性增长，真根因）** |
| frontier tail 宽度 max | 763 | 2,532 | 3.32（线性增长）|

护航：`cargo test --release --lib`（1396 passed，0 failed，诊断插桩移除后与 HEAD 完全一致状态下跑通）；
`git status`/`git diff` 确认 `rust/src/theta_v0/classifier/mod.rs`、`rust/src/theta_v0/classifier/
rmove_compose.rs` 相对 HEAD 净 0 改动（诊断代码加入又移除，无持久化）。

## 影响声明

**零代码改动持久化**。本工位在 `mod.rs::sublevel_diverges` 与 `mod.rs::extract_second_resume` 内临时插
入 `stage_profile::record_span` 诊断探针（`diag_position_scan`/`diag_prev_scan`/`diag_tail_width`），用于
阶段0 profile-first 定量验证 task #60 前提，随后**完整移除**（`git diff` 对两个文件均为空）。未改动
`find_second_type_structure`/`extract_second_signals`/`second_type_imp_broken_center` 的公开签名，未改动
任何测试调用点。`cargo test --release --lib` 1396 passed / 0 failed 确认移除后状态与协作者上次提交
（area-memo/task #57）完全一致，无回归。task #60 判定为 NO-SHIP：候选枚举优化方案不适用，真根因（frontier
tail 宽度线性增长）已定位并移交新工位（超出本工位 rmove_compose.rs 文件域）。
