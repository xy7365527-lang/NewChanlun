# wf8 金标准漂移二分归因报告（779 → 1,097 行）

- 日期：2026-07-28
- 性质：**纯调研，未改仓、未重订任何锚**（`/tmp/vocab_align_dump` 只读，未触碰）
- 结论一句话：翻转 commit = **`c8e87c4a0d`（#572 π 级联止损修复）**，单跳、区间内唯一；是**有裁定授权的合法行为修复**，但**漏做 ADR-0001 常例要求的基线重订登记**。

---

## 0. 统计口径行

- 复跑命令（三点完全同一条，仅工位与 target 目录不同）：
  ```
  M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=<per-point> \
  CARGO_TARGET_DIR=/tmp/bisect-target-<tag> \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
  ```
- 工位：每点独立 `git worktree add --detach /tmp/gdb-<tag> <tag>`，**全新建、零未提交改动**；
  `analysis/data_cache` 软链主仓（`data_dir()` 走 `CARGO_MANIFEST_DIR` 的父目录，worktree 内必须挂）。
- 数据：`analysis/data_cache/btc_1m_full.json`（329,485,099 B，mtime 2026-06-25，三点同一份物理文件）。
- 窗口：`M8_WIN_FILTER=wf8` ⟹ 单窗 `BTC wf8 test=2023-08-17..2024-02-16`（264,960 bar），三点日志一致。
- 每点独立 `CARGO_TARGET_DIR`，无 cargo 缓存串味；三点均 `test result: ok. 1 passed`（退出码 0）。
- 比对量：`trades.jsonl` 行数 / 字节数 / SHA-256 + `cmp` 逐字节；`tower_events.jsonl` 同。
- 主工作区当时脏（`rust/src/bin/p107_level_calib.rs` 等有未提交改动），**因此全程未在主工作区跑**——
  遵 ADR-0001 line 202 的流程教训（混合 lib 污染读数）。

---

## 1. 二分结果：一跳命中，无需继续二分

| 点 | commit | trades 行 | trades 字节 | trades SHA-256 | tower 字节 |
|---|---|---|---|---|---|
| P0（锚代码态） | `155a5db845` | **779** | **1,163,427** | `06243bac…f9b9` | 354,946 |
| P1（嫌疑首位） | `c8e87c4a0d` | **1,097** | **1,678,751** | `5d90baea…eecc` | 354,946 |
| P2（HEAD） | `8ed3572c02` | **1,097** | **1,678,751** | `5d90baea…eecc` | 354,946 |

逐字节校验（`cmp` 全部返回 0）：

| 比对 | 结果 |
|---|---|
| P0 trades vs `/tmp/vocab_align_dump/trades.jsonl` | **BIT-EXACT** |
| P0 tower vs 锚 tower | BIT-EXACT |
| P1 tower vs 锚 tower | BIT-EXACT |
| P2 tower vs 锚 tower | BIT-EXACT |
| P1 trades vs P2 trades | **BIT-EXACT** |

**两条推论**：

1. **锚可复现**。`155a5db845` 干净态复跑，`trades.jsonl` 与 `/tmp/vocab_align_dump/trades.jsonl`
   **逐字节相同**，SHA `06243bac…f9b9` 与 ADR-0001 line 206 登记的 #542 重订值一致。
   锚不是陈旧伪影，是那个代码态的真实、可重放产物。
2. **翻转唯一**。`c8e87c4a0d` 与 HEAD 的 `trades.jsonl` 逐字节相同 ⟹ 该 commit 之后到 HEAD
   之间的全部 commit（`06689c71a1` / `fa0dcb5882` / `61ed615393` / `a5cf2479d5` / `393a07c488` /
   `e50e34d50b` / `115c8166e7` / `76b7af7cb7` 等）在 wf8 关臂上**端到端 bit-exact 中性**，
   它们各自"纯测试 / 行为零变化"的自称由此获端到端旁证（含 `76b7af7cb7` #594 那笔
   compose/step/fill/shadow 生产面改动）。

