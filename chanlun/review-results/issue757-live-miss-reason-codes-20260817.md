# #757 交付：批量丢弃机制不可观测实例——专门原因码正式化（#617 方向 A，#724 裁定）

> 角色：实施工蜂（sandcastle 沙盒，分支 `sandcastle/issue-757`，基线 main `5217e7aee4`）
> 票据：#757（#724 裁定 2026-07-29 的唯一投入点；机制钉界 #617；时机边界 #618；三桶实测 #603）
> 纪律：只动观测面，不进判据路径；090 照实。

## 0. 结论摘要

1. **原因码 schema 定稿并落码**（§1 spec，裁定点呈编排者）：完成身份「无 earlier Live」
   三类成因码 `LiveMissCause` = `chain_confirmed_no_pending_window`（机制不可观测）/
   `same_bar_center_confirmation`（时机边界）/ `observable_window_missed`（可观测未命中），
   判定顺序 = 枚举顺序，唯一分类点 `BatchObservabilityTracker::classify_l1`。
2. **落码全部在观测面**：塔层批量丢弃实例（`scan_active_window` 批内非末窗）经新增出参
   逐只落行；L1 身份级三类分级在 targeted pass 收尾逐身份落行 + stderr 汇总。**既有
   stdout / P421_LIFECYCLE_DUMP / P116_DUMP 三面零扰动**——产出物走新增独立 sink
   `P757_OBSERVABILITY_DUMP`（env 未设零写出），汇总走 stderr 新增两行审计行
   （不在 golden 内，#724 A4 同口径）。cmp=0 护栏由「只新增写入点、不改写既有写入点」
   结构性保证。
3. **验证**：`cargo check --all-targets` 过；`cargo clippy` 零新增告警（205 == 基线 205，
   逐条比对）；`cargo fmt --check` 过；`cargo test --lib` 2719 绿（含新增 3 只 #757 测试）；
   `--all-targets` 唯一红 = `signal_extraction_emits_no_second_class_only`，**基线同红**
   （stash 复验，与本票无关，在案登记）。
4. **090 限度**：本机无 `analysis/data_cache/btc_1m_full.json`（.gitignore 排除），**BTC 100k
   验收跑（18/17/11 对拍 + 三窗 cmp=0）无法本地执行**，跑法与判据已备（§4.3），留编排层
   重型验证窗口。

## 1. spec：原因码 schema（裁定点呈编排者）

### 1.1 三类成因码

| 码（`reason_tag`） | 分级 | 定义（判据） | 教义/实测锚 |
|---|---|---|---|
| `chain_confirmed_no_pending_window` | 机制不可观测 | C 段**从未**作为 pending/frontier 被观测到（采样点 = `active_segment_frontier` 的 `Some` 返回，与 #603 桶①探针口径逐字同源） | #617：批量确认批内非首段的「当下状态」在任何粒度下不存在——非观测缺口，是状态本身不存在；BTC 100k 实测 18 只 |
| `same_bar_center_confirmation` | 时机边界 | C 曾 pending 可观测，但 B 中枢到**完成信号同 bar** 才首次 sealed（sealed 前缀水线 = `tower_confirmed_len`，#93 单一来源）；覆盖率判据严格 `<` 天然排除同 bar 命中 | #618：判据边界产物，非数据分裂；实测 17 只 |
| `observable_window_missed` | 可观测未命中 | C 曾 pending 且 B 在完成前已 sealed——实例本可观测，未覆盖归因他处（在案主因 `structure_not_locatable`，#599 §6 已判出范围） | #603 桶③实测 11 只 |

**判定顺序 = 上表顺序**：机制不可观测优先（三者中唯一的教义终局判定），时机边界次之，
可观测未命中兜底。覆盖（有 earlier Live，`observed_at < signal_at`）的身份不落码——
#597 destination 口径的第一支路径，不属本 schema。

### 1.2 分级口径细节（与既有实测对齐的设计决定）

- **分母**：账本内 level==1、`structure_end_at` 有值的 pan 域（`Consolidation`）条目。
  trend 域条目（`seg_c_full` 为工程桥口径）记 `out_of_scope`，不参与分级。
- **「曾 pending」采样点取 `active_segment_frontier`** 而非 `IncrSegments` 新增字段：
  #724 报告建议的前置实装点是「L1 段层新增『本次 append push 几段』字段」，本交付**偏离
  该建议**，理由两条：① #603 桶①的实测口径是「从未进过队列」，队列喂数点正是
  `active_segment_frontier`——同点采样使 18 只读数可直接对拍，零口径换算；② 不动 parser
  任何结构（连观测字段也不加），把「既有行为零变化」从「新增字段不参与判据」降级为
  「生产结构体一个 bit 不动」，护栏更强。**此偏离呈编排者裁定**；若裁定仍要段层字段，
  可后续小票补（两口径可互证）。
