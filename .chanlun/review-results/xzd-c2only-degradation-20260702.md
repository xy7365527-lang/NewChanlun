# 小转大 C3 硬门退化实装：gate_pass 仅 C2 + C3 诊断降级 + level==1 探针实测（task #41）

- task: #41（ws-c2only 工位）
- date: 2026-07-02
- 依据：codex 终局裁定 `.chanlun/review-results/xzd2-impl-codex-audit-20260702.md` §5.2/§5.4
- 认识论等级：**L2**（真实 BTC 350K bar 逐信号统计，探针实测，非合成数据）

---

## 1. 代码改动（`rust/src/theta_v0/backtest/econ_positive.rs`，工作树 WIP，未 commit）

### 1.1 `XzdEvidence::gate_pass()` 退化为仅 C2

```rust
pub(super) fn gate_pass(&self) -> bool {
    self.type2_confirmed
}
```

`sub_last_zs_type3`（=C3）保留字段但不再参与 `gate_pass`，文档标注「诊断字段，不参与 gate_pass」。

### 1.2 新增 4 项 C3 死门诊断字段（`XzdEvidence` 结构体 + `xzd_c3_diag` 辅助函数）

`last_zs_exists` / `same_side_l0_type3_any` / `same_center_any` / `same_side_causal_ok` /
`sub_bsp_type3_count`——由 `xiaozhuanda_confirm` 在构造 evidence 时一并算出（与 `type2_confirmed`/
`sub_last_zs_type3` 同源同快照，无额外前视风险）。`same_side_causal_ok` 是本次新增的分裂断点，
用于区分「时间确认问题」（同侧 Type3 只存在于 confirm_index 之后）vs「center 归属问题」（因果窗口
内存在但 center 不匹配 last_zs）——回应审计代理 §3.4 的开放疑点。

### 1.3 诊断探针接入 `acc_classification_level_hole_dx` 测试（`#[ignore]`，L2 真实数据）

按 level 聚合 `xzd_routed_by_level`/`xzd_c2_by_level`/`xzd_c3_by_level`；lvl==1 单独统计 4 项诊断
字段命中数；lvl>=2 累加 `sub_bsp_type3_count`（死门真封，预期恒 0，非零则 `assert_eq!` 炸）。报告
新增两个 section：「C3 死门诊断探针」+「lvl==1 子集分裂断点」，落盘
`.chanlun/review-results/acc-classification-level-hole-20260701.md`。

### 1.4 报告标签：`C2-only xzd`

「小转大通道命中」section 标题与正文改为 `C2-only xzd`，不再声称 C3 已满足；`n_xzd_pass` 语义从
「C2∧C3 通过」改为「C2 成立即通过」。

### 1.5 测试更新

`xzd_gate_pass_is_c2_and_c3` → 重命名 `xzd_gate_pass_is_c2_only`，断言改为「C2 真 C3 假仍通过」
（gate_pass 退化的直接见证）；`xzd_c3_last_sublevel_zs_type3_only`（纯函数单测）不变——它测的是
`xzd_sub_last_zs_type3` 这个诊断函数本身的正确性，与是否参门无关。

---

