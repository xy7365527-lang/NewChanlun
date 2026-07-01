# Task #7 acc-classification：level 塔中间级空洞 H1/H2/H3 判别

**认识论等级**：L2（真实 BTC 数据逐信号分级别计数，可产否定性结果）。
**窗口**：bars=300000（2025-11-04→2026-05-31，全量=4613599），max_bars=300000。
**判别**：三路 instrument N^δ 门前后分级别计数（bit-exact 复制 decompose_capturable_spread 收集路径）。

## 分级别计数表（cls.levels.len() 最大=6）
| level | tower段数 | bsp_pre(门前) | 第一类 | 第二类 | 第三类 | Γ非Flat | sig_post(门后) | 门滤除 |
|---|---|---|---|---|---|---|---|---|
| 0 | 2502 | 714 | 0 | 0 | 714 | 714 | 492 | 222 |
| 1 | 756 | 77 | 0 | 77 | 0 | 77 | 0 | 77 |
| 2 | 220 | 22 | 0 | 22 | 0 | 22 | 1 | 21 |
| 3 | 62 | 5 | 0 | 5 | 0 | 5 | 1 | 4 |
| 4 | 15 | 2 | 0 | 2 | 0 | 2 | 1 | 1 |
| 5 | 4 | 1 | 0 | 1 | 0 | 1 | 1 | 0 |

## H1/H2/H3 判定（中间级 = level 1..levels_seen_max-1）
- level1: bsp_pre=77 (一/二/三=0/77/0) Γ非Flat=77 sig_post=0 → 门滤空(H1候选)
- level2: bsp_pre=22 (一/二/三=0/22/0) Γ非Flat=22 sig_post=1 → 有信号
- level3: bsp_pre=5 (一/二/三=0/5/0) Γ非Flat=5 sig_post=1 → 有信号
- level4: bsp_pre=2 (一/二/三=0/2/0) Γ非Flat=2 sig_post=1 → 有信号

### 判定：**H1（N^δ 门滤空）**：中间级 bsp_pre>0 但门后 sig_post=0 ⟹ [J_{ℓ-1}⊆J_ℓ] 嵌套链严格滤掉中间级。需 codex 异质确认（约束4）。

## 结构 sanity：level≥1 通过门信号的 rung 链（team-lead ③）
| lvl | source_index | δ | bits(u8) | rung层数(tower[lvl+1..]含src) |
|---|---|---|---|---|
| 2 | 3810 | -1 | 0x10 | 0 |
| 3 | 12183 | 1 | 0x02 | 0 |
| 4 | 73279 | -1 | 0x10 | 0 |
| 5 | 270577 | 1 | 0x02 | 0 |

**读解**：level_L 信号的 rung 层数 = 它上方 tower 各级含该 source_index 的段数。这些是**上级语境**（N^δ 递归链），不是「该信号也算作 level_(L-1) 信号」——每个 bsp 只在提取它的那个 classifier level 计一次（mod.rs 分级提取），rung 链是 N^δ 门的准入语境，非计数重复。
## 配对后 decomps level 分布（sig_post=门后配对前 vs decomps=配对后）
| level | sig_post(门后) | decomps(配对后) | 配对丢失 |
|---|---|---|---|
| 0 | 492 | 489 | 3 |
| 2 | 1 | 1 | 0 |
| 3 | 1 | 1 | 0 |
| 4 | 1 | 1 | 0 |
| 5 | 1 | 1 | 0 |

- n_unpaired（无配对出场反转信号，右删失剔除）=3
- **配对机制**：next_opp 表跨级别混合（所有 level 信号按 entry_bar 排序）——中间级信号找「下一个反向信号」时不分级别，几乎总配 level0（信号 489/493 是 level0）。decomp.level=入场信号 level（配对出场 level 不影响）。

---

# 最终判定（跨窗对比——修正上文 line 23 的窗内 H1 初判）

上文 line 23 的 "H1" 是**窗内**判定逻辑的输出（测试代码只见 300K 单窗，无法对比全历史报告）。加入**跨窗对比**后判定修正如下。

## 跨窗 level 分布对比（决定性证据）