- **sealed 首见只数 sealed 前缀**：frontier 批内（`last_window_emitted` 覆盖的开放单元）
  不算「确认」——与 #618「首次被确认」同口径。水线回缩（cascade bar）后重新 sealed 的
  元素首见 bar 保原值（首确认是历史事实，首写不后移；R7 测试锁定）。
- **级别范围**：身份级三类分级只落 L1（C 腿 pending 口径的适用域）。L2/L3 身份的 C 是
  窗口单元，其机制不可观测面由塔层批量丢弃记录承载；身份级分级不在本票 Scope
  （#724 V3：L4+ 若立项须第一天内建同级别口径）。
- **塔层批量丢弃实例**：`scan_active_window` 一次重扫产出 m ≥ 2 窗口时，批内非末窗
  （m−1 只）逐只记录源坐标区间（`BatchDroppedWindow`），与末窗结局（是否吸收虚拟单元）
  无关——「从未作为当下行进中窗口存在」的事实不随结局改变。计数口径：
  `instances` = Σ(m−1)，`rescans` = 含 ≥1 只丢弃的重扫事件数（对齐 #602 §3.2 的
  Σ(m−1)=236 / 71 批量重扫口径，可复算对拍）。

## 2. 落码面（观测面 only，逐面登记）

| 面 | 载体 | 开关 | 内容 |
|---|---|---|---|
| 塔层实例逐只 | `P757_BATCH_DROP as_of= level= cause=chain_confirmed_no_pending_window outcome= win_start= win_end=` | env `P757_OBSERVABILITY_DUMP` | 每只被丢窗口一行；`outcome` = 末窗结局码（`active_window`/`lower_frontier_not_absorbed`） |
| L1 身份逐只 | `P757_LIVE_MISS level=1 cause= seg_a=(,) c_start= b_center_start= observed_at= signal_at=` | 同上 | 每只无 earlier Live 的 L1 完成身份一行 |
| stderr 汇总 | `P757_LIVE_MISS_SUMMARY l1:covered= l1:uncovered= l1:<三码>= l1:out_of_scope=` | 恒打（新增行） | 验收直读：三码应读出 18/17/11 |
| stderr 汇总 | `P757_BATCH_DROP_SUMMARY l2:rescans= l2:instances= l3:…` | 恒打（新增行） | 对齐 #602 Σ(m−1) 口径 |

库侧 API（`nest_lifecycle.rs`）：`LiveMissCause` / `BatchDroppedWindow` /
`BatchObservabilityTracker`（`observe_frontier` / `observe_tower_level` / `classify_l1`）；
`scan_active_window` 与 `active_l1/l2_window_frontier` 新增 `drops` 出参（调用点已全部
跟进，含测试夹具）。p123 侧：跟踪器逐 bar 喂数（`keys.frontier` 每 bar 已算，零新增
成本；sealed 前缀增量扫描 O(新 sealed 数)/bar）。

**不进判据路径的结构性证据**：无任何既有写入点被改写——账本字段/修订链/事件流/塔与
parser 产出/dump 行格式全部原样；新增类型只被 p123 观测面与自身测试消费。

## 3. 验证

- `cargo check --all-targets`：0 error。
- `cargo clippy --all-targets`：205 警告 == 基线 205（stash 逐条比对，零新增；本票曾引入
  一条 `level used to index tower`，已改写为迭代器消除）。
- `cargo fmt --check`：过。
- `cargo test --lib`：2719 绿 / 0 红（含本票新增 R5 批量丢弃 m=2 落码、R6 三类分级、
  R7 sealed 首见回缩 3 只）。
- `cargo test --bin p123_fast_replay`：13 绿。
- `cargo test --all-targets`：唯一红 `signal_extraction_emits_no_second_class_only`
  （tests/theta_v0_classifier_parity.rs:673「产第三类（离开后回试不破 ZG）」）——
  **stash 复验基线同红**，与本票无关，在案登记不伪造。
- 既有行为零变化：结构性论证（§2 末行）+ 全量测试绿；字节级 cmp=0 属重型窗口（§4.3）。

## 4. 验收口径（呈编排层）

### 4.1 票面三条对照

1. **L1 身份级 18 只可分档读出**：`P757_LIVE_MISS_SUMMARY` 的
   `l1:chain_confirmed_no_pending_window=` 直读；逐只清单在 `P757_LIVE_MISS` 行。
   预期值 18（#603 桶①同口径）。**本机未实测**（§4.3）。
2. **塔层批量丢弃实例带码可查**：`P757_BATCH_DROP` 逐只带 `cause=` 码；
   `P757_BATCH_DROP_SUMMARY` 给 Σ(m−1) 口径计数（#602 复算值 236 / 71 可对拍，注意其
   91 vs 236 悬案在案——#724 G3，本读数按其复算口径，不介入悬案）。
3. **既有行为零变化（cmp=0 护栏）**：结构性保证（§2）；字节级对拍见 §4.3 跑法。

### 4.2 spec 裁定点（呈编排者）

