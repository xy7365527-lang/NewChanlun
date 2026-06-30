# Diagnose: pi_bsp_timing.rs（goal #5 π^bsp 忠实度）— 20260629-1820

## ★异质源不可用声明（诚实性前置）
**Codex API 配额耗尽**（429 `insufficient_quota`，gpt-5.3→5.2 fallback 同配额，无跨 provider 异质源）。
本轮**异质否定无法执行**。以下是**同质审查**（code-verifier 工位自身代码层分析 + 真实数据实证），
不是异质审查。异质审查待配额恢复后补做。不伪装异质否定成立（llm-role-boundary：说真话比漂亮假话好）。

## build/test 状态
- HEAD b29d3d4b19 `cargo build --lib`：退出码 0（仅 20 warnings，无 error）。
- `cargo test --lib`：1215 passed, 0 failed, 67 ignored。孤儿 R/S 已 commit，**无回归**。

## 五质疑诊断结果

### 质疑1（随机对照结构性退化）→ **实证推翻**（同质审查误判）
预测：所有 trade exit_bar=n-1 ⟹ span=min_entry+1≈1 ⟹ shift_degenerate 恒真。
实测（ES/BTC/GC/DX 2024-06 窗口）：**controls_degen = false**（全部）。
原因：声部 entry_bar 分散（首个买卖点要等结构形成 ⟹ min_entry 足够大 ⟹ span≥2）；多数声部 hold<len。
**质疑1的强形式被真实数据否定。** 随机对照对 pi_bsp 未结构性失效。

### 质疑2（L2 等级膨胀）→ **不成立，L2 标注正确**
实测：theta_same_caliber 全为负（-0.0017~-0.0056），shift/indep mean 多正，p 全 0.87~0.99，
beats_random=false（全部）。**这是有价值的否定性 L2 结果**——缠论买卖点择时在这些窗口不贡献 alpha（甚至负）。
L2 = 可否证，这里被否证了 ⟹ L2 标注成立（formalization-validity-domain：否定性结果 > 确认性结果）。

### 质疑3（口径双重性 trades 强平 vs net_per_bar MtM）→ **存在，但 ceiling 已声明**
trade_pnls 用 entry→n-1 close 强平（含浮盈），net_per_bar 用逐 bar MtM。两套 PnL 口径不同。
但 trade_pnls 仅喂 metrics::compute 的 win_rate/profit_factor + significance 随机对照；
strat_return/Sharpe 走 equity_curve（MtM）。注释第308-310行如实声明了「平仓未存 exit_bar ⟹ 强平」。
**非 coordinate fork**（两者都走生产 prices[i]，无第二坐标系）；是口径分工，已声明。

### 质疑4（§16 父子建模）→ **★真实缺陷：§16 多空双开是死代码 + 声明膨胀**
实测：**短差子声部数 = 0（ES/BTC/GC 全部）**。§16 子声部机制从未触发。
根因（代码层）：
- 出场逻辑（L199-215 先平后开）：多头遇卖证书先被 closing 移除。
- 子声部逻辑（L223/L240）：open_new 查 active 里 pdir==-dir 的反向根作父。
- **冲突**：同一反向证书既触发父出场（§5）又应触发开子（§16）。先平后开顺序下父先死 ⟹
  open_new 时 active 已无反向根 ⟹ parent 恒 None ⟹ 永远开根，从不开子。
- §9「先平后开」与 §16「父 active 时开反向子」结构性互斥 ⟹ §16 路径不可达。
**声明膨胀**：模块 doc + 注释（L17/L36/L223）声明实装了 §16 多空双开，输出还印「短差子声部数」，
但该机制是死代码。ceiling 注释**未声明** §16 不可达。这违反 formalization-validity-domain（声明 > 实际）。

### 质疑5（type 一买/二买/三买区分丢失）→ **存在，且未在 ceiling 声明**
cert_of 只读 conf_plus/conf_minus（买侧/卖侧析取），class_index 仅用于 seen-set 去重。
丢弃了 I_γ⊆{1,2,3} 的 type 区分（一买=趋势反转/二买=回调/三买=中枢突破，操作语义不同）。
§1-2 的 b_ℓ 6 维向量被坍缩为 2 维（买/卖侧）。
ceiling 注释（L28-36）**未声明** type 坍缩 ⟹ 声明不完整。影响：择时把三类买卖点同质化，
可能是 theta_same_caliber 为负的部分原因（二买/三买的操作语义与一买不同，统一当根入场可能错配）。

## 判定（对象否定的对象——我对代码的否定，非异质）
- 质疑1/2：实证推翻，无问题。
- 质疑3：已声明，非 fork，合格。
- **质疑4：真实缺陷 + 声明膨胀（§16 死代码未声明）— HIGH**。
- **质疑5：真实缺陷 + 声明不完整（type 坍缩未声明）— MEDIUM**。

## 边界条件（否定翻转条件）
- 质疑4 翻转：若 §16 设计意图是「子声部在父出场的**后续** bar 开」（非同 bar），则需另一 carrier 路径，
  当前实现仍不支持（无延迟开子机制）⟹ 不翻转。若 PDF §9/§16 本就规定先平后开优先于多空双开，
  则短差子声部=0 是**正确**的（§16 在该顺序下本就罕见）⟹ 那么缺陷降为「声明应说明 §16 实际罕见/不可达」。
  **此分支需 PDF §16 原文裁决**——属定义层，建议 source-auditor 核 PDF §9 vs §16 优先级。
- 质疑5 翻转：若 goal #5 只问「买卖点方向（买/卖侧）是否携带择时 alpha」而非「三类各自」，则 type 坍缩是
  合法简化，只需在 ceiling 补声明即可（非缺陷，是未声明的合法简化）。

## 影响声明
- 涉及模块：rust/src/bin/pi_bsp_timing.rs（唯一改动文件，工位 R 产出）。
- 不涉及定义文件改动。质疑4/5 的「PDF 优先级裁决」属定义层，需 source-auditor + PDF 原文。
- 异质审查（Codex）待配额恢复补做——本记录是同质审查，已标注等级。