| level | **300K 窗**（本测试，2025-11-04→2026-05-31，尾部对齐子集） | **全历史窗**（461万 bar，acc-multilevel-mu 报告） |
|---|---|---|
| 0 | 489 | 7741 |
| 1 | **0** | **0** |
| 2 | **1** (buy2, δ+1) | **0** |
| 3 | **1** (buy2, δ+1) | **0** |
| 4 | **1** (sell2, δ-1) | **0** |
| 5 | **1** (buy2 0x02, δ+1) | **1** (sell2 0x10, δ-1) |

**300K ⊂ 全历史**（两窗尾部对齐，300K = 全历史最后 30万 bar）。子窗 level2/3/4 各 1 信号，母窗（全历史）却为 0。

## 判定：塔构造窗口依赖（候选解释）

> **⚠ 本节初判被 codex 异质审查降级——见文末「codex 异质确认」。保留：H1 被否证 + 窗口依赖是真实机制。降级：「不是运行时 bug」未证明（已知 frontier 中枢发散 bug 未排除）。以下按初判原样保留供追溯，"不是 bug"的强断言以文末降级判定为准。**

**排除 H1（N^δ 门滤空）**：H1 声称"中间级 bsp 被门严格滤空"。但 300K 窗 level2/3/4 **各有 1 个信号通过门且配对成活**（sig_post=1 → decomps=1，无配对丢失）。门**没有**把中间级滤空——H1 被 300K 数据直接否证。（level1 确实门全滤，但那只是 level1 单级，非"中间级空洞"整体成因。）

**排除纯 H3（架构：上级层只产第二类）**：H3 是**真实的架构事实**（已坐实：中间级 bsp_pre 100% 第二类，mod.rs:247 上级层第一/三类不提取），它解释了"为什么高级别信号只能是第二类、为什么稀疏"。但 H3 **不能**解释"为什么 300K 有而全历史无"——架构对两窗一致，不产生跨窗差异。

**坐实机制 = 塔构造的窗口依赖性**：
1. 塔从**窗口第一个 bar** 重建（`IncrementalClassifier::new(bars,...)` 以窗口切片为输入，无窗口前的历史）。
2. 300K 窗在第 3810/12183/73279 bar（局部坐标）处，高级别走势/中枢边界由**窗口内前史**决定；全历史窗在对应全局坐标（≈431万+）处，高级别结构由**431万 bar 累积前史**决定——**同一价格点的塔结构在两窗不同**。
3. 第二类识别（`extract_second_for_level`）依赖 `upper_moves.sub_moves` 的「第一类离开+回拉不创新高/低+背驰」结构——这个结构对塔边界敏感，窗口不同 ⟹ 识别结果不同。
4. **交叉验证（决定性）**：300K 的 level5 = buy2(0x02)/δ+1，全历史的 level5 = sell2(0x10)/δ-1——**类型和方向都不同 ⟹ 不是同一个信号被 bug 丢弃，是两窗产出了完全不同的高级别信号集**。若是"全历史路径 bug 丢信号"，则全历史应是 300K 的超集（含 300K 那几个），但实际全历史的 level5 与 300K 的 level5 是不同信号 ⟹ 排除"丢信号 bug"。

**真封证据（排除计数/索引/分块 bug）**：sig_post_sum=496 = decomp(493) + n_unpaired(3)，逐级聚合完整（decomp_sum=493=n_signals）。无索引越界、无分块边界丢失、无计数重复——team-lead 假设的候选点 (a)(b)(c)(d) 均无命中。

## 对反演 7741→0→0→0→0→1 的解释

全历史窗这个分布**是真实的**（不是 bug 产物），但其反常性（非单调、中间空、顶有 1）**不代表"高级别真稀疏是市场基本属性"**，而是：
- level0 密集（7741）= 第一/三类在 L0 密集提取。
- level1-4 = 0：全历史窗**这条特定塔路径**上，第二类识别在 level1-4 恰好一个都没命中（第二类稀疏 × 塔窗口依赖的联合结果）。
- level5 = 1：全历史窗塔在最高级恰好命中 1 个第二类。
- **300K 窗证明这个"中间全 0"是塔路径特定的，不是级别本身不可能有信号**——换个窗口（300K）中间级就有信号。