## 2. L2 实测（350K bar 真实 BTC，`ECON_L2_MAX_BARS=350000 cargo test --release
acc_classification_level_hole_dx -- --ignored --nocapture`）

### 2.1 真封（守恒）

```
真封：sig_post_sum=976 >= n_signals=972 = decomp_sum=972
真封③（P1+小转大）：depth_hist_sum=862+n_xzd_pass=114 = n_gate_pass_total=976 = sig_post_sum=976
```

- **区间套 Nest 通道 862 条逐字不变**（depth 直方图和=862，与 #33 审计前基线一致）——C2-only 改动
  零污染区间套通道（`GateCertificate::Nest` 分支未被触碰，`build_nest_certificate`/`n_delta` 不动）。
- **小转大 Xzd 通道通过数从 0 → 114**：与团队预期完全一致（原 C2=114/114、C3=0/114，C2-only 后
  114 条全部通过门，进入信号集/level 分布/后续配对）。

另跑默认 300K 窗（`cargo test ... acc_classification_level_hole_dx -- --ignored --nocapture`，无
`ECON_L2_MAX_BARS`）：触发 `signals_dx == collect_signals(&ds,&config)` 的 bit-exact 交叉核对
（350K 窗因 > `MAX_BARS_DEFAULT`(300_000) 跳过该核对，属既有 perf-tier 设计，非本次改动引入）。
300K 窗测试同样 `ok`——生产 `collect_signals` 路径（消费 `ev.gate_pass()`）与 dx 手写门逐元组
bit-exact 一致，证明 gate_pass 退化正确传导到生产路径，非仅 dx 侧改动。

### 2.2 C3 死门诊断探针分级别统计（350K 窗，routed=Xzd 域触达数）

| level | Xzd routed | C2 成立 | C3 成立(same_side_same_center) | C3 命中率 |
|---|---|---|---|---|
| 1 | 84 | 84 | 0 | 0.00% |
| 2 | 22 | 22 | 0 | 0.00% |
| 3 | 5 | 5 | 0 | 0.00% |
| 4 | 2 | 2 | 0 | 0.00% |
| 5 | 1 | 1 | 0 | 0.00% |

### 2.3 lvl==1 子集分裂断点（裁定 §5.4 精确规格，routed=84）

| 诊断字段 | 命中数 | 命中率 |
|---|---|---|
| last_zs_exists | 84 | 100.00% |
| same_side_l0_type3_any | 83 | 98.81% |
| same_center_any | 0 | 0.00% |
| same_side_causal_ok | 83 | 98.81% |
| same_side_same_center（C3 真实判据） | 0 | 0.00% |

**读解（本窗如实报告，非预设结论）**：s 跨度内几乎总有次级中枢（100%），同侧 L0 Type3 点几乎总
存在（98.81%）且几乎总在因果窗口内（`source_index<=confirm_index` 98.81%，与 `same_side_l0_type3_any`
几乎重合，说明**不是时间确认问题**——Type3 点在确认时刻已可见）。但这些 Type3 点的 `center` 从未
等于 `last_zs`（`same_center_any=0%`）——**是纯粹的 center 归属问题**：`judge_third` 按 leave 段
当下最近中枢归属 `center`，`last_zs` 按 `s` 空间跨度过滤取最后中枢，两条选择链在本窗 BTC 1m 几何
下**结构性不重合**（呼应审计代理 §3.4 的开放疑点，本探针给出确定性答案：几乎从不重合）。

### 2.4 lvl>=2 死门真封

```
lvl>=2 routed=30，sub_bsp Type3 点总数=0
真封通过：lvl>=2 结构性死门坐实（sub_bsp_type3_count=0），与 codex §5.1 静态代码分析一致。
```

---

## 3. 单元测试

`cargo test --release theta_v0::backtest::econ_positive`（含新增/改名的 `xzd_gate_pass_is_c2_only`）：
30 passed / 0 failed / 7 ignored（ignored 为需真实 BTC 数据的 L2 测试，已单独用 `--ignored` 跑过，
见 §2）。`cargo build --release` 全绿，无新增警告。

---

## 4. 结果包六要素

1. **结论**：`XzdEvidence::gate_pass()` 退化为仅 `type2_confirmed`（C3 硬门移除）；`sub_last_zs_type3`
   降级为诊断字段；新增 4 项诊断字段（`last_zs_exists`/`same_side_l0_type3_any`/`same_center_any`/
   `same_side_causal_ok`）+ `sub_bsp_type3_count`；诊断探针接入 `acc_classification_level_hole_dx`
   测试，350K 真实 BTC 实测：Nest 862 条逐字不变，Xzd 通过数 0→114，报告标签统一为 `C2-only xzd`。
   探针实测显示 **level==1 子集 `same_side_same_center` 命中率为 0/84（0%）**——不满足裁定 §边界
   条件的翻转阈值（"显著 >0"），**全域 C2-only 退化处置在本窗成立，无需分级处置**。同时探针额外发现
   level==1 死门的**根因是 center 归属不重合**（而非时间确认窗口截断），比裁定原文的"需诊断探针
   另裁"更进一步，给出了确定性归因。
2. **定义依据**：codex 终局裁定 `.chanlun/review-results/xzd2-impl-codex-audit-20260702.md` §5.2
   （处置建议：gate_pass 退化仅 C2，C3 降软标注，输出标 `C2-only xzd`）+ §5.4（诊断探针精确规格：
   按 level 聚合，lvl==1 四项诊断，lvl>=2 死门真封）。
3. **边界条件**：本裁决的唯一翻转条件——若探针显示 level==1 `same_side_same_center` 命中率显著
   >0——**本窗实测为 0%，翻转条件不成立**。若未来数据窗（如全历史 4.6M bar 或其他标的）跑出
   level==1 `same_side_same_center` 显著 >0，需重新评估是否改为分级处置（lvl>=2 C2-only、lvl==1
   保留 C2∧C3），本次改动不预先关闭该路径（`same_side_same_center` 判据函数
   `xzd_sub_last_zs_type3` 与诊断字段均保留在代码中，未删除，重新启用只需改 `gate_pass()` 读取
   逻辑）。
4. **下游推论**：小转大域 114 条信号（原全 0）现全部进入信号集/level 分布/配对/W-VERIFY(#13) 待评估
   池，口径为 `C2-only xzd`——下游任何消费 Xzd 门控结果的分析（`SpreadAttribution`、level 分布报告等）
   须延续该标注，不得声明 C3 必要条件已满足。W-VERIFY(#13) alpha 全量重测的输入信号集因此发生变化
   （+114 条小转大信号，此前该通道恒为空），#13 执行时需感知这一变化。
5. **谱系引用**：606号（区间套有效域=Type1）、673号（Cand^δ 三分拆，同源"level 塔空洞"模式）——
   本次 level==1 center 归属不重合的确定性发现建议 genealogist 评估是否需要独立谱系条目（"C3 判据
   两条 center 选择链结构性不重合"，与 673 号系列可能是同一类模式的第三实例）。
6. **影响声明**：改动仅限 `rust/src/theta_v0/backtest/econ_positive.rs`（`XzdEvidence`/
   `xiaozhuanda_confirm`/`xzd_c3_diag`/`gate_pass`/测试/`acc_classification_level_hole_dx` 报告
   生成逻辑）；未触碰 `classifier/mod.rs`、`recursive_tower.rs`、`incremental.rs`、`strategy/`
   （并发工位域）；未做 git 操作（改动叠加在现有工作树 WIP 之上，未 commit）；
   `build_nest_certificate`/`n_delta`（区间套 bit-exact）逐字未动，350K 实测 862 条验证保留。
