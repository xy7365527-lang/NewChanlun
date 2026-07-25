# #218 owner 归属修正实装（路线 B + v2 教义修订）——实装说明

- 日期：2026-07-24 ｜ 票据：issue #218（wayfinder:task；parent map #126「塔侧跨级链增产」；spec #219 v2
  `chanlun/review-results/spec-owner-attribution-fix-20260724.md`；裁定文
  `chanlun/escalate/owner-attribution-carrier-anchor-ruling-20260724.md`）
- 工作面：worktree `/tmp/kimi-nest-mainline`（分支 kimi-nest-mainline-20260717）；git 零 mutation；改动叠加在未提交面上
- 验收状态：两接缝新测试全绿（构造器 3 新 + 主接缝 9 新）；既有测试处置完成（owner 相关机械改写逐个归因，
  非 owner 零改动）；wf7 单窗重放校验 25 项断言退出码 0（范围裁定 2026-07-24：p3fold/wf8 不跑）

## 1. 改动面（8 文件 + 产物）

| 文件 | 改动 | 性质 |
|---|---|---|
| `rust/src/theta_v0/classifier/bsp.rs` | `OwnerRef` 枚举新增（`Center(Center)` / `Type1Anchor(usize)`）；`BspPoint.center: Option<Center>` → `Option<OwnerRef>`；字段 doc 如实改写 | 面 A 载体形态 |
| `rust/src/theta_v0/classifier/signal.rs` | `make_second_point` 第 4 实参 c1 → 一类点锚坐标（`index_of(m1)`，m1 = 第一类离开走势终点 = 该走势终点极值点）；`make_first_point`/`make_third_point` 载 `OwnerRef::Center`；识别层（c1/i1/i2/背驰）一个 bit 不动；构造器接缝 3 新测试 + 2 既有载体断言机械改写（归因面 A） | 面 A |
| `rust/src/theta_v0/classifier/nest.rs` | `OwnerAnchorCtx`（anchor_at oracle + 事件两元锚）+ `terminal_bits_in_book(_core)`/`terminal_bits_at_event(_measured)` 签名扩（centers + ctx）；owner 判同换两族精确等值（一/三类核心区间 (zd,zg) 带判同；二类 anchor_at(一类点坐标) == (extreme_price, group_anchor)）；探针口径同步（`owner_anchor_missing_pts` 语义重定 = 锚不可解、`band_eq_start_neq_pts` 带碰撞观察）；核化触及处 doc 如实改写；主接缝 9 新测试 + 既有 owner 测试机械改写（逐个归因） | 面 B |
| `rust/src/theta_v0/classifier/nest_index.rs` | `build_nest_certificate_index` 加 oracle + 事件锚查找两入参（主接缝签名扩展，spec ID-2）；`NestEndorsementInstrument` 字段重命名（`trend_owner_anchor_neq`/`owner_anchor_missing_pts`）+ 带碰撞 + 遍历三计数（scan_book_total/in_window/same_side） | 面 B 装置 |
| `rust/src/theta_v0/classifier/projection.rs` | `anchor_resolver` 构造子（T1 供给线单一来源：fractal_at_source + merged_group_anchor，与本文件 :143-145 同一查法，禁第二查法） | 面 B oracle |
| `rust/src/theta_v0/backtest/admission.rs` | gate `anchor_by_id` 正查账本（absorb_exts 与 by_triple_anchor 同源写入，零新增解析）；`sync_index` 接线 oracle + 事件锚透传 | 面 B 接线 |
| `rust/src/theta_v0/backtest/fill.rs` | NEST_GATE_FAIL 行 schema 演进（#214 新增行族、非冻结红线面，随票登记）：桶名 owner_anchor_neq、owner_pts_anchor_missing、band_eq_start_neq、scan_pts 三计数 | 面 B 落账 |
| `rust/src/theta_v0/{strategy/mod.rs,strategy/coverage.rs,strategy/interp.rs,backtest/signal.rs,backtest/econ_positive.rs,backtest/runner.rs,classifier/turn_class.rs,classifier/mod.rs} + `tests/theta_v0_classifier_parity.rs` + 8 研究 bin（p92/p116/p117_s1a/p117_h4/p123/p124_shard/p124_merge/p125） | `OwnerRef` 载体形态机械适配（构造点包装 / match 臂 / 账本 centers 供给 / bin 签名同步 + 锚供给说明注释） | 消费面复核（spec ID-1 钉案逐点落票） |

注：**mod.rs 零改动**——spec ID-1「零新入参」成立：一类点锚坐标经既有 `index_of` 闭包供给（同 m2 坐标
取法），`extract_second_signals` 签名未变，调用点 `second_for_parent` 未变（任务提示中的「mod.rs 一处
实参」按 spec 钉案消解为 doc 一致性已查：mod.rs:2234-2239 对 c1 的既有描述（识别用判定中枢）与新
归属语义无冲突，无需改写）。

## 2. 两族判同结构（面 B 核心，spec ID-2）

```text
terminal_bits_in_book_core（私有核，#214 结构不动：单次遍历共核产判定+探针）
  Trend 臂 per 点 owner 判同（全部精确等值无容差，v3 硬禁令；零序号判同）：
  ├─ OwnerRef::Center(pc)（一/三类载体）：
  │    B = centers.find(start_index == b_center_start)（序号只当查找键，不当身份）
  │    pc.(zd,zg) == B.(zd,zg)  ⟹ 判等（带等且序号不等 ⟹ band_eq_start_neq_pts 碰撞观察）
  │    B 查无 ⟹ 锚不可解（生产不可达防御，诚实判负）
  └─ OwnerRef::Type1Anchor(src)（二类载体，面 A 改载该走势一类点锚）：
       anchor_at(src) == (event.extreme_price, event.group_anchor)
       anchor_at = projection::anchor_resolver（T1 供给线单一来源）
       事件锚 = gate anchor_by_id 透传（NestCandidateEventExt 已带，零新增解析）
       任一侧 None ⟹ 锚不可解（身份合取不可证，诚实判负 + owner_anchor_missing_pts）