**⟹ 高级别 alpha 检验（acc-multilevel-mu 的 level2+ 全否证）建立在"全历史单窗塔路径"上**，该路径的高级别信号集是窗口依赖的偶然产物（n=个位数），**Le Cam 硬墙（n<30 结构性稀疏）判定成立但归因需修正**：稀疏不是"级别本身无信号"，是"单窗塔路径 + 第二类稀疏 + 窗口依赖"三者联合。这与 MEMORY `project_regime_is_level_truncation_artifact`（regime=级别截断伪影）同构——高级别信号分布是**塔截断/窗口起点的伪影**，非市场级别基本属性。

## 认识论等级与有效域

- **L2**（真实 BTC 数据，可否证）：跨窗对比是 L2 否定性结果——否证了 H1（门滤空）与"高级别真稀疏是基本属性"两个前置假设。
- **有效域边界**：本判定基于 300K vs 全历史两窗。未验证：(a) 全历史窗跑我的判别测试（O(n²) 461万 bar 数小时，不可行）——故"全历史 level2-4=0"依赖 acc-multilevel-mu 报告的既有数据，非本测试重跑；(b) 其他窗口大小（如 100K/1M）的 level 分布，可能进一步刻画窗口依赖曲线。
- **照实 161**：这是**否定性结果**（H1 被否证 + 稀疏归因被修正），比确认 H1 更有价值——它缩小了"高级别 alpha 否证"的有效域（该否证是单窗塔路径伪影，非级别本质）。

## 机制自验证（代码证据）——⚠ 排除主张被 codex 否定

> **codex 否定：下文引用的 bit-exact 测试是合成数据 L1（零信息），真实 CL 的 `bit_exact_per_bar` 未跑且代码留档 CL bar 1464 已知发散（incremental.rs:467）。用 L1 排除 L2 已知 bug = 认识论倒置。保留：窗口局部+因果（事实1）成立；删除有效性：事实2「bit-exact 排除增量 bug」不成立——见文末降级判定。**

判定的两个承重代码事实（读 `incremental.rs:57/85-91/139-152` 坐实）：

1. **塔构造窗口局部 + 因果**（classify_at:85-91）：`IncrementalClassifier::new(bars, config)` 的 `bars` = **窗口切片**。`classify_at(i)` = `parse_layer(&bars[..=i])` → 只用窗口内 ≤i 数据，**窗口前无历史**。300K 窗 bar0 = 切片 `bars[..=0]`（无前史）；全历史窗同一全局 bar 有 431万 bar 前史塔累积。**同一价格点两窗输入不同 ⟹ 塔不同**——这是判定的机制，且是**正确的因果设计**（无 look-ahead，639），不是 bug。

