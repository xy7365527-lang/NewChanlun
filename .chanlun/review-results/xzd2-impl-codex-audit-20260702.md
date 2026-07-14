# codex 审计：小转大通道阶段2实装——C3 死门裁定（全权裁定）

> **【效力域降级】编排者裁定 (a)（2026-07-02，task #80）**：本报告依赖 tower/rung 跨级定位口径。`rung-interval-containment-20260702.md`（task #77）已用 350K BTC 实测**否证**"端点相等 rung 定位系统性 false negative"（区间套问题①）——两口径 bit-identical，FN=0，本报告计数**不受该项影响，结论获加固**。但 **frontier 问题②（tail 宽度/cached_units 快照）污染仍未清除**（task #65/#75 跟踪中）——本报告的分布/命中率数字尚未排除该项干扰，在 frontier 问题②收口前，结论视为**条件性**（原文数据保留不改）。

- task: #33（ws-xzd2audit，codex-challenger 工位）
- date: 2026-07-02
- 调用方式：`codex exec --skip-git-repo-check --sandbox read-only -`（强制 CLI，编排者指令）
- 权限：编排者明令「有要裁定的直接问 codex，他全权裁定」——本文件是终局裁定，不再上浮
- 认识论等级：**L1（代码结构静态审计 + 已有 L2 报告交叉引用）**——本次审计未新跑 350K/862条 dx harness，
  仅对已有实测数据（114/0）做结构性归因；codex 自己在裁定中也明确标注了这一边界

---

## 1. 被审对象

`rust/src/theta_v0/backtest/econ_positive.rs` 新增小转大确认通道（工作树 WIP，未 commit）：
`XzdEvidence` / `GateCertificate{Nest,Xzd}` / `build_gate_certificate` 二通道分派 /
`xiaozhuanda_confirm`（C2/C3 判据）/ 生产门 `collect_signals` 二通道改造 / 3 个 xzd 单元测试。

设计稿：`.chanlun/review-results/xiaozhuanda-design-20260702.md`
前审：`.chanlun/review-results/xiaozhuanda-codex-audit-20260702.md`（三修正强制来源，§6-1/6-2/6-3/6-4）

## 2. 触发背景

350K bar 真实 BTC 回测窗口实测：小转大域（Xzd）触达 114 条信号；C2（本级二类买卖点存在）
= 114/114 = 100% 通过；C3（本级走势最后一个次级中枢出现三类买卖点）= **0/114，从未触发**；
C2∧C3 通过 = 0。需裁定：0/114 是 (a) 严格门真结果，还是 (b) C3 死门伪影。

## 3. 我（code-layer 审计代理）在提交 codex 前完成的代码级溯源

### 3.1 坐标系一致性核实（排除"current_strokes merged 坐标"类陷阱）

`LeveledMove.start_index/end_index`（不论 level）与 `Center.start_index/end_index` 均为
**L0 原始 K 序坐标**——核实链：`center.rs::UnitRange` docstring 明示"该走势单元在 L0 原始 K 序
的起点/终点"；`center_from_segments`/`center_from_window` 用 `a.start_index`/`c.end_index`
直接透传；`recursive_tower.rs::project_to_units` 用 `m.start_index`/`m.end_index` 逐级传递。
**结论：C3 不是坐标系分叉问题**（与既往 current_strokes/MoveTuple 陷阱不同模式）。

### 3.2 发现的结构性缺口：`LevelState.bsp` 在 level>=1 从不含 Type1/Type3

`classifier/mod.rs` 全量路径（`classify_impl`，mod.rs:254-262）与增量路径
（`classify_with_tower_incremental`，mod.rs:1206-1223，生产 `collect_signals` 实际消费路径）
对 BSP 提取的门控完全一致：

```rust
let mut bsp: Vec<BspPoint> = if is_l0 {
    signal::extract_signals_with_hist(&centers, &l0.segments, &hist, &[], &[], &close_src).0
} else {
    Vec::new()   // level_idx > 0：不产 Type1/Type3
};
bsp.extend(extract_second_for_level(&upper_moves, &hist, &close_src));  // 只产 Type2(B2/S2)
```

`extract_second_for_level`（mod.rs:1322-1355）只调 `signal::extract_second_signals`，只产
`is_second`（B2/S2）位，从不设 `buy3/sell3`。`make_third_point`（唯一构造 Type3 BspPoint 的函数）
只被 `judge_third` 调用，`judge_third` 只被 `extract_signals_with_hist` 调用，而后者只在
`is_l0` 时跑。**推论**：`cls_i.levels[k].bsp`（`k>=1`）结构上永不含 `buy3`/`sell3` 置位的点。
C3 检查 `sub_bsp = cls_i.levels[lvl-1].bsp` 中的 `q.bits.buy3/sell3`——当 `lvl>=2` 时该集合
结构上为空集，`.any(...)` 恒假，与信号质量/市场结构无关。**这部分是纯代码级死门。**

### 3.3 level 分布交叉引用（无法单独解释全部 114）