```

- 判负入「owner 锚不等」桶（穿透序 窗口 → 方向 → owner 不变；完备性平账保留）。
- `Exact` 臂（C-a 诊断常数）、窗口 `[c_start, t*]`、方向 `confirm_side`、最早性 `min_by_key`、
  点类分域、`turn_source` 语义——逐字保留（spec ID-2「不动」清单）。
- `TerminalEndorsement.owner_center_start` 保留（归因快照，非判定输入）：Center 载体取
  start_index；Type1Anchor 载体如实 None（doc 已如实改写）。

## 3. 验收证据（测试）

- 构造器接缝 `extract_second_signals`（面 A）：`cargo test --lib theta_v0::classifier::signal::tests::second`
  = **8 passed / 0 failed**（3 新：`second_buy_owner_carrier_is_type1_anchor`（载体 == 一类点锚 +
  识别不变断言）、`second_sell_owner_carrier_is_type1_anchor`（卖侧镜像）、
  `second_owner_carrier_independent_of_judge_center_c1`（归属/确认分层：换 c1′ 识别结果与锚不变）；
  2 既有载体断言机械改写归因面 A；3 拒收测试零改动）。
- 主接缝 `build_nest_certificate_index`（面 B）：`cargo test --lib theta_v0::classifier::nest`
  = **84 passed / 0 failed**（62 既有 + 12 i214 + 10 i218 新：序号漂移双向用例（同带异序号判等 /
  同序号异带判负）、二类锚判等/判负/锚不可解子计数（点侧 + 事件侧）、B 查无防御、四桶新口径
  完备性平账 + 点级计数、非 owner 不变量（异向/窗口外/Pan 逐值一致）、带碰撞观察、Exact 臂不动、b_center_start=None 两族 fail-closed 合一（codex 评审采纳）。
- 既有测试处置：owner 相关单测按新口径机械改写并逐个归因（signal.rs 2 处载体断言 → 面 A；
  nest.rs c_window 6 处 + i214 12 处 → 面 B；runner.rs gate 夹具 centers 供给 3 处 → 面 B；
  `c_window_trend_owner_extension_start_index_judges` → 语义重写为
  `c_window_trend_owner_band_judges_over_start_index`（旧名注释引述，判同机制 start_index → 带判同）；
  **非 owner 测试（Pan 域/窗口/方向/Exact 臂）零改动通过**（签名机械适配 ctx 入参不计语义改动）。
- 全量 `cargo test --lib` = **1802 passed / 1 failed**；唯一失败
  `extract_signals_bit_exact_digest_guard` 为 #110 工作线既线失败在案（勿修勿归因；
  面 A 二次漂移登记见读数报告 §6 GOLDEN 面）。`cargo check --bins` 0 错误；集成测试
  `tests/theta_v0_classifier_parity.rs` 机械适配后随 release 构建通过。
- wf7 重放校验：`python3 chanlun/review-results/owner-attribution-fix-verify-20260724.py`
  = **25 项断言退出码 0**（留档 `owner-attribution-fix-verify-output-20260724.txt`）。

## 4. 消费面逐点复核落票（spec ID-1 钉案：`BspPoint.center` 既有消费面受触复核）

| 消费点 | 处置 | 归因 |
|---|---|---|
| `make_first/third_point`（signal.rs） | `OwnerRef::Center(*c)` 包装，语义不动 | 面 A 形态 |
| `make_second_point`（signal.rs） | 改载 `OwnerRef::Type1Anchor(index_of(m1))` | 面 A 本裁 |
| nest.rs owner 判定/探针/快照 | 两族判同重写（§2） | 面 B 本裁 |
| `strategy/mod.rs` ×2（StopInput 构造） | Center 变体读出；Type1Anchor+3 类 bit = 不变量违反显式拒（生产不可达防御）；Type1Anchor 无 3 类 bit 零占位（1/2 类止损只读 pivot，语义不变） | 消费面复核 |
| `backtest/signal.rs`（structural_stop 构造） | 同上口径 | 消费面复核 |
| `turn_class.rs:158`（XZD 要件 2 三类点命中，全字段等式照旧） | `OwnerRef::Center` 包装 | 消费面复核 |
| `econ_positive.rs:1357/1394`（XZD C3 死门双序号判同，#44 证伪在案的退役判据） | Center 变体 matches! 适配，语义不动 | 消费面复核 |
| `runner.rs:5705`（`point.center.is_none()`） | Option 方法，零改动 | 消费面复核 |
| 测试/夹具构造点（bsp.rs/signal.rs/mod.rs/coverage.rs/interp.rs/strategy/mod.rs/runner.rs/turn_class.rs + parity 集成测试） | `OwnerRef::Center(...)` 机械包装，断言逐值不变 | 面 A 形态 |
| 研究 bin ×8（p92/p116/p117_s1a/p117_h4/p123/p124_shard/p124_merge/p125） | 签名同步（centers 真供给 + `bin_anchor_ctx()` 锚供给说明：归档研究 bin 未接事件锚账本，二类判同锚不可解诚实判负；一/三类带判同全功能） | 消费面复核 |

## 5. 090 声明=能力（doc 如实改写清单）

- bsp.rs `center` 字段 doc、signal.rs `make_second_point`/`extract_second_signals`/c1 doc、
  nest.rs `TerminalMatch::CWindow`/`terminal_bits_in_book(_core)`/`terminal_bits_at_event(_measured)`/
  `TerminalEndorsement.owner_center_start`/`EndorsementProbe`/`OwnerAnchorCtx` doc、
  nest_index.rs 装置字段 doc、level_view.rs 不重写（b_center_start 字段语义未变——它本来就是
  「B 的 start_index 快照」，新判定把它降级为查找键，字段 doc :499-501 已核：其「判同基准」
  表述指向 nest.rs 判定式，nest.rs 侧 doc 已同步）。
- 未验证项（如实）：①`Exact` 臂探针零值仅单测覆盖（生产不走 Exact，#214 同）；②中枢参照
  B 查无防御臂生产不可达（合成测试覆盖）；③遍历计数 scan_pts 为 #218 新字段，#214 基线无
  对照面（不变性由 c_i total 逐项对照 + 谓词共核论证承担）；④p3fold/wf8 未跑（范围裁定
  2026-07-24，如实标）；⑤研究 bin 二类判同锚不可解降级（§4 表末行，面 B 注册项）。

## 6. 复算入口

- scoped 测试：`cargo test --lib theta_v0::classifier::signal::tests::second`、
  `cargo test --lib theta_v0::classifier::nest`。
- wf7 重放：`THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=<dir>/opsem T5A_CHAIN_DUMP_DIR=<dir>/dump
  M8_WIN_FILTER=wf7 <release test bin> --ignored --nocapture --exact
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos`。
- 校验：`python3 chanlun/review-results/owner-attribution-fix-verify-20260724.py`
  （默认 /tmp/nest214/after vs /tmp/nest218/after，wf7 单窗；25 项断言退出码 0）。
- 配套档：读数对账报告 `owner-attribution-fix-readings-20260724.md`（含逐事件双向翻转
  清单与 GOLDEN 登记）；两轴 code-review 结论 `owner-attribution-fix-code-review-20260724.md`。