2. **增量塔 bit-exact == 全量重算**（incremental.rs:57 模块契约 + `bit_exact_per_bar`(真实CL数据) + `bit_exact_synthetic`(always-run) 逐 bar 断言）：增量链 `classify_with_tower_incremental` **逐 bar bit-identical** `classify_with_tower(parse_layer(bars[..=i]))` 全量重算。**⟹ 排除质询4（"增量塔在长历史下 stale/漂移使高级别塔退化"）**——增量路径可证等于全量重算，全历史 level 分布不是增量伪影，是真实的 `parse_layer(full_bars[..=i])` 塔产出。generation 快路(#104/#105/#106)只是 extract 缓存复用（同代次输出不变），不改塔内容。

**联合结论**：全历史 level2-4=0 是**真实塔产出**（非 bug、非增量漂移），其成因是"全历史单窗塔路径 + 第二类稀疏 + 窗口依赖"三联合。判定 = 塔构造窗口依赖（正确因果设计的自然后果），**非运行时 bug、非 H1 门滤空、非级别本质稀疏**。

## codex 异质确认（约束4，Task #8）——判定降级（fail）

codex 异质审查（.chanlun/review-results/codex-review-20260701-2042.md）**否定成立**，上文"非运行时 bug"的排除结论被降级。**保留**：H1（N^δ 门滤空）确实被 300K 数据否证（中间级通过门且配对成活，这点 codex 未推翻）；窗口依赖是真实代码机制（IncrementalClassifier::new 只接收窗口切片，codex 未推翻）。**降级**：从"坐实非 bug"降为"候选解释，非 bug 未证明"。

codex 三处承重推理破裂：

1. **bit-exact 认识论倒置（致命，质询3/4）**：上文"机制自验证"引用的排除证据（`bit_exact_synthetic` 等）是**合成数据 L1**（零信息增量，formalization-validity-domain）。真实 CL 的 `bit_exact_per_bar` 是 `#[ignore]` 未跑。**且代码自身留档已知发散**（incremental.rs:467-472 + mod.rs:952-958）：`classify_with_tower_incremental` 在真实 CL **bar 1464 与全量重算发散**（`detect_centers_windowed_resume` frontier 中枢 resume cursor 把未确认末窗口当 immutable，违反 bit-exact 充要条件#2；pre-existing，属上浮矛盾）。`diag_classifier_resume_frontier_divergence` 发现发散只打印不 panic ⟹ 不捕获。**用 L1 合成测试排除 L2 已知 frontier bug = 认识论倒置**。故"全历史 level 分布是真实塔产出、非增量伪影"**未证明**——BTC 全历史可能命中同类 frontier 发散使高级别塔退化。

2. **level5 交叉验证推理不完备（质询4）**：上文"300K level5=buy2 vs 全历史 level5=sell2 ⟹ 非同一信号被 bug 丢弃"只排除了"简单丢信号 bug"，**未排除 frontier 中枢 bug 改写高级别结构**——bug 可在产出内容改变的同时令 level2-4 消失，两窗 level5 不同恰是改写结果。不构成"非 bug"的证明。

3. **统计口径未对齐（质询1）**：全历史 level 分布来自 `decompose_capturable_spread` 后的 `decomps`（门后+配对后台账）按 (level,δ) 聚合；300K 诊断表有 bsp_pre/gamma_nonflat/sig_post 四层。本报告拿 300K 的 decomps 层（level2-4=1）对比全历史 decomps 层（level2-4=0）**层级对齐**（都是 decomps），但**未在全历史跑四层口径**——无法排除"全历史 bsp_pre 中间级也有，被 N^δ 门/配对滤到 decomps=0"（若如此则中间级空洞在全历史下部分是 H1 门滤，与 300K 的 level1 门滤同源）。

codex 唯一否定不成立项（质询2）：seen-set 键 `(lvl, source_index, bsp_class)` 的 `lvl` 字段真隔离跨级别，无坐标碰撞误去重。

### 降级后判定
- **确证（未被推翻）**：H1「N^δ 门把中间级严格滤空」被否证（300K level2-4 通过门+配对成活）；窗口依赖是真实机制。
- **未证明（降级）**："全历史 level2-4=0 = 窗口依赖非 bug"——已知 frontier 中枢发散 bug（CL bar 1464）未被系统性排除，全历史 level 分布可能含 bug 成分。
- **acc-multilevel-mu 的 level2+ 全否证归因仍待厘清**：Le Cam 硬墙（n<30）判定成立，但"高级别稀疏"的根因（窗口依赖伪影 vs frontier bug vs 真稀疏）**未定**。

### 结论翻转条件（codex 给出，本报告采纳）
窗口依赖判定升回"坐实非 bug" ⟺ 满足**两个**条件：
1. 全历史也跑 level-hole-dx 四层口径计数（bsp_pre/gamma_nonflat/sig_post/decomps 全报），确认全历史中间级 bsp_pre 分布；
2. 增量 vs 全量对拍验证 frontier 中枢 bug（incremental.rs:467 bar 1464 类）在 **BTC 数据已修复**（`bit_exact_per_bar` 在 BTC 上跑通不发散）。

二者任一不满足 ⟹ 判定停在"候选解释"。**这两条是后续工位（frontier bug 修复 = 已上浮矛盾；全历史四层重跑 = O(n²) 数小时）的输入，不在本诊断范围完成——诚实开口，不补丁遮盖。**

## 影响声明（修正）
- **本诊断的净产出**：否证 H1（门滤空）+ 暴露全历史 level 分布归因的**双重不确定性**（窗口依赖 vs frontier bug 未分离）。这本身是 L2 否定性结果（缩小了"高级别 alpha 否证"可依赖的确定性——该否证的根因未定，acc-alpha 不应把"高级别无 alpha"当已坐实前提）。
- **未解决**：全历史 level2-4=0 的确切根因（需上述两翻转条件）。frontier 中枢发散 bug（CL bar 1464，incremental.rs:467）是已上浮矛盾，其在 BTC 上的表现待验证。