引用 `.chanlun/review-results/l2-depth-distribution-20260702.md`（全历史 4.6M bar，
`descend_type1_anchor_depth` None 桶 = Xzd 域同源数据）：level1=966(71.4%) / level2=304(22.5%) /
level3=64(4.7%) / level4=19(1.4%)，合计1353。按窗口比例外推（350K/4.6M≈7.6%，
1353×7.6%≈103，与114量级吻合），**约71%的114条（约81条）落在level==1**，其
`sub_bsp=cls_i.levels[0].bsp`——唯一真正含Type3点的level。**"结构性死门"严格覆盖约29%
（level>=2，约33条）；level==1约71%（约81条）无法仅凭代码100%判死**，留给 codex 裁定。

### 3.4 未能自行排除的疑点（level==1 子集）

`q.center`（Type3 bsp 携带）来自 `judge_third` 的 `c` 参数 = `nearest_confirmed_center_idx`
按 **leave 段时间点** 反查的"当下最近已确认中枢"；`last_zs`（C3 查询目标）来自
`sub_centers.filter(落在 s 跨度内).max_by_key(end_index)` 按 **s 的空间跨度** 过滤。
两条选择路径逻辑上不同源，只在特定几何条件下重合。审计代理本人无法判断这种重合在 BTC 1m
实盘几何下是"结构性几乎必然"还是"结构性几乎罕见"，作为开放问题提交给 codex。

---

## 4. codex 探索轨迹摘要（工具调用narrative，完整机器输出见下方裁定原文）

1. 强制启动仪式确认 warm_start / goal active，审计范围收束在 econ_positive.rs + 分类器 BSP 生产链。
2. 尝试 Serena/codebase-memory 图工具定位 `xzd_sub_last_zs_type3`/`build_gate_certificate`/
   `extract_second_for_level`；MCP 图查询被环境取消，按 AGENTS fallback 规则退回本地只读源码读取。
3. 逐字核对 `classifier/mod.rs` 全量+增量两条路径，确认"高层 BSP 只追加 `extract_second_for_level`"
   与审计代理提交的证据一致。
4. 读 `signal.rs` 二类构造 + `find_move_by_end_index`/区间套分支，核对次要审计面。
5. 独立确认：level>=2 是硬死门（`sub_bsp` 无 Type3 生产路径）；level==1 需要两个诊断断点
   （跨度内是否有最后 L0 中枢 / L0 Type3 的 center 是否命中该中枢）才能定论，不臆断。
6. 交叉核对已有350K报告数值（Xzd=114, C2=114, C3=0）与题面一致，但报告缺 level==1 断点拆分。
7. 核对 `build_gate_certificate` 对 Nest 分支的透传（先调原 `build_nest_certificate`，Some 时
   原样返回），确认 Type1/有锚 Type2/3 的 Nest 路径未被 C3 逻辑污染。
8. 收口裁定：部分(b)，足以废掉当前 C3 硬门聚合结论。

---

## 5. codex 终局裁定（原文，逐字保留）

### 5.1 C3 死门裁定：**部分(b)，并且足以废掉当前 C3 硬门结论**

> `level>=2` 子集是 100% 结构性死门：`collect_signals` 把 `sub_bsp` 取成
> `cls_i.levels[lvl - 1].bsp`，而 `classifier/mod.rs` 对 `level_idx > 0` 的 BSP 只从空 vec
> 追加 `extract_second_for_level`；`extract_second_signals` 只产 B2/S2，`center=None`，不可能产
> `buy3/sell3`。C3 又硬查 `q.bits.buy3/sell3 && q.center == last_zs`，所以 lvl>=2 必恒假。
> 对应代码点：econ_positive.rs:307、mod.rs:254、mod.rs:1228、signal.rs:479、econ_positive.rs:871。
>
> `level==1` 不能从静态代码 100% 判死：它的 `sub_bsp=L0.bsp`，L0 确实能通过
> `judge_third -> make_third_point` 产生 `buy3/sell3 + center=Some`。但当前 exact-center
> 判据很窄：`last_zs` 按 `s` 跨度取最后 L0 中枢，而 L0 Type3 的 `q.center` 按 leave 段当下
> 最近中枢归属，两条选择链不是同一函数。350K 报告已有 `Xzd=114, C2=114, C3=0`，但缺少
> level==1 的断点统计，不能诚实地说这部分也是静态死门。
>
> **终局裁定：当前 0/114 不能作为严格门真结果采信；至少部分是实现伪影。C3 不得继续作为
> 小转大硬门。**

### 5.2 处置建议

> 按设计稿预留路径执行：`XzdEvidence::gate_pass()` 退化为**仅 C2**，C3 降为软标注/诊断字段。
> 若未来要恢复 C3 硬门，必须先补高层 Type3 生产，或改 C3 为直接从次级走势/中枢几何计算，
> 而不是依赖 `LevelState.bsp`。
>
> "仅 C2"不是无门：它仍要求 C1 小转大域 + 本级二类结构成立，B2 由递归组装层
> `find_second_type_structure` 产出。但对 Type2 信号自身，C2 是近似自证，因此会明显放宽吞吐。
> 可以进入 W-VERIFY/L2 alpha 评估，但输出必须标成 `C2-only xzd`，不要声明 C3 必要条件已满足。

