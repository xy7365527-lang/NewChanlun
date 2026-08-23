# #1206 探针：出场 cond1 拒 × 前高突破坐实交叉——「已破仍被拒」真误拒份额

- **issue**：[#1206](https://github.com/xy7365527-lang/NewChanlun/issues/1206)（#1204 问 1 的前置探针）。
- **性质**：测量 + 读数，**零生产码改动**。不裁口径、不改进出判定（另开票），本票只产读数。
- **基线**：`sandcastle/issue-1206`（HEAD `e999f00`）。探针复用 #1147/#1152 既有 `#[cfg(test)]` 插桩，零代码改动。
- **数据**：Binance 归档重下 2024-01..2025-01 BTCUSDT 1m（`scripts/download_btc_binance.py`，13 个月 zip），窗口 `2024-01-01 .. 2025-01-01`（528480 bar），与 #1147/#1152 同窗同数据。
- **日期**：2026-08-23。

---

## 0. 一句话结论

**出场 cond1 拒的 308 个候选中，「已破前高却未强力不背驰创新高」的真误拒面在 12 小时前瞻窗下 ≈ 28%（86/308）；随前瞻窗 6h→48h，该面从 37.0% 降到 11.4%——窗口越长，被归入「破高后强力不背驰创新高」例外臂（拒得对）的越多。** 即：**真误拒面约 1/4 ~ 1/3**，其余约 2/3 是例外臂（破高后创新高，拒得对）+ 少量「未破前高」（中枢继续，拒得对）。这个数说明 #1204 问 1 的 (a) 改动面**不是零、也不是大头**，且对「之后」窗口的口径高度敏感。

---

## 1. 口径逐字（操作化）

### 1.1 原文正本（锚）

- **43 课答疑正文**（`docs/chanlun/text/blog/043-第43课.md:194`，行首无注标记、界内、无行内注 ⟹ 正文）：

  > 这程序很清楚，一般都按盘整背驰来，但有一种特殊情况是不出现盘整背驰的，就是小级别变大级别的情况，这种情况，如果不破前期高点，那只是一个新中枢，所以当然持有，如果破了，那就一定要走，除非出现强力不背驰创新高的情况。

  （票面引 `043-第43课.md:198-202` 实为「:194 正文 + :196-198 注」区间；本报告锚 :194 正文，:196-198 是行首「(注：」的编纂者注。）

- **43 课追问正文**（`docs/chanlun/text/blog/043-第43课.md:218`）：

  > =是，不能杀跌。而且还存在着继续不背弛创新高的可能。不过这里说的都是比较粗糙的方法……

- **39 课流程正文**（`docs/chanlun/text/blog/039-第39课.md:30`，摘引避开行内「(娇注：…）」）：

  > 如果没有，当i为偶，若Ai+3不跌破Ai高点，则继续持有到Ai+k+3跌破Ai+k高点后在不创新高或盘整顶背驰的Ai+k+4卖出，其中k为偶数。

### 1.2 前高/前低（参考极值）判定

- 候选点 = `source_index`（BSP 点锚，`end_index == source_index` 的锚点子段 `anchor_sub`）。
- **δ=Short（顶/出场，卖）**：`前高` = 候选点所属结构（`sub_moves`）中锚点子段的**前一向上子段**的区间最高价（`sub_moves[anchor_idx−1]`，方向 Up）。
- **δ=Long（底/出场，买）**：`前低` = 锚点子段的**前一下向子段**的区间最低价（方向 Down）。
- 无前一反向子段的边界（全量 1 例，level≥2 子段同向）：退用 `exec_move` 区间极值（δ=Short 取 high，δ=Long 取 low）。

### 1.3 「候选点之后」窗口

- 之后窗口 = `bar ∈ (source_index, source_index + H]`（闭区间右端含 H；候选点当 bar 不计）。
- **主口径 H = 720 bars（12 小时）**；敏感性 H ∈ {360, 1440, 2880}（§4）。
- 另给 `bar`（候选评估 bar）起点的对照（§4 注），主表用 `source_index`。

### 1.4 双口径（严格/宽松）

| 口径 | 跌破判据（δ=Short） | 升破判据（δ=Long） |
|---|---|---|
| **宽松档（任意触及）** | 之后窗口内 ∃ bar 的 **low** < 前高 | ∃ bar 的 **high** > 前低 |
| **严格档（收盘/段端点跌破）** | 之后窗口内 ∃ bar 的 **close** < 前高 | ∃ bar 的 **close** > 前低 |

严格档取「收盘跌破」作 39 课「Ai+k+3 跌破 Ai+k 高点」的机械读法——「跌破」是段意义上的（段端点/收盘），不是盘中下影线触及。两档在本候选集上读数几乎一致（§3），说明「触及 vs 收盘」不是主敏感轴。

### 1.5 三分类

对每个候选，在之后窗口内按事件序判：

| 类 | 判据 | 语义 |
|---|---|---|
| **未破前高** | 之后窗口内从未跌破前高（δ=Short）/升破前低（δ=Long） | 新中枢 = 中枢继续 → 持有，**拒得对** |
| **已破前高（真误拒）** | 跌破前高，且首次跌破之后窗口内**没有**收盘价重新升破前高（δ=Short）/跌破前低（δ=Long） | 第三类卖点坐实 → 一定要走，**拒 = 误拒** |
| **破高后强力不背驰创新高（例外）** | 跌破前高，且首次跌破之后窗口内收盘价重新升破前高/跌破前低 | 43 课唯一例外 → 持有，**拒得对** |

**例外臂明写简化**：「强力不背驰创新高」简化为「首次跌破之后、窗口内收盘价重新升破前高（创新高）」，**不另算 #985 力度原语（ForceL）**。因此本报告的「真误拒」是**下界**（把「创新高但力度背驰」的候选也暂算进了例外臂）；若要细化例外臂力度门槛，另开票。

---

## 2. 取数与复现验证

- cond1 拒 = `stepfail ∈ {type23_descend_nodiv_condSome(1), type1_div_fail_Some(1)}`（后者是 Type1 路径的 cond1，2 例；与 #1147「cond1 方向桶 365」、#1152「出场 cond1 308」同口径）。
- 过滤：`would_close == True ∧ admit == False` ⟹ **308 例**（Long 160 / Short 148；level1 299 / level2 8 / level3 1；Type2/3 306 + Type1 2）。
- **探针重跑逐位复现 #1147**：`NEST_GATE_STATS total=2212 admitted=1111 rejected=1101`、`NEST_GATE_EXIT_CAND total=886 admitted=130 rejected=756`、`n_orders=238245 strat_return=0.467363`；标准 dump 与 #1147 原始 dump **逐字节相同**（`diff <(sort 新) <(sort 旧)` 空）。
- 全量结构 dump（306 键 → 399 候选线：308 cond1 + 91 同键异桶候选）与 #1152 样本结构 dump 在 51 个共同键上**逐字段相同**（0 mismatch）。

---

## 3. 三分类 × 双口径表

### 3.1 全量 308（主口径 H=720，`source_index` 起）

| 类 | 宽松档 | 严格档 |
|---|---|---|
| 未破前高（拒得对） | 9（2.9%） | 11（3.6%） |
| **已破前高（真误拒）** | **86（27.9%）** | **86（27.9%）** |
| 破高后强力不背驰创新高（例外，拒得对） | 213（69.2%） | 211（68.5%） |
| 合计 | 308 | 308 |

按方向拆（宽松档）：

| 方向 | n | 未破 | 已破（真误拒） | 例外 |
|---|---|---|---|---|
| Short（顶/出场） | 148 | 3（2.0%） | 43（29.1%） | 102（68.9%） |
| Long（底/出场） | 160 | 6（3.8%） | 43（26.9%） | 111（69.4%） |

### 3.2 样本面（42 键 → 44 例 cond1：42 Type2/3 + 2 Type1；主口径 H=720，`source_index` 起）

| 类 | 宽松档 | 严格档 |
|---|---|---|
| 未破前高 | 4（9.1%） | 4（9.1%） |
| **已破前高（真误拒）** | **14（31.8%）** | **14（31.8%）** |
| 破高后强力不背驰创新高（例外） | 26（59.1%） | 26（59.1%） |

样本面与全量面同构（真误拒 ≈ 1/3、例外 ≈ 2/3），样本的「未破」占比略高（9.1% vs 2.9%）是抽样方差，非系统性差异。

---

## 4. 敏感性（主敏感轴 = 前瞻窗 H）

| H（bars） | 未破 | 已破（真误拒） | 例外 | 严格档已破 |
|---|---|---|---|---|
| 360（6h） | 11（3.6%） | **114（37.0%）** | 183（59.4%） | 114（37.0%） |
| **720（12h）** | 9（2.9%） | **86（27.9%）** | 213（69.2%） | 86（27.9%） |
| 1440（24h） | 6（1.9%） | **57（18.5%）** | 245（79.5%） | 57（18.5%） |
| 2880（48h） | 4（1.3%） | **35（11.4%）** | 269（87.3%） | 35（11.4%） |

- **前瞻窗越长，「已破」越少、例外越多**：原因是 BTC 2024 全年强趋势，跌破前高后 24~48h 内大多重新创新高（被归入例外臂）。「已破」面从 6h 的 37% 一路降到 48h 的 11%。
- **双口径（close vs low）几乎不敏感**：全量各 H 下严格档与宽松档的「已破」数完全一致（86/114/57/35）；两档只在「未破/例外」边界差 1~2 例（触及但未收盘跌破的少数）。
- **起点（`source_index` vs `bar`）影响小**：H=720 时 `bar` 起点的已破 = 88（vs `source_index` 的 86），差 2 例。
- **前高口径（sub_moves vs exec_move）是敏感轴**：若把前高换成 exec_move 区间极值（本级段顶），「已破」升到 161/308（52%）且「未破」归零——但该口径把「创新高」本身算进了前高（exec_move 顶含锚点子段内的创新高），使 43 课例外臂失效，故本报告**不采用**，只登记为对照。sub_moves 前高（前一反向子段）才是「前期高点」（创新高之前的那一高点）的正确落点。

---

## 5. 结论（照实，只产读数）

**真误拒面（已破前高却被 cond1 拒、且未强力不背驰创新高）在主口径 12h 前瞻窗下 = 86/308 = 27.9%；6h 窗 37.0%、48h 窗 11.4%。** 一句话：**约 1/4 ~ 1/3 的出场 cond1 拒候选是真误拒，其余约 2/3 是「破高后创新高」例外臂（拒得对）+ 少量未破（拒得对）。** 据此，#1204 问 1 的 (a) 改动面（把「破前高坐实」的出场候选放行）大致覆盖 1/4 ~ 1/3，具体数取决于「之后」窗口口径；「破没破前高」目前从未被求值，这一交叉在案。

---

## 6. 复现命令

```bash
cd /home/agent/workspace
# 1) 数据（Binance 归档，13 个月；输出 analysis/data_cache/btc_1m_full.json，gitignored）
BTC_START_YEAR=2024 BTC_START_MONTH=1 BTC_END_YEAR=2025 BTC_END_MONTH=1 \
  BTC_OUTPUT=btc_1m_full.json python3 scripts/download_btc_binance.py

# 2) 生成 308 键文件（从 #1147 原始 dump）
python3 - <<'EOF'
import json
rows=[json.loads(l) for l in open('.chanlun/review-results/issue1147-stepfail-raw-2024.jsonl')]
def c1(r): return r['stepfail'] in ('type23_descend_nodiv_condSome(1)','type1_div_fail_Some(1)')
keys=[(r['bar'],r['level'],r['source_index']) for r in rows
      if r['would_close'] and not r['admit'] and c1(r)]
seen=set(); out=[]
for k in keys:
    if k not in seen: seen.add(k); out.append(dict(zip(('bar','level','source_index'),k)))
open('/tmp/p1206_keys.jsonl','w').writelines(json.dumps(k)+'\n' for k in out)
EOF

# 3) 探针（复用 #1152 sample_structure 插桩，零代码改动；≈17 min）
cd rust
export PYDIR=/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu
export LD_LIBRARY_PATH=$PYDIR/lib:$LD_LIBRARY_PATH LIBRARY_PATH=$PYDIR/lib
P1147_STEPFAIL_DUMP_PATH=/tmp/p1206_stepfail.jsonl \
P1147_WINDOW_START=2024-01-01 P1147_WINDOW_END=2025-01-01 \
P1152_SAMPLE_KEYS_PATH=/tmp/p1206_keys.jsonl P1152_SAMPLE_DUMP_PATH=/tmp/p1206_struct.jsonl \
cargo test --release --lib gate_on_stepfail_composition_dx -- --ignored --nocapture

# 4) 分类读数（纯标准库）
python3 .chanlun/review-results/issue1206-classify.py \
  analysis/data_cache/btc_1m_full.json \
  .chanlun/review-results/issue1147-stepfail-raw-2024.jsonl \
  /tmp/p1206_struct.jsonl \
  .chanlun/review-results/issue1152-cond1-sample-keys-2024.jsonl \
  .chanlun/review-results/issue1152-cond1-sample-structure-2024.jsonl
```

---

## 7. 未测项 / 局限（照实）

1. **前瞻窗 H 无正本**：39 课「Ai+k+3」的段位没给分钟数；本票用固定 H 并全量报敏感性（§4），H 的选择归编排者/后续票。
2. **例外臂力度未算**：「强力不背驰」简化为「创新高」（明写简化，§1.5）；真误拒读数因此是下界。细化须照 #985 力度原语（`segment_force_l`），另开票。
3. **单标的单窗**：BTC 全年 2024（强趋势年，buy&hold +123.6%），例外臂占比被趋势放大；别的品种/年份未跑。
4. **前高口径对照只给读数**：exec_move 对照（§4）不采用，原因已写；sub_moves 与 exec_move 两口径的最终选择归教义票。
5. **零生产码改动**：本票只产读数；未动任何生产代码，收尾 `python3 scripts/check_fixture_drift.py` 见 §8。

---

## 8. 收尾

- `python3 scripts/check_fixture_drift.py` → `✗ FIXTURE-GATE ENVIRONMENT（非漂移）：lake 不在 PATH`（与 #1147 同款环境缺 Lean/Lake 工具链，非漂移；本 diff 零触碰 `formal/`）。
- 数据文件 `analysis/data_cache/btc_1m_full.json` 已 gitignore，不入仓。

*本报告只产读数，不裁口径、不改进出判定。真误拒份额已出：#1204 问 1 的 (a)/(b) 可据此拍。*