- §1.1 三码命名与定义（`chain_confirmed_no_pending_window` 沿用 #617 建议名）。
- §1.2 对 #724 前置实装点（`IncrSegments` 字段）的**偏离**及其两条理由。
- §1.2 级别范围限定（身份级分级只落 L1）。

### 4.3 重型验证窗口跑法（本机无数据，留编排层）

```sh
# BTC 100k 单窗（对齐 #602/#603 原窗），cargo test --release --manifest-path rust/Cargo.toml #   --test issue533_p123_byte_guardrail -- --ignored --nocapture   # 三面 cmp=0 护栏
# 观测面读数（同一数据）：
P116_MAX_BARS=100000 P757_OBSERVABILITY_DUMP=/tmp/p757.dump   cargo run --release --bin p123_fast_replay -- analysis/data_cache/btc_1m_full.json 2>&1   | grep -E "P757_LIVE_MISS_SUMMARY|P757_BATCH_DROP_SUMMARY"
# 预期：l1:chain_confirmed_no_pending_window=18；l3:instances 与 #602 复算 236 同口径对拍。
```

若三码读数 ≠ 18/17/11：可能偏差源已在案——① #618 对 17 只的归因措辞是「**常在**同 bar
首次被确认」（非逐只钉死），个位数偏差属既有归因的未钉部分；② #88 级联复活使段在
完成后再度 pending 的极端形态会让 `ever_pending` 偏宽（机制码偏少），两偏差都在
分类点文档登记，**不凑数**。

## 5. 090 照实声明

- 本机无 BTC 数据文件，**未跑任何真实数据回放**；18/17/11 与 cmp=0 均未本地实测，
  全部验收读数留 §4.3 窗口。本地验证 = 编译/静态/合成夹具测试三层。
- 三码定义经合成夹具单元测试锁定（R5/R6/R7），但合成夹具不证明生产数据上的计数。
- `LeveledMove.start_index`（塔层）与身份键 `b_center_start`（level_view 快照口径）的
  坐标同一性依据 #618「两路径构造算法逐字相同、同读 tower[level]」结论，未经本票
  独立实测复证；若该同一性在个别形态破裂，时机边界码会偏少、可观测未命中码偏多
  （误差方向已登记）。
- 未触碰 `.chanlun/definitions/`——本 schema 是工程观测面原因码，非缠论概念教义裁定。

## 6. 结果包六要素

1. **结论**：三类原因码 schema 定稿并落码（观测面 only）；既有三面零扰动由结构性
   保证；本地静态+合成测试全绿（唯一红为基线既有）；BTC 验收跑留编排层窗口。
2. **定义依据**：`rust/src/theta_v0/classifier/nest_lifecycle.rs`（`LiveMissCause` /
   `BatchObservabilityTracker` / `BatchDroppedWindow` / `scan_active_window` 出参）；
   `rust/src/bin/p123_fast_replay.rs`（喂数点 / `classify_l1_live_misses` /
   `write_p757_batch_drop_rows` / 两行 stderr 摘要）。票据依据：#724 裁定、#617 §5
   方向 A、#618 时机钉界、#603 三桶口径。
3. **边界条件（结论何时翻转）**：(a) 编排者否决 §1.2 的 `IncrSegments` 字段偏离
   ⟹ 补段层字段小票，两口径互证；(b) BTC 验收三码 ≠ 18/17/11 且偏差超出 §4.3 在案
   偏差源 ⟹ 口径复查（先查 `b_center_start` 坐标同一性）；(c) #724 G1 若实测出
   「跨 bar 真实缺口」非零占比 ⟹ 方向 C（OpenTail 修订）上呈，本 schema 不变；
   (d) 换标的/换窗须重测（全部读数为 BTC 100k 单窗 L2 级证据，#724 V4）。
4. **下游推论**：V1（map #597 destination 验收重跑不再依赖已删探针——原因码落账本
   观测面）与 A2（#617 建议码结束「只活在研究报告」状态）随本票闭环；归因脚本
   参数化（#724 建议 5）与 G1/G2 探针票不变、仍待立。
5. **谱系引用**：#523（观测接缝根因）；#527/#559（活窗与两类消失原因码先例——本 schema
   是同层级的第三类，不并入 `VanishCause`：消失原因码管「曾建仓后消失」，本码管
   「完成身份从未有 earlier Live」，两者判据对象不同、不互通）；#597（destination
   口径）；#599/#603（三桶实测与探针口径）；#602（塔层 Σ(m−1) 口径）；#617（机制
   钉界 + 方向 A + 建议码名）；#618（时机边界钉界）；#629（Σ(m−1) 复算）；#724
   （裁定与九面枚举）。
6. **影响声明**：改动文件 = `rust/src/theta_v0/classifier/nest_lifecycle.rs`、
   `rust/src/bin/p123_fast_replay.rs`（新增类型/出参/观测面落行，无既有写入点改写）、
   本报告（新增）。单 commit 落 `sandcastle/issue-757`；不关票、不 merge、不 push、
   不动 main。