### 5.3 次要审计面判定

1. **C2 跨条目查找**：正确。同 `source_index` 多条目用 `any` 合理；Long/Short 镜像正确。
   Type2 信号自身会使 C2 近似自证成立，这是语义预期，不是 bug。
2. **GateCertificate 分派**：正确。enum 明确把 `Nest.n_delta()` 与 `Xzd.gate_pass()` 分开，
   未见空 rungs 后门复发（econ_positive.rs:840）。
3. **C3 零前视**：当前 as-of 输入无未来泄漏。`sub_centers/sub_bsp` 来自 `classify_at(i)` 的
   同一快照；`confirm_index` 目前只是证据字段，不造成前视。
4. **Nest bit-exact 声明**：代码结构可信。新门先调用原 `build_nest_certificate`，Some 时原样
   走 `cert.n_delta()`；旧 `build_multilevel_nest_cert` 本身也是 `build_nest_certificate + n_delta`。
   **codex 未在本次只读审计里重跑 350K/dx harness**（与审计代理自述边界一致——本文件不声称
   已做 L2/dx 级验证）。

### 5.4 新增诊断探针规格（供后续工位实装，用于裁断 level==1）

> 对每条 `GateCertificate::Xzd(ev)` 记录 `lvl, side, source_index, confirm_index, cand_type,
> C2, C3`，并按 level 聚合。对 `lvl==1` 额外统计：
> - `last_zs_exists`：`sub_centers` 中落在 `s.start_index..=s.end_index` 的最后中枢是否存在。
> - `same_side_l0_type3_any`：as-of L0 BSP 中是否存在同向 Type3。
> - `same_center_any`：是否有任意 Type3 的 `q.center == last_zs`。
> - `same_side_same_center`：当前 C3 的真实命中断点。
> - `q.source_index <= confirm_index` 与 `q.source_index` 相对 `s.end_index` 的分布，判断是
>   时间确认问题还是 center 归属问题。
> - 对 `lvl>=2` 记录 `sub_bsp_type3_count`，预期恒为 0，作为死门真封。

---

## 6. 结果包六要素

1. **结论**：C3 硬门裁定 = 部分(b)（level>=2 约29%结构性死门，100%代码确定；level==1约71%
   无法单凭静态代码判定，需诊断探针）。处置 = `XzdEvidence::gate_pass()` 退化为仅C2，C3降为
   软标注，输出标注`C2-only xzd`。次要审计面1-3判正确，面4（Nest bit-exact）代码结构可信但
   未经dx harness实测验证。
2. **定义依据**：`053:28`（C2权威依据，二类点是小转大最佳确认手段）；思维导图`128`（C3=必要
   非充分条件）；`044:56`（动态最后次级中枢）；设计稿§6-3已预留"C3降为软标注,门退化仅C2"处置路径
   （本裁定与前审§6-3一致，非新增矛盾）。
3. **边界条件**：若后续诊断探针（§5.4）显示level==1子集`same_side_same_center`命中率显著>0
   （即level==1本身C3并非死门），则处置应改为"仅lvl>=2走C2-only，lvl==1保留C2∧C3硬门"的
   分级处置，而非本裁定的全域退化；这是本裁定唯一给出的翻转条件。
4. **下游推论**：`XzdEvidence::gate_pass()`需改为仅返回`type2_confirmed`；`sub_last_zs_type3`
   降级为诊断字段（不参与gate_pass）；114条Xzd信号全部通过（原C2=114/114已验证），下游
   `SpreadAttribution`/W-VERIFY(#13)口径需标注`C2-only xzd`前缀，避免声明C3必要条件已满足。
5. **谱系引用**：606号（区间套有效域=Type1）、673号（Cand^δ范围误用，同源结构问题——本次发现
   的"is_l0门控"缺口与673号是同一类"level塔空洞"模式的新实例，建议genealogist评估是否需要
   独立谱系条目或并入673号系列）。
6. **影响声明**：本文件为只读审计产出，零代码改动、零git操作。下游需要的代码改动（
   `xiaozhuanda_confirm`/`XzdEvidence::gate_pass`退化为仅C2 + C3字段降级为诊断态 + 新增诊断
   探针）由后续实装工位（建议新建task，metadata.agent_type=implementer）执行，本工位不代做。

---

## 7. 复算

```
codex exec --skip-git-repo-check --sandbox read-only - < /tmp/codex-xzd2-audit-prompt.md
```
（prompt原文已并入本文件§3-4上下文；完整原始transcript含工具调用noise，未随本文件归档，
如需逐字复核可重跑上述命令复现同等结论——裁定基于确定性代码结构分析，非随机采样。）
