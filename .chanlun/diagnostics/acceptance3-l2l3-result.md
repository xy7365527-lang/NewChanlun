# acceptance[3] L2/L3 全窗重判 — π_Θ^bsp 多声部状态机 OOS 结果

**工位**: swarm/l2l3-rejudge-2 (L0.3) | parent_callback: main
**日期**: 2026-06-30
**对象**: `rust/src/bin/pi_bsp_timing.rs`（π_Θ^bsp 买卖点多声部离散择时状态机，goal#5）
**测量口径**: NAV net-position MtM（§18 r^bsp=N^bsp·ΔP−C），同工位 L 测 π^cov 口径，Sharpe 可比
**随机对照**: n_beats_random 双口径（shift + indep），seed 冻结，MIN_TRADES=30 守

---

## 1. 结论

**当前引擎性能下，π^bsp 全窗 L2 重判仅能推进到 1/8（OKLO），其余 7/8 因 O(N²)/超线性墙诚实标 blocked。**

| # | 品种 | bar 量级 | 状态 | Sharpe | strat_return | BH | beats_random (shift/indep p) | 子声部 | 分层诊断 |
|---|------|---------|------|--------|--------------|-----|------------------------------|--------|----------|
| 1 | **OKLO** | 343,282 (~0.65y) | **L2 跑完** | 0.0503 | −1.5195 | +2.0564 | **false** (0.999 / 1.000) | 18 | 产得出·不盈利 |
| 2 | QQQ | ~1.6M (108MB) | **blocked** | — | — | — | — | — | 25min TIMEOUT_EXIT=124 |
| 3 | DX | (132MB) | blocked | — | — | — | — | — | 引擎性能未达，> QQQ |
| 4 | BRN | (149MB) | blocked | — | — | — | — | — | 引擎性能未达，> QQQ |
| 5 | CL | (225MB) | blocked | — | — | — | — | — | 引擎性能未达，> QQQ |
| 6 | GC | (239MB) | blocked | — | — | — | — | — | 引擎性能未达，> QQQ |
| 7 | ES | (250MB) | blocked | — | — | — | — | — | 引擎性能未达，> QQQ |
| 8 | BTC | (329MB) | blocked | — | — | — | — | — | 引擎性能未达，> QQQ |

**认识论等级**：OKLO=**L2（单标的，否定性结果，有信息增量）**。8 品种齐了才是 L3——**当前 L3 未达成**。

### OKLO L2 详情（确凿，唯一跑完）
```
bar 数          : 343282     年化基数: 0.6527
声部总数        : 652（多 323 / 空 329）
短差子声部数    : 18（§6/§16 σ_u=−σ_p）★真激活
净额峰值 |N^bsp|: 26.00
方向证书总数    : 966   hostOf 命中: 966 (100.0%)
命中且有父容器  : 313   父容器有 active: 0
★严格可提升 Lift: 17（Context^δ_j∧Fresh 双门）
Sharpe=0.0503  strat_return=−1.5195  BH=+2.0564
Sortino=0.1889  MaxDD=1.5159  win_rate=0.4663  PF=0.9045
theta_same_caliber=−0.0615
shift  mean/p   : −0.025794 / 0.9990
indep  mean/p   : +0.009398 / 1.0000
beats_random    : false    controls_degen: false
```

## 2. 定义依据

- π^bsp = §12 净额 N_t^bsp = Σ_{v∈A} σ_v q_v（声部状态机持仓净额，**非** §1 覆盖 Σ_e s_e ε_e）。
- §19：π^cov≤0 不能推出 π^bsp≤0 —— 本工位单独测 π^bsp 回答 goal#5（与 l3_fullwindow 测的 π^cov v1 是**不同对象**，见 [[project_stheta_v1_fullwindow_l3_falsified]]）。
- 子声部「真激活」依据：§3 position instance v=(c_v,η_v,σ_v,n_v) + §6/§16 短差子声部 σ_u=−σ_p。OKLO n_children=18>0 ⟹ task#17 Lift 谓词链（Context^δ_j∧Fresh）收紧后子声部仍**结构性可达且非空**。
- 「不盈利」依据：§4 随机对照协议 beats_random=false（shift p=0.999, indep p=1.000，两口径均不显著），且 strat_return=−1.52 ≪ BH=+2.06。

## 3. 边界条件（结论翻转条件）

- **L3 达成翻转**：acceptance[4] 全 O(n)（当前 bsp~n^1.26 超线性残余）后重跑——届时 QQQ/DX/BRN/CL/GC/ES/BTC 7 品种可在合理时限跑完，1/8 → 8/8 即 L3。本工位 blocked 标记的唯一解除条件是引擎性能达 O(n)。
- **OKLO alpha 翻转**：若 OKLO 在更长/更短窗口或不同费率下 beats_random 转 true，则「不盈利」结论翻转。当前 shift/indep 双口径 p≈1.0，距翻转极远。
- **子声部激活翻转**：若 task#17 之后某次重编译使 Lift 谓词链改动，n_children 可能再变（前任 20→现 18），但 >0 的「真激活」结论稳定。

## 4. 下游推论

- π^bsp 在 OKLO 上「产得出（子声部 18 真激活）但不盈利（beats_random=false）」——与前任 OKLO 结论同向（前任 Sharpe=−0.0466 子声部 20；本次 task#17 Lift 收紧 + task#14 K_i carrier forest 后 Sharpe=0.0503 子声部 18，符号微正但 beats_random 仍 false）。
- **L2 否定性结果对 OKLO 这一标的成立**：声部状态机的多声部对冲层（§16 多空双开）在 OKLO 1min 全窗激活但不携带 alpha。**不可外推到其他 7 品种**（formalization-validity-domain：有效域=OKLO 单标的，非 8 品种定义域）。
- goal#5（π^bsp 是否携带 π^cov 没有的 alpha）**在全 8 品种上仍未回答**——唯一跑完的 OKLO 给出否定信号，但 1/8 不构成 L3 鲁棒否证。

## 5. 谱系引用

- [[project_stheta_v1_fullwindow_l3_falsified]]：l3_fullwindow 测的是 **π_Θ^cov v1**（8/8 否证），与本工位测的 **π^bsp** 是不同对象（645/§19：cov≤0 ⊬ bsp≤0）。本工位是 π^bsp 的首次全窗尝试。
- 231 号（formalization-validity-domain）：OKLO L2 否定性结果有信息增量（缩小有效域边界），blocked 的 7 品种不外推。
- task#17（Lift 谓词链 Context_j+Fresh）：子声部从 20→18 的收紧来源。task#14（K_i carrier forest）：host-miss 修复使 hostOf 命中 100%。

## 6. 影响声明

- **未改动任何代码**——本工位是纯回测测量（行动类，018 四分法）。
- 产出：本结果文件 + OKLO L2 数据点。
- blocked 标记给 acceptance[4]（全 O(n) 引擎优化）一个明确的下游消费者：性能达标后本工位可一键推进到 L3。

---

## 方法论纠错记录（诚实）

第一次 QQQ 跑用 `... | tail -40; echo "EXIT=$?"`，捕获的是 pipe 末端 tail 的退出码（恒 0），**误读**为完成。第二次直接 `timeout 1500 ... > file; echo "TIMEOUT_EXIT=$?"` 正确捕获到 **124**（timeout 杀），坐实 QQQ 撞墙。教训：管道场景下 `$?` 取末命令退出码，测超时必须直接捕获 timeout 退出码。
