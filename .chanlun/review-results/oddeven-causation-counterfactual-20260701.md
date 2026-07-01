# 奇偶交替成因：反事实识别（机制 A/B/C）+ beta 剥离（L2，决定性否定性结果）

**认识论等级**：L2（真实数据单标的 BTC 全历史461万bar，反事实+随机置换，可产否定性结果，可复算）。
**commit**：b7cc9e9bdc（663口径）。
**方法来源**：编排者 ChatGPT 回应 docs/chanlun/external-consults/parity-phase-consult-20260701.txt（12/12 不可识别性→必须反事实）。
**诊断**：`tests/econ_oddeven_diagnosis.rs::oddeven_causation_counterfactual`（纯只读，不改核心，L1 自检 `counterfactual_helpers_l1` 已过）。
**复算**：`cargo test --release --features backtest_bin --test econ_oddeven_diagnosis oddeven_causation_counterfactual -- --ignored --nocapture`（全量~13.5min）。

## ★核心结论（修正上一轮，更严格且否定性）

上一轮结论（oddeven-mu-causation-20260701.md）：假设4（对称破缺/持有窗方向）是主因，恒等式 sign(μ̂)=sign(δ×持有窗净涨跌)。
**这一轮反事实推翻其"可交易结构"含义**：奇偶交替**不携带超过 regime/beta 漂移的方向结构信息** ⟹ 机制C（beta漂移）主导，非机制A（几何真结构）。

## 反事实1：多出场口径（识别机制 A vs B）

同一批信号（全历史12630条），3 种独立出场口径，看奇偶交替（sign μ̂(ℓ,δ)=sign(δ·(−1)^ℓ)）匹配率：

| 口径 | 描述 | 奇偶匹配 |
|---|---|---|
| 口径4 | 下一反向信号（现口径，价用确认bar） | **12/12** |
| 口径5 | 终点pivot（无确认滞后，价用pivot端点=a_b） | 8/12 |
| 口径6 H500 | 固定持有500bar（出场口径无关） | 8/12 |
| 口径6 H1000 | 固定持有1000bar | 9/12 |
| 口径6 H2000 | 固定持有2000bar | 10/12 |

**判定**：
- **不是纯机制B**：口径5/6（非"下一反向"口径）仍保留 8-10/12 交替，若纯出场口径产物应坍塌到随机(6/12)。
- **口径4的12/12是放大**：只有口径4（下一反向配对+确认价）达到完美12/12——配对边界恰好切在方向转换点+确认滞后叠加。换口径立刻降到8-10/12 ⟹ 12/12的**完美性**是口径4特有的放大，不是稳健结构。
- 口径5（去确认滞后）L0δ-1从−19翻+105、L5δ+1从−228翻+484 ⟹ 确认滞后是口径4完美交替的必要成分（呼应上一轮假设2的相位机制）。

## 反事实2：随机置换方向标签（ChatGPT 方法1，剥离 beta）—— ★决定性

保持每条信号 entry/出场/持有期/成本 + 买卖总数不变，只 shuffle δ 标签（200次固定种子 Fisher-Yates）。
统计量 S=Σ_class|μ̂|（真标签下方向携带的结构信息量）。

