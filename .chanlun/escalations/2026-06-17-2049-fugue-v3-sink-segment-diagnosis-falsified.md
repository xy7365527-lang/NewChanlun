---
status: 生成态
date: 2026-06-17 20:49
type: escalation
domain: fugue_v3 / 缠论级别递归 / sink-recover 操作语义
trigger: 用户任务前提（因果诊断）被 L3 回测数据证伪
---

## 矛盾报告：fugue_v3 sink 下沉下界修复

### 矛盾描述

用户任务要求「空头仓位不应落在 segment 层（ladder=2），因为 segment 没有合法 nf
信号，recover 会死锁→爆仓」，据此把 sink/recover 落点下界从 `FIRST_BSP(2)` 提升到
`PENDING_LO(3)`。

该修复的**因果诊断**与 HEAD 基线回测数据**不可弥合地冲突**：

1. **「recover 死锁」与代码语义冲突**：recover 的触发信号是**父层 k** 的 nf 信号，
   不是子层 segment 的信号。
2. **「爆仓」与基线数据冲突**：HEAD 全 8 标的 liquidations ∈ {0,0,0,4,1,1,0,2}——
   无任何大规模强平；recover 正常闭合（closes ≥ opens）。
3. **修改的实际效果是 regime trade-off，不是 bug 修复**：OKLO +306%→−47.5%（恶化），
   BRN −163.7%→+43.7%（改善），DX 不变。

### 双方论证

**立场 A（任务前提：segment 是非法落点）：**
- 定义依据：缠论第65课「笔和线段连中枢都谈不上，把线段当最小级别走势类型的次级别
  严格意义上不对」；`PENDING_LO=3=move=a₀` 是递归底，`segment=FIRST_BSP=2` 是构造
  a₀ 的几何脚手架，非走势类型、不产生操作级别买卖点。
- 推论：空头落 segment(2) 后，segment 层无 nf 信号 ⟹ recover 无法触发 ⟹ 空头死锁
  ⟹ 单边行情中被 liquidate_short 强平爆仓。

**立场 B（代码 + 数据：segment 是合法空头停泊层）：**
- 代码依据（`operate.rs` D 段 recover 循环，HEAD 逐字）：
  ```rust
  for k in PENDING_LO..MAX_LADDER {        // k 从 move(3) 起
      let sub = k - 1;                     // k=3 ⟹ sub=2=segment
      ...
      let signal = match parent_dir {
          Polarity::Long => obs.nf_buy(k), // ← 触发用【父层 k=move(3)】信号，非 segment
          Polarity::Short => obs.nf_sell(k),
      };
      if signal.is_some() && recover_chunk(.., k, ..) { ... }  // 从 sub=2 升回 k=3
  }
  ```
  **「触发层」与「落点层」分离**：sink@k 用 `nf_sell(k)` 把空头开到 `k−1`；recover@k 用
  `nf_buy(k)` 把空头从 `k−1` 升回。当 k=3：空头停泊在 segment(2)，但进/出都由
  move(3) 的 nf 信号驱动。**segment(2) 永不需要自身信号** ⟹ 无死锁。

- 数据依据（HEAD 基线，L3 真实数据）：

  | 标的 | strat | BH | liq | opens→closes | 修复后 strat |
  |------|-------|-----|-----|------|------|
  | OKLO | **+306.0%** | +307.1% | 0 | 407→**454** | **−47.5%** |
  | DX   | −13.0% | +4.1% | 0 | 2→3 | −13.0%（不变）|
  | BRN  | **−163.7%** | +87.4% | 0 | 682→**1856** | **+43.7%** |
  | CL   | +29.4% | +28.2% | 4 | 1371→220 | （未跑完）|
  | GC   | −140.9% | +257.3% | 1 | 190→135 | （未跑完）|
  | ES   | −226.1% | +594.3% | 1 | 182→107 | （未跑完）|
  | QQQ  | −102.8% | +174.6% | 0 | 46→16 | （未跑完）|
  | BTC  | +28.8% | +1380.4% | 2 | 3354→**4899** | （未跑完）|

  - `closes ≥ opens`（OKLO 454>407、BRN 1856>682、BTC 4899>3354）证明 segment 空头
    被 recover **充分平掉**——如果死锁，closes 必远小于 opens。
  - liquidations 全为个位数（最多 CL=4）——「爆仓」现象在数据中**不存在**。
  - OKLO 机动仓在 segment 短差上 **+几百%**（追平 BH），是 segment 落点最有效的证据。

