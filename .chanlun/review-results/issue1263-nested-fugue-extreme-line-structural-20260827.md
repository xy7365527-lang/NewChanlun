# #1263 探针：nested_fugue 极值线否定 × 背驰段打破结构判据交叉——价格代理的吻合/误杀/漏杀面

- **issue**：[#1263](https://github.com/xy7365527-lang/NewChanlun/issues/1263)（#1232 第一条 nested_fugue 极值线三处的前置探针，只产读数不裁）。
- **性质**：测量 + 读数，**零生产码改动**（新增代码全部 `#[cfg(test)]`）。
- **基线**：`sandcastle/issue-1263`（HEAD `c5bc559434`）。
- **数据**：BTC 2024 同窗（复用 #1147/#1223 链数据 `analysis/data_cache/btc_1m_full.json`，Binance 归档重下 2024-01..2025-01）；磁带走 **nested_fugue 生产回放路径**（`analysis/_dump_tape_rust.py BTC` → `analysis/data_cache/_tape_v2r_BTC.bin`，tape_fp `bsp=9092 div=18887 flips=33197`）。窗口 `2024-01-01 .. 2025-01-01`（闭区间，528480 bar）。
- **日期**：2026-08-27。

---

## 0. 一句话结论

**三处「现价破记录极值 ⟹ 否定」的价格代理与「背驰段被打破」结构判据几乎不吻合：3335 个事件里吻合仅 6 例（0.18%），误杀 1667、漏杀 1662。** 每 site 的精确值见 §4。根因不是「代理有噪声」，是**方向相反**：价格代理判的是「背驰段**延伸**（破极值新高/新低）」，而 027:23 的「打破背驰段」结构判据判的是「背驰段**被反向打破**（三买卖坐实/走势完成）」。对照臂实测：价格代理对「走势未完成」（#1267 结构判据）的吻合率高达 99.4%（窗口清窗 514/517），对「背驰段被打破」吻合率 0.18%。**按 #1263 评论的验收口径（100% 完全等价，任一例分歧即缺陷证据），本票实测 3329 例分歧 ⟹ 走替换；「背驰段被打破」的操作化规格见 §7。**

---

## 1. 三处否定 site（口径逐字）

| site | 生产位置（file:line） | 否定条件（价格代理） | 语义 |
|---|---|---|---|
| **窗口清窗** | `rust/src/trading/nested_fugue.rs:609`（卖）`:627`（买） | 卖 `c > w.extreme`；买 `c < w.extreme` | candidate 武装窗口被现价破极值否定 |
| **定位失效** | `rust/src/trading/nested_fugue.rs:722` | 卖 `c > ext` | 区间套定位记忆被现价破极值否定 |
| **声部解栈** | `rust/src/trading/nested_fugue.rs:803-804` | 空 `c > line`；多 `c < line` | 链上 voice 出生相否定线被现价破线 ⟹ 解栈 |

方向约定（下文统一）：`Short` = 卖/顶背驰（背驰段方向 **Up**）；`Long` = 买/底背驰（背驰段方向 **Down**）。

- **价格代理（价格破）**：`Short ⟹ c > extreme`（向上破极值，即新高）；`Long ⟹ c < extreme`（向下破极值，即新低）。
- **结构判据（结构破）＝「背驰段被打破」**（生产结构函数，不另造，操作化于 Rust 探针 `nested_fugue.rs:352`）：
  - `Short`（段方向 Up）⟹ 向下打破 = confirmed Sell1/Sell3 ∨ 该层方向翻 Down；
  - `Long`（段方向 Down）⟹ 向上打破 = confirmed Buy1/Buy3 ∨ 该层方向翻 Up。
  - 三家族点名：**反向突破** = 本 bar confirmed BSP（`tape.rs:35` `bsp_events`，信号层产出）；**中枢三态** = `CenterBook::{is_dead_down,is_frozen}`（`center_book.rs:376/382`，confirmed Type3 坐实即中枢终结）；**次级别走势完成** = 本 bar `dir_flips` 方向翻转（`tape.rs:45`）+ `up_move_settled`（`tape.rs:37`）。

## 2. 数据与复现验证

- **窗口**：`2024-01-01 .. 2025-01-01`（闭区间，528480 bar，磁带前 528480 bar 切片；全量 571680 bar）。
- **计数器对表（生产回放路径一致性）**：本探针窗口内 `root_entries=8 / spawns=0 / negates=7 / nest_breaks=517`；生产回放 `analysis/nested_recursive_fugue_final_backtest.py BTC`（全 571680 bar）`root_entries=8 / spawns=0 / negates=7 / nest_breaks=555`——差 38 = 2025-01 尾窗（43,200 bar），其余逐位一致 ⟹ 探针磁带与生产回放同源。
- **★ BTC 2024 窗口的机制事实（照实）**：`spawns=0`——根 voice 恒落在最高 θ 涌现层 ladder2（= floor），所有 nest fire 走 floor_stop（全窗 3267），子 voice 从不诞生 ⟹ 声部解栈 site 的否定事件只有 7 例（全为根入场经 nf 出生相、Long 方向）。
- **插桩零扰动**：三处 site 的 `#[cfg(test)]` dump 通道只在 env `P1263_DUMP_PATH` 非空时记录；生产路径（非 test 构建）零字节改动，`cargo check --tests` 绿。

## 3. 事件流规模

- 事件总数 **3335**（价格破 1673 + 仅结构破 1662）。按 site：窗口清窗 1049 / 定位失效 2254 / 声部解栈 32。
- 原始事件级 dump：`.chanlun/review-results/issue1263-nrf-negation-events-2024.jsonl`（每行 `site/bar/ladder/dir/extreme/close/price_broke/struct_broke/structural`，structural 为同 bar 结构态原始分量）。
- 分类器：`.chanlun/review-results/issue1263-classify.py`（纯标准库，可复现）。

## 4. 主口径读数：价格破 × 背驰段被打破（同 bar）

**三分类**：吻合 = 价格破 ∧ 结构破；**代理误杀** = 价格破 ∧ 结构未破；**代理漏杀** = 结构破 ∧ 价格未破。

### 4.1 全量

| 类别 | 精确计数 | 占比（n=3335） |
|---|---|---|
| **吻合** | **6** | **0.18%** |
| **误杀** | **1667** | 50.0% |
| **漏杀** | **1662** | 49.8% |

### 4.2 每 site

| site | n | 吻合 | 误杀 | 漏杀 |
|---|---|---|---|---|
| 窗口清窗 | 1049 | 4 | 513 | 532 |
| 定位失效 | 2254 | 2 | 1147 | 1105 |
| 声部解栈 | 32 | 0 | 7 | 25 |

### 4.3 分 ladder / 分方向

| site | ladder | dir | n | 吻合 | 误杀 | 漏杀 |
|---|---|---|---|---|---|---|
| 窗口清窗 | 2 | Long | 456 | 2 | 165 | 289 |
| 窗口清窗 | 2 | Short | 389 | 2 | 152 | 235 |
| 窗口清窗 | 3 | Long | 106 | 0 | 103 | 3 |
| 窗口清窗 | 3 | Short | 98 | 0 | 93 | 5 |
| 定位失效 | 2 | Short | 1993 | 2 | 979 | 1012 |
| 定位失效 | 3 | Short | 261 | 0 | 168 | 93 |
| 声部解栈 | 2 | Long | 32 | 0 | 7 | 25 |

## 5. 对照臂：价格破 × 走势未完成（#1267 结构判据镜像）

「走势未完成」（`nested_fugue.rs:372`，镜像 #1267）= 走势类型延续（`dir_now` 仍原方向）∨ 中枢未死（`CenterBook::alive`，`center_book.rs:358`）∨ 三买卖未坐实（`!is_dead_down`/`!is_frozen`）。**这是与「背驰段被打破」方向相反的结构判据**。

| site | n | 吻合（价格破∧未完成） | 误杀（价格破∧已完成） | 漏杀（未完成∧价格未破） |
|---|---|---|---|---|
| 窗口清窗 | 1049 | **514** | 3 | 16 |
| 定位失效 | 2254 | **621** | 528 | 47 |
| 声部解栈 | 32 | **3** | 4 | 4 |

**读法**：窗口清窗的价格破事件（n=517）中 **514 例（99.4%）发生在「走势未完成」态**——价格代理本质在测「背驰段尚未反向打破（仍在延伸）」，而不是「背驰段被打破」。定位失效的吻合率较低（621/1149=54%）另有归因：located 记忆在价格破极值前已被更早的三卖坐实「结构完成」但记忆不随结构死亡清空（§6.2），故价格后破旧极值时结构态已翻。

## 6. 逐例归因（为什么价格与结构判得不同）

### 6.1 误杀面（价格破 ∧ 结构未破，n=1667）

- **窗口清窗（513）**：全部 `confirmed_any=False` 且 `center_alive=True`（100%）。价格破极值时同 bar **无任何 confirmed BSP、中枢仍活、方向未翻**——纯价格破新高/新低，结构面零「被打破」证据。例：`(137,2,Long)`、`(1546,3,Long)`、`(3763,2,Short)`。
- **定位失效（1147）**：全部 `confirmed_any=False`；中枢态三分——558 例 `center_dead_down=True`（**三卖坐实在更早 bar 已坐实**，located 记忆不随结构死亡清空，价格后来破**旧**极值才清记忆）、571 例 `center_alive=True`（中枢仍活，价格破旧极值，同 bar 无结构事件）、18 例 `center_frozen=True`（三买坐实中枢向上冻结，位于卖侧 located 记忆不随向上冻结清空）。例：`(298,2,Short)`（dead_down=True）、`(2373,2,Short)`（alive=True）。
- **声部解栈（7）**：全部 `confirmed_any=False`。根 voice 破否定线时无结构事件。例：`(3564,2,Long)`、`(4016,2,Long)`。

### 6.2 漏杀面（结构破 ∧ 价格未破，n=1662）

触发分量（每 site）：

| site | 漏杀 n | confirmed Buy3(三买坐实) | confirmed Sell3(三卖坐实) | 方向翻（走势完成） |
|---|---|---|---|---|
| 窗口清窗 | 532 | 292 | 240 | 0 |
| 定位失效 | 1105 | 0 | 1053 | 52 |
| 声部解栈 | 25 | 19 | 0 | 6 |

**归因**：结构上「背驰段被打破」（三买卖坐实 = 中枢三态终结 = 反向突破）已在本 bar 发生，但**价格没有破极值**（价格代理的否定条件没触发）——价格代理漏掉了结构已确认的打破。例：`(1238,2,Long)`、`(3619,2,Short)`（窗口清窗）；`(271,2,Short)`、`(2366,2,Short)`（定位失效）；`(1516,2,Long)`（声部解栈）。

### 6.3 吻合面（n=6）

仅 6 例价格与结构同 bar 同向。窗口清窗 4 例：`(10820,2,Long)`、`(264706,2,Long)`、`(294539,2,Short)`、`(335056,2,Short)`；定位失效 2 例：`(68239,2,Short)`、`(306549,2,Short)`。声部解栈 0 例。

### 6.4 根因一句话

价格代理与结构判据**方向相反**：价格破（Short）＝向上破极值（新高）＝背驰段**延伸**（背了又背）；结构破（Short）＝向下打破（三卖坐实/走势完成）＝背驰段**被反向打破**。027:23「只要没有打破背驰段，就要密切注意」的「打破」是后者（反向打破），而三处代码把「破记录极值」（前者，延伸）当作否定触发——两者在定义上不同向，实测吻合 0.18%。

## 7. 结构判据的操作化规格（给替换实装直接可用）

**「背驰段被打破」= 下列生产结构函数/条件任一成立（同 bar、同 ladder k、按背驰段方向）：**

- **反向突破**：本 bar `bsp_events[k]` 出现 confirmed 反向买卖点——`Short`（段 Up）⟹ `Sell1 ∨ Sell3`；`Long`（段 Down）⟹ `Buy1 ∨ Buy3`（`tape.rs:35`，信号层产出，零另造）。
- **中枢三态（三买卖坐实）**：`Short` ⟹ `CenterBook::is_dead_down(k, last_seg)`（`center_book.rs:376`，confirmed Sell3 终结中枢）；`Long` ⟹ `CenterBook::is_frozen(k)`（`center_book.rs:382`，confirmed Buy3 终结中枢）。注：confirmed Type3 事件与上一项 `Sell3/Buy3` 同源——本项给出「中枢死亡」的状态读法。
- **次级别走势完成**：本 bar `dir_flips[k]` 翻反向——`Short ⟹ Down`；`Long ⟹ Up`（`tape.rs:45`）。

**方向语义（供 #1232 裁定斟酌，本票只读数）**：上述规格判「背驰段被**反向**打破」。若替换实装的否定语义是「背驰段**未完成**（原方向延伸）⟹ 否定」（止损语义），则结构判据 = **「走势未完成」**（`nested_fugue.rs:372` 的 #1267 镜像：走势类型延续 ∨ 中枢未死 ∨ 三买卖未坐实），与现价格代理同向（实测 99.4% 吻合）。两条规格方向相反，替换实装须先裁定取哪一条——本探针只给出两条各自的操作化与实测面。

## 8. 结论建议（给 #1232 第一条 (a)/(b)/(c) 的底盘，不代裁）

1. **读数底盘**：价格代理与「背驰段被打破」结构判据吻合 6/3335 = 0.18%，误杀 1667 + 漏杀 1662 = 3329 例分歧——按 #1263 评论验收口径（100% 完全等价、任一例分歧即缺陷证据），**价格判据不能被证明与结构谓词同一（由构造等价）**，反而方向相反。
2. **(a) 替换面**：三处否定条件的「价格破极值」不是缠论几何载体（027:23 的「打破」是反向打破，价格代理测的是延伸）——建议走替换，替换判据用 §7 的操作化规格。
3. **(b) 方向抉择**：替换前必须先裁「否定」的语义是「背驰段被打破 ⟹ 否定」还是「背驰段未完成 ⟹ 否定」——两者方向相反，分别对应 §7 两条规格；本票已给出两条规格各自的实测面（0.18% vs 99.4% 窗口清窗）。
4. **(c) 例外登记面**：若选择把「价格破极值」登记为几何载体例外，须按 #804 举证门三件补齐退场条件（本票读数显示其与 027:23 结构谓词方向相反，作为例外举证的门槛更高，是否成立归裁定）。

## 9. 未测项 / 局限（照实）

1. **单标的单窗**：只有 BTC 全年 2024。别的品种/年份没跑；BTC 2024 是强趋势年（buy&hold +123.6%）。
2. **声部解栈 site 样本极薄**：BTC 2024 窗口 `spawns=0`（根恒在 floor ladder2），否定事件仅 7 例（全为根入场 Long 出生相）。该 site 读数（误杀 7/漏杀 25）是根 voice 的止损面，不是子 voice 级联否定面——若需子 voice 面，须换品种/窗口使 spawns>0（例：全历史 8 标的里 OKLO 有 spawn），本票按票面 BTC 同窗未越界。
3. **「同 bar」口径**：本票结构破判「结构事件发生在否定同 bar」；若改判「截至该 bar 已结构打破（累计态）」，吻合升至 570（17.1%，主要来自 located 记忆跨越更早三卖坐实）——但仍远非等价，结论方向不变（§6.2 已点名该机制）。
4. **结构破的「反向」只判一层**：结构判据在 ladder k 本层读，未下探 k−1 次级别完成的多层形态（次级别走势完成只取本层 `dir_flips`）；多层下探的严格形态会影响漏杀计数边界，未测。
5. **零生产码改动**：探针全部 `#[cfg(test)]`；`env_registry` 未新增键（env 名直接 `std::env::var` 读，不进生产注册表）。

## 10. 复现命令

```bash
cd /home/agent/workspace
# 1) 数据（Binance 归档，13 个月；输出 analysis/data_cache/btc_1m_full.json，gitignored）
BTC_START_YEAR=2024 BTC_START_MONTH=1 BTC_END_YEAR=2025 BTC_END_MONTH=1 \
  BTC_OUTPUT=btc_1m_full.json python3 scripts/download_btc_binance.py

# 2) 磁带（nested_fugue 生产回放路径；需 PyO3 模块 newchan_rust 已 maturin develop）
PYTHONPATH=src:analysis .venv-probe/bin/python analysis/_dump_tape_rust.py BTC

# 3) 探针（Rust #[cfg(test)] #[ignore]，事件级 dump；窗口 2024-01-01..2025-01-01）
cd rust
export PYDIR=/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu
export LD_LIBRARY_PATH=$PYDIR/lib:$LD_LIBRARY_PATH LIBRARY_PATH=$PYDIR/lib
P1263_DUMP_PATH=/tmp/p1263_events.jsonl \
cargo test --release --lib nested_fugue_extreme_probe -- --ignored --nocapture

# 4) 分类读数（纯标准库）
cd ..
python3 .chanlun/review-results/issue1263-classify.py /tmp/p1263_events.jsonl
```

## 11. 收尾

- `python3 scripts/check_fixture_drift.py` → 退出码 3：`✗ FIXTURE-GATE ENVIRONMENT（非漂移）：lake 不在 PATH`（与 #1147/#1206/#1221/#1223 同款环境缺 Lean/Lake 工具链，非漂移；本 diff 零触碰 `formal/`）。
- 数据文件 `analysis/data_cache/btc_1m_full.json` 与 `_tape_v2r_BTC.bin` 已 gitignore，不入仓。

*本报告只产读数，不裁口径。读数已出：价格代理与「背驰段被打破」结构判据吻合 0.18%（误杀 1667 / 漏杀 1662），价格代理实际测「走势未完成」（窗口清窗 99.4%）；操作化规格见 §7。*