- **S_obs(真标签) = 1050.80**
- **S_perm均值 = 1154.83**（打乱δ后反而更大）
- **perm_p(#perm≥obs) = 0.6915**

**判定（否定性，决定性）**：
perm_p=0.69 ≫ 0.05，且 S_obs < S_perm均值 ⟹ **真方向标签携带的结构信息不超过随机置换，甚至略小**。
- 若机制A（几何真结构）：打乱δ应破坏结构⟹S_obs应显著大于S_perm⟹perm_p<0.05。**否证**。
- perm_p≈0.69 ⟹ 奇偶交替是 **beta/regime 漂移伪影（机制C）**：δ标签打乱后各级别持有窗的|平均PnL|不减反增——方向标签不是结构信息的载体，持有窗方向漂移（BTC单边趋势在各级别的投影）才是。

## 三机制最终判定

| 机制 | 判定 | 证据 |
|---|---|---|
| A 几何反相真结构 | **否证** | 随机置换 perm_p=0.69（方向标签不携超漂移结构信息） |
| B 出场口径产物 | **部分**（口径4放大器） | 口径4=12/12但换口径降8-10/12；口径5/6仍部分交替⟹非纯口径 |
| C regime/beta漂移 | **主因** | 置换检验坐实；恒等式 sign μ̂=sign(δ×持有窗净涨跌)+持有窗方向=BTC单边趋势投影 |

**综合**：奇偶交替 = BTC单边趋势(regime/beta) × 级别持有窗相位 × 口径4配对边界的**三重投影放大**，**不是可交易的方向alpha**。12/12的完美对称性是特定口径(下一反向+确认价)对beta漂移的放大伪影。

## 对编排者两问的最终回答

**问题1"这说明了什么"**：说明 663 的"6/12类μ̂>0"不是发现了6个可交易的级别方向信号——是 beta 漂移在(level,δ)网格上的确定性投影。μ̂正类不可作独立 entry 升基座（置换检验证明方向标签无超漂移信息，winner's curse 已被反事实坐实，非仅怀疑）。

**问题2"短差理论应该赚"**：ChatGPT 精确条件——往返振幅>入场滞后+出场滞后+成本，买卖两边**同时**成立。强趋势市（BTC全历史涨）只有顺大级别一边满足⟹逆大级别短差亏（编排者候选b"强上涨压制做空腿"数学成立）。缺 σ_higher 过滤（更高级别方向门控）⟹ 无法只在"往返regime"下双向做。短差双向赚需震荡/中枢往返regime + σ_higher过滤，非当前无门控全做。

## 正确操作（ChatGPT严格结论）
若OOS证 μℓ,δ>0 ⟺ δ=(−1)^ℓ：偶级别只做买、奇级别只做卖（χ=1[δ=(−1)^ℓ ∧ LCB>θ]）。
**但本反事实已否证其结构性**（perm_p=0.69）⟹ 不建议基于奇偶规律建仓，除非先加 σ_higher regime 门控 + OOS 重验。
**关键警告（ChatGPT）**：负类不能简单反做——X_inv=−X−2C，只有 μ<−2C 反向才可能正。正类交易、负类不交易（除非单独估 μ_inv）。

## 结果包六要素

1. **结论**：奇偶交替主因=**机制C（regime/beta漂移）**，非机制A（几何真结构，随机置换 perm_p=0.69 否证）。机制B（出场口径）是口径4的12/12放大器（换口径降8-10/12）。奇偶交替不是可交易方向alpha，是BTC单边趋势×级别持有窗相位×口径配对的三重投影。短差双向赚需往返振幅>双侧滞后+成本且买卖同时成立=震荡regime+σ_higher门控，强趋势市只单边满足。
2. **定义依据**：663 μ̂=Σactual_pnl/n；actual_pnl=δ(Pτout−Pτin)−Ce（econ_positive.rs:253）；奇偶严格含义 sign μ̂(ℓ,δ)=sign(δ·(−1)^ℓ)（ChatGPT parity-phase-consult）；随机置换=方法1（保入场/出场/持有/成本+买卖数,只shuffle δ）；ChatGPT beta修正 (μ++μ-)/2≈aℓ−C(方向中性edge)、(μ+−μ-)/2≈bℓ(beta漂移)。
3. **边界条件**：结论翻转条件——(a) L3跨标的若震荡型标的(非单边)随机置换 perm_p<0.05 则机制A在某regime成立(当前BTC单边域否证)；(b) 加 σ_higher regime门控后重跑,若门控内 perm_p<0.05 则往返regime下方向携结构信息；(c) n_perm=200 固定种子,增大置换数结论稳健性可查。
4. **下游推论**：663"6/12类μ̂>0"是 beta 投影非可交易信号⟹μ̂正类不可作独立entry升基座(置换坐实winner's curse,非仅怀疑,呼应 econpositive-walkforward-20260630)。要交易奇偶必须先加 σ_higher regime门控隔离往返段+OOS重验。负类不可反做(X_inv=−X−2C)。
5. **谱系引用**：663 econpositive；664对象错配；formalization-validity-domain(奇偶交替有效域<定义域,仅beta投影非结构);161(务实=留缺口);上一轮 oddeven-mu-causation-20260701(恒等式,本轮反事实修正其可交易含义);econpositive-walkforward-20260630(winner's curse先例)。**新概念候选(修正)**：奇偶交替=regime投影伪结构,建议genealogist评估"级别方向alpha"是beta漂移投影的谱系记录。
6. **影响声明**：tests/econ_oddeven_diagnosis.rs 新增 oddeven_causation_counterfactual + 3口径helper + counterfactual_helpers_l1(纯只读诊断,不改核心/decompose/TradeRecord,bit-exact不涉及)。**异质审查缺席**：Codex配额429耗尽(project_heterosource_openai_quota_exhausted),本反事实结论未经异质否定,待配额恢复补codex diagnose。