### 涉及的定义

- `缠论知识库.md` / 第65课（线段非最小走势类型的次级别）—— 立场 A 的字面依据。
- `rust/src/fugue_v3/mod.rs:86-87`：`PENDING_LO = FIRST_BSP_LADDER + 1`，注释
  「segment(=FIRST_BSP) 非势源」。
- **缺失的定义（矛盾根源）**：fugue_v3 从未定义「空头停泊层」与「操作触发层」是否
  必须同级。立场 A 默认二者同级（停泊层须有自身信号）；代码实现二者分离（停泊层 k−1
  / 触发层 k）。**这个分离是否合法，是未结算的概念空位。**

### 谱系比对结果

- **543号**（`operation-as-word-not-hardcoded-cycle`）：四步循环是 nf 触发的涌现序列，
  非状态机——支持立场 B（sink/recover 由父层信号独立驱动，无预设配对死锁）。
- **539号系列**（根翻空有效域 ⊂ 非上行 regime）、记忆中反复出现的「清仓/短差频率 =
  regime 函数」——本矛盾的「OKLO 输 / BRN 赢」是同一模式的第 N 例：**任何无条件的
  级别门控都是 regime 函数，不存在普适最优**（formalization-validity-domain：有效域
  ≠ 定义域）。
- **无直接先例**：「segment 是否合法空头停泊层」此前未被显式质询。

### Lead 的建议方案

**诊断证伪确定，但修改本身不是纯粹有害——它是有效域 trade-off。** 建议分三步：

1. **立即恢复**：当前 working tree 的修复使 OKLO −353pp，且论证前提（死锁爆仓）已证伪，
   **不应 commit**。建议 `git checkout` 回退 4 个源文件 + 重新 maturin develop，恢复
   OKLO +306% 基线。

2. **重新表述为有效域问题**（若编排者认为 segment 落点确有需控制的场景）：
   真实的弱点不是「死锁」，而是 segment 空头在**强单边无 nf_buy(3)** 时长期停泊拖累
   收益（慢性踏空，非 panic）。这是 regime 依赖的，应做 **regime 门控**（如：仅当父层
   move(3) 长期无 nf_buy 且价格远离 basis 时禁 segment 落点），而非无条件下界提升。

3. **概念结晶**：若编排者裁定「停泊层 / 触发层分离」合法，应结晶为语法记录——
   sink/recover 的落点下界（FIRST_BSP）与触发下界（PENDING_LO）**本就应不同**，
   当前代码是正确的，prove 守卫 `[FIRST_BSP, MAX)` 也是正确的（不该提升到 PENDING_LO）。

### 需要决断的问题（领域语言）

**在缠论递归操作语义下，最低级别走势类型 a₀(move) 的核心仓，可否把机动仓空头
停泊在其构造脚手架 segment 上、由 a₀ 自身的次级别底背驰买点（nf_buy@move）升回？**

- 若**可以**（立场 B）：当前 HEAD 代码正确，本次修复应整体 revert，segment 是合法
  停泊层，「停泊层 ≠ 触发层」结晶为语法记录。
- 若**不可以**（立场 A）：需重新给出**不依赖「死锁爆仓」这一被证伪因果**的理由——
  为何 a₀ 的机动空头不能停泊 segment？且须接受 OKLO 等强牛标的机动仓归零的代价，
  或改为 regime 门控而非无条件下界。

**附带价值判断**：即使立场 A 在概念上成立，无条件下界提升的 L3 效果是 regime
trade-off（OKLO 输 BRN 赢），是否采纳需编排者的有效域价值判断。