嫌疑面中 `61ed615393`（#558，纯 Python）与全部 docs-only commit 已经 `git show --stat` 自证排除，
未凭 message 猜测。

---

## 2. 漂移不是"多 318 行"，是分布重构

`exit_type` 分布（括号内 = 其中 `via_structural_prune=true` 的条数）：

| exit_type | P0 `155a5db845`（779） | P1/HEAD（1,097） |
|---|---|---|
| **RiskExit** | **0** | **776**（prune 0） |
| CloseRoot | 366（prune 302） | 208（prune 196） |
| CloseReverseOpen | 240（prune 139） | 109（prune 105） |
| ReduceCore | 171 | 4 |
| Hold | 2 | 0 |

锚的 779 行里 **`RiskExit` 一条都没有**。这正是 #564 诊断的断链在成交账本上的投影：
修复前止损命中只压成方向布尔喂 KΘ 净额门，**从不落地为一笔退出成交**。
修复后 776 笔 RiskExit 出现，并连带把 CloseRoot / CloseReverseOpen / ReduceCore 各桶重排
（父腿被 RiskExit 提前收走 ⟹ 原本走 CloseRoot/ReduceCore 的路径不再发生）。

---

## 3. 归因：引用到行

`c8e87c4a0d` diff 中改变成交行为的是这一串（三处，一条因果链）：

**① `rust/src/theta_v0/backtest/admission.rs`，`k_theta_risk_gate`（+1189 起）**
返回类型加第三分量 `Vec<ActiveLeg>`；原
```rust
let mut long_stop = false;  let mut short_stop = false;
… if !bar.untradable && stop_hit(bar, stop, exit_side) { match leg.dir { Long => long_stop = true, … } }
```
改为
```rust
let mut stop_risk_seeds = Vec::new();
… if !bar.untradable && stop_hit(bar, stop, exit_side) { stop_risk_seeds.push(*leg); }
let long_stop  = stop_risk_seeds.iter().any(|leg| leg.dir == VoiceSide::Long);
let short_stop = stop_risk_seeds.iter().any(|leg| leg.dir == VoiceSide::Short);
```
⟹ 腿 ID 不再被压平丢失，布尔降为派生投影。**这一步单独看仍是行为中性的**（布尔值不变）。

**② `rust/src/theta_v0/backtest/fill.rs`（+3213 / +3510 起）**
接住第三分量并把它传下去，调用点由 `coverage::pi_theta_step_traced` 换成
`coverage::pi_theta_step_traced_with_risk_seeds(…, gate, &stop_risk_seeds, …)`。

**③ `rust/src/theta_v0/strategy/coverage/step.rs`，`coverage_step_from_buckets_sep_with_risk_seeds`（+131 起）— 这里是行为真正改变的一行**
```rust
let mut direct_close_seeds = buckets.close.clone();
for seed in risk_close_seeds {
    if !direct_close_seeds.iter().any(|leg| leg.id == seed.id) { direct_close_seeds.push(*seed); }
    if let Some(carrier) = risk_seed_carrier(work.tree_prefix(tree_end), seed) { … push(carrier) }
}
let closed = close_indices(prev_active, &direct_close_seeds);   // 原为 &buckets.close
```
止损命中的腿及其结构 carrier 被并入关闭种子集，进入既有
`step_active_set_with_subtree_close` ⟹ 父腿产 `RiskExit`、后代产
`CloseReverseOpen + via_structural_prune`。同 commit 内 `close_and_flip_seeds` 与
`restore_ancestor_chain_from_registry` 的种子源也一并由 `buckets.close` 换成 `direct_close_seeds`。

新增的 `risk_seed_carrier`（step.rs +49 起）用「同级严格右端点精确命中 → 唯一左开右闭 span 兜底 →
多重命中失败关闭」重建 carrier 身份，不按方向或宽度猜测。

### 合法性：有授权，是行为修复不是无意漂移

