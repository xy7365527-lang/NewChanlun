# p110 装配工件审计：链断在装配层而非数据层的可能性（只读）

日期：2026-07-17 ｜ 数据：btc_1m_full.json（4,613,599 bar）｜ 侧信道：`/tmp/p92_ckpt_dump.txt`
方法：源码通读（nest.rs / p92_nest_replay_postruling.rs / level_view.rs）+ dump 离线复算
（分析器 `/tmp/p110_ckpt_audit.py`）。未重跑全量重放——门检数字引 p92-postfix 回归同二进制输出。

## F0 术语固定：judge_at 的三套语义（数据事实）

同名字段在三条路径上取值不同，混读即生"工件"：

1. **provider 初值** = 快照 `view.query.as_of`（level_view.rs:590、:649）。
2. **主路径回填钟** = prefix 遍中该 EventKey 首见 bar（`book.divergences`/`book.candidates`
   的 `or_insert` 首见保持，p92 bin:479-486；回填 bin:191-197；observe_snapshot 再登记
   bin:718-733）。`P92_RULE` 自述 `clock=first_prefix created_at=forbidden`。
3. **CKPT 侧信道** = 快照原值不回填，"judge_at 取快照原值，不参与对账"（bin:841 注释）。

快照版本机制：provider 语义钉版 `C2VersionTuple::auto_pairing`（level_view.rs:63-68，
GGDD_V1/MOVE_BLOCK_AC_V1/EXACT_THREE_V3）；塔代次 `forest_epoch`（classifier/mod.rs:931）
+ `signal_signature`（bin:395-416）作 prefix 遍重投影触发器（bin:468-469）。
**结构非 append-only**：prefix 塔前沿可被后续数据重写（幽灵即其产物，见 (b)）。

## (a) judge_at 时钟分配能否把本可成链的父子错配？

### 结论：装配层无钟门，钟在机制上不可能断链；实测 A 边 100% 逆行但全部只入 sidecar

- **机制（数据事实，源码）**：typed 装配门只有方向一致 + `is_sub` 结构包含
  （nest.rs:548-562）；`judge_at` 只 `push` 进 sidecar（nest.rs:561），
  `d3_descent_stats` "只计数，不参与证书真值"（nest.rs:440-445）。排序键为结构
  `sel_key`（nest.rs:544-547），不含钟。严格装配同构（nest.rs:798-822，"confirm_src
  仅登记，绝不参与否决"）。单测固定此性质：父钟 200 > 子钟 100 仍成链
  （nest.rs:1492-1515）。旧装配同理（nest.rs:1372-1380 测试）。
- **实测（数据事实，dump）**：66 张终端证书共 25 条多 rung 边（A 24 / B 1），
  **24/25 钟逆行**（父 judge_at > 子 judge_at，Δ=3,711–160,206 bar；脚本 (a1)），
  全部在 A 口径；唯一 B 边正序（dump:806 `judge_at=152680,153753`）——与主路径
  `P92_D3 edges=1 violations=0`（回归 L12）逐边互证：D3 只数 B，B 唯一边恰好正序。
  例：dump:807 `judge_at=795426,736245,707523`（A exec=1 top=3 Long）。
- **钟空洞=0（数据事实）**：judge_at==4613598 兜底 0 例（脚本 (a2)）；与
  `unresolved_targets=0`（p92-postfix 回归 L9）互证——所有终态目标在 prefix 遍
  均以匹配 flag 首见登记（bin:484-486 解析规则）。
- **钟不早于结构（数据事实）**：judge_at ≥ turn_source 且 ≥ interval_b.hi 全 66 张成立
  （脚本 (a2)），与 `P92_SNAPSHOT future_violations=0`（回归 L10）一致。
- **逆行归因（口径产物，非装配工件）**：Trend 事件可发射需 C 终段完整
  （forward-find `segment.end_index <= as_of`，level_view.rs:694-698），级别越高段
  闭合越慢；而 Trend 事件 turn_source=seg_c.1（level_view.rs:589）可远在子级首见之前。
  judge_at=「终态形态首见」而非「背驰发生时刻」，父子逆行是该认识论语义的必然产物。
- **遗留未分离量（诚实边界）**：触发门（bin:468-469）只在 forest_epoch/signature 变化时
  重投影，若签名漏变可致首见晚登；dump 无法把「触发延迟」从「结构闭合延迟」中分离。
  未证伪此支路，但 `unresolved_targets=0` + 空洞=0 限定其只可能加晚、不可能吞事件。

**对 (a) 的裁定建议**：钟逆行/晚登不否决链（机制保证），但使 judge_at 向量不可作
「背驰可证时序」直接消费——p106 入场锚、D3 逐边时序、任何信号延迟分析若按
judge_at 排序父子，会把口径产物读成市场事实。**待裁定**：judge_at 语义披露
（更名/加注「终态形态首见」），D3 可测边定义是否改用 turn_source 之外的见证。

## (b) 快照版本身份迁移是否导致链身份对不上而丢链？

### 结论：丢链证伪——67 并集 = 66 终端 + 1 幽灵，幽灵以新身份在终端复现

- **复算 p103（数据事实，独立重算）**：CKPT 并集 67 键，终端 66，并集−终端=1，
  **终端−并集=0**（脚本 (b1)）。唯一消失键：dump:35
  `3:725489:724160-725489|2:704358|1:706241`（A, 750000 单点出现）；其基例
  `1:706241` 在终端以 top `3:689893:688090-689893` 复现（dump:807）。
- **迁移普查（数据事实）**：同 (caliber,exec,top,side,基例) 身份向量 ≥2 种者全史仅
  此 1 例（脚本 (b2)）；同键非连续出现（结构抖动身份未迁）0 例（脚本 (b3)）。
  exec-base 身份 append-only 成立；**top 身份不 append-only**。