| 环节 | 证据 |
|---|---|
| #564（CLOSED） | `π 疑似行为 bug：父结构止损穿价未触发子树级联平仓（#526 P0 锁复现）`——诊断实锤 `stop_hit` 算了但腿 ID 在压布尔时丢失 |
| #568（CLOSED） | 裁定正文：「**Q1=S1：π 应当级联**（父结构止损触发 ⟹ 子平先父平后）。「语义差」假说死亡……**判：是 bug**」；「Q2=采诊断修复方向四件套」（admission 保留腿 ID → risk-close seed → 复用 `step_active_set_with_subtree_close` → KΘ 夹紧留作投影），标注「两裁，**编排者确认**」 |
| #572（CLOSED） | SPEC 票，即本 commit 落地 |
| ADR-0001 | #568 援引「ADR-0001 明令风险平仓必须保留逐腿动作」 |

⟹ **行为变更本身合法且经裁定**，779 行金标准所锁的旧行为是 bug 态。

### 但漏了一步：基线重订登记缺失

ADR-0001 line 204 定的常例（用户裁定，#486 时立）：
> **常例（用户裁定）**：此后实装票合法改变 classifier/产出链时，基线按「同工位反证 + 重订」逐票更新。

实测 `grep`：`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md` 中 **`#572` 零命中**，
全仓 `1678751` / `1,678,751` 在 `docs/` 下零命中（唯一出现在 `.chanlun/review-results/issue606-impl-20260728.md:82`，
且那里是当作"未知既存差异"记的）。⟹ **#572 未按常例做基线重订，ADR 现载值已失真**。

### 连带失真：四层绝对值符号翻转

| 量 | P0 `155a5db845` | P1/HEAD | ADR-0001 line 204/206 现载 |
|---|---|---|---|
| execR | **+4,626,831** | **−312,337** | +4626831 ← **已不成立** |
| R | **+5,154,033** | **−276,641** | +5154033 ← **已不成立** |
| MaxDD | 0.1085 | 0.0776 | 0.1085 ← 已不成立 |
| LCB(R) | −2,176,392 | −1,961,062 | −2176392 ← 已不成立 |
| 三态 | INCONCLUSIVE | **无(R≤0)** | INCONCLUSIVE ← 已不成立 |

这不只是金标准文件漂移——ADR-0001 登记的 wf8 四层基线**符号翻转**（正 execR → 负 execR，
三态由 INCONCLUSIVE 降为「无(R≤0)」）。#542 那条重订明写「**四层绝对值基线不变**……本票只动
schema，不动产出链与行为」，该句在 `155a5db845` 上仍成立；#572 之后不再成立且无人登记。

---

## 4. 锚的成色判定

**`/tmp/vocab_align_dump/trades.jsonl`（779 行 / 1,163,427 B / SHA `06243bac…f9b9`）
= `155a5db845` 代码态的真品，含 #542 schema 加身份，不含 #572 π 级联止损修复。**

- **有效性**：它**不是**"不同批次运行的脏数据"，也**不是**陈旧无效物——干净 worktree 复跑逐字节复现，
  SHA 与 ADR-0001 line 206 登记值对得上。作为 `155a5db845` 的回归锚它**完全有效**。
- **代表什么**：它锁的是「**止损命中从不产生退出成交**」的行为态（`RiskExit=0`）。
  #568 已裁定该行为是 bug。⟹ 锚现在代表的是**一个已被裁定为错误、且已被修复的行为**。
- **对 HEAD 的效力**：**归零**。任何在 `c8e87c4a0d` 之后拿 `/tmp/vocab_align_dump/trades.jsonl`
  做 `cmp=0` 验收的票，都会得到必然的假红。`tower_events.jsonl`（354,946 B）**不受影响，四点全 BIT-EXACT，
  仍是有效锚**。

### 需订正的既有记载

- **`.chanlun/review-results/issue606-impl-20260728.md:84`（归因错误，须订正）**
  原文：「干净 HEAD 同样不等于 `/tmp/vocab_align_dump/trades.jsonl`……此差异**先于本票存在**，
  与 S1 改动无关（……两文件很可能来自**不同批次运行，非同一次 dump**）」。
  前半句（与 #606 无关）**正确**；后半句归因**错误**——两文件同批次同次 dump，
  差异源是 `c8e87c4a0d` 一个 commit，本报告 bit-exact 复现证毕。
  该票据此把验收基准从金标准降级为「vs 干净 HEAD 自比」的做法，**结论不受影响**（#606 确实行为零变化，
  已由 P1 vs P2 bit-exact 独立佐证），但降级理由须换成真因。
- `.chanlun/review-results/issue487-impl / issue489-* / issue542-*` 诸票的 `cmp=0` 验收
  **全部有效**——它们都在 `c8e87c4a0d` 之前，锚在其代码态上确实成立。

---

## 5. 处置建议（只出材料，不执行）

### 建议：重订新锚 + 补登 ADR，**不建议回退肇事面**

**不回退的理由（反证材料已齐）**：#572 是 #564 诊断 → #568 编排者裁定 → #572 SPEC 落地的完整授权链，
且 #568 明文「判：是 bug」「Q1=S1：π 应当级联」。回退等于把已裁定的 bug 重新装回生产，
并使 ADR-0001「风险平仓必须保留逐腿动作」失守。旧锚锁的 `RiskExit=0` 是被裁定的错误行为。

**建议动作（按序，均待用户点头后另切票执行）**：

1. **留底 + 重订金标准**：`/tmp/vocab_align_dump` → `/tmp/vocab_align_dump-pre572` 留底
  （沿 `-pre486` / `-pre542` 先例），新锚取 `trades.jsonl` 1,678,751 B / 1,097 行 /
   SHA `5d90baea…eecc`；`tower_events.jsonl` 354,946 B / SHA 不变，**不需重订**。
   现成产物在 `/tmp/bisect-out-8ed3572c02/`（HEAD 干净 worktree 所产，已与 `c8e87c4a0d` bit-exact 互证）。
2. **补登 ADR-0001 基线重订条目（#572）**，与 #486/#542 两条同格式，须载明：
   - 归因证据 = **同工位反证**（本报告：`155a5db845` bit-exact 复现旧锚，`c8e87c4a0d` 单跳翻转，
     `c8e87c4a0d`≡HEAD 排除第二漂移）；
   - **四层绝对值基线重订**：`execR=−312337 / R=−276641 / MaxDD=0.0776 / LCB(R)=−1961062`，
     三态 INCONCLUSIVE → **无(R≤0)**（符号翻转须显式标注，不可只改数字）；
   - #542 条目内「四层绝对值基线不变」一句加时效标注（其有效域截至 `155a5db845`）。
3. **订正 `issue606-impl-20260728.md:84` 的归因段**（第 4 节所述）。
4. **流程口子（值得单独立票）**：`c8e87c4a0d` 的 commit message 自称
   「差异面全枚举——既有 2111 测试零断言改动」——它枚举了**单测断言面**，
   但 opsem dump 金标准（`OPSEM_DUMP_DIR` 产物）**不在枚举面内**，于是一次符号翻转级的基线变更
   静默通过。建议把「wf8 关臂 `trades`/`tower` 对金标准 `cmp`」纳入行为变更票的强制差异面清单，
   使 ADR line 204 的常例有机械触发点，而非靠人记得。

---

## 附：产物留存

| 路径 | 内容 |
|---|---|
| `/tmp/bisect-out-155a5db845/` | P0 产物（= 锚，bit-exact） |
| `/tmp/bisect-out-c8e87c4a0d/` | P1 产物 |
| `/tmp/bisect-out-8ed3572c02/` | P2 = HEAD 产物（重订新锚的候选源） |
| `/tmp/bisect-log-<tag>.txt` | 三点完整 stdout（含四层 `[m8] wf8:` 行） |
| `/tmp/gdb-<tag>/` | 三个干净 worktree（可 `git worktree remove` 回收） |

`/tmp/vocab_align_dump/` 全程只读，未修改、未删除、未重订。