- **迁移机制（数据事实，源码）**：塔 L3 结构前沿未定型，后续数据重写中枢分解后
  forward-find 重选 C 终段（level_view.rs:691-699 注释即此裁定史）；
  turn_source 向后迁移 35,596 bar（725489→689893），EventKey 变（bin:57-66 定义）
  ⟹ 旧键消失、新键首见。方向注意：迁移**向过去**重锚，新身份的 turn 更早。
- **对账口径风险（装配工件，已 mitigation）**：CKPT 快照间身份可比性依赖
  EventKey 全等；p103 条款建议（以 exec-base 为稳定主键、top 仅归因字段）正确且必要。
  任何按身份向量 join 检查点史的下游（如把 p102 式归因扩展到时间维）必须按
  基例后缀匹配而非全向量匹配，否则把迁移误读为「撤销+新建」。
- **能力边界**：本审计只证「检查点史上无净丢链」；终端快照内部是否存在
  「结构曾可成链、终态被重写后不再可成链」的反向案例，CKPT 侧信道可答——
  答案是否（消失键=1 且已匹配），但检查点粒度 250k bar，粒度内瞬态不可见。

## (c) 「终态快照+prefix 首见拼合」的区间几何是否系统性偏移？

### 结论：几何不偏移（区间即终态真值），偏移的是钟——系统性晚登，且随级别放大

- **拼合构造（数据事实，源码）**：证书区间全取终态快照事件字段（typed_interval，
  nest.rs:448-458）；钟由 prefix 遍按 **EventKey 全等**登记（bin:57-66 键定义、
  479-486 登记）——终态几何的任何中途形态都不登钟，只登终态形态的首见。
- **偏移量（数据事实，dump）**：lag = judge_at − interval_b.hi（= judge_at − turn_source，
  两者实测逐元素相等）：
  - 基例：A p50=2,177 / mean=4,347 / max=22,499；B p50=3,548 / mean=5,580；
  - 父级 rung：A p50=19,348 / mean=28,597 / **max=105,533**（即幽灵链 L3 元素，
    dump:807，turn=689,893 → judge=795,426）；
  - B 唯一 rung lag=5,554。
- **性质判定（口径产物）**：晚登主量级来自结构闭合语义（C 终段须完整、级别越高越慢）
  与迁移重锚（(b)），非登记 bug；未来违例=0 保证钟不反向作假。
- **拼合的系统性后果（待裁定）**：以终态几何回贴 prefix 钟，等价于宣称「该证书在
  judge_max 时刻以其终态形态可知」——对 66/66 unfold_exact（p103）成立；但对
  「该链最早何时以任意形态可成链」系统性高估，高估幅度与身份稳定性负相关。
  若 p106 用 judge_at 推入场锚/确认延迟，此偏移全量进入结论。**待裁定**：
  是否需要在证书 sidecar 增补「任意形态首见」见证钟，或明确禁用 judge_at 作锚。

## 对主线问题的回答

「链断在装配层」对**产量证伪**：装配无钟门、无快照丢链、无几何吞并
（dedup 键全结构字段，bin:682-685；provider complete=true）；66 张 = p92 装配口径
（terminal_bits_new、无 intake fallback）下终态结构可产全集——与 nest 91 张之差是
已记录的进料口径差异（p103 口径差异记载，末端 `2:4613084` 簇），不属装配断链。
低产量归因应回到数据事实（B 口径窄、级别谱 L0 不监听、回试段裁定②）而非装配层。

「链断在装配层」对**诊断语义证实**两处：(1) D3 可测边=1 是 B 口径链深谱
（24/25 单级）的直接产物，p103「D3 无单调约束」的可测样本几乎全在 A 口径；
且唯一 B 边正序 ⟹ 主路径 violations=0 不携带「时序良态」任何信息（n=1）；
(2) judge_at 向量的 A 边 100% 逆行 + 级别放大晚登，使其当前形态不宜作时序证据消费。

## 发现清单

| # | 发现 | 定性 |
|---|------|------|
| 1 | typed/严格装配门均不含 judge_at（nest.rs:548-562,798-822） | 数据事实 |
| 2 | 24/25 多 rung 边钟逆行（A 边全逆行，Δ≤160,206；唯一 B 边正序） | 数据事实 |
| 3 | 钟空洞=0、未来违例=0、judge_at≥结构右端全成立 | 数据事实 |
| 4 | 逆行主因=级别闭合延迟+终态形态首见语义 | 口径产物 |
| 5 | CKPT 并集 67=终端 66+幽灵 1；终端−并集=0，无净丢链 | 数据事实 |
| 6 | top 身份非 append-only（迁移向过去重锚 35,596 bar） | 数据事实 |
| 7 | 检查点史按身份全向量 join 会把迁移误读为撤销 | 装配工件 |
| 8 | 拼合钟系统性晚登：基例千级、父级万级、迁移例 10.5 万级 | 口径产物 |
| 9 | judge_at 语义披露/「任意形态首见」见证钟是否增补 | 待裁定 |
| 10 | 触发门延迟与结构闭合延迟在 dump 中不可分离 | 诚实边界 |

## 复现

    python3 /tmp/p110_ckpt_audit.py            # 读 /tmp/p92_ckpt_dump.txt
    # dump 再生成（同 p103，~40min+）：
    # P92_CKPT=250000 ./target/release/p92_nest_replay_postruling \
    #   analysis/data_cache/btc_1m_full.json > /tmp/p92_ckpt_dump.txt

关键行锚：FALLBACK×2 dump:751-752；终端 66 证 dump:753-818；唯一正序 B 边
dump:806；最长逆行链（幽灵终端形）dump:807；幽灵原形 dump:35。
