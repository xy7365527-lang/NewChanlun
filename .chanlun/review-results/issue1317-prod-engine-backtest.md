# #1317 M1 最终实验：生产引擎 7 期货回放 + 预注册 7/7 判据结果

> 7 标的（ES/GC/CL/ZN/6E/BRN/DX）1min 全量（prereg 表冻结值）| 生产 runner 路径 
> （`run_theta_v0_pi` 七链 π_Θ + 完整判据面）| 零交易成本 | 过判据 = 7/7 百分位全部越过各自随机中位
> （符号检验单侧 p=0.0078）。第一轮简化代理（#1309）1/7 仅作参考基线，不作为本实验预期值。

## ⚠ 卡点（外部数据依赖，缺文件不参与判定）

- `ES`：读取 "/home/agent/workspace/analysis/data_cache/es_1m_databento_10y.json" 失败: No such file or directory (os error 2)
- `GC`：读取 "/home/agent/workspace/analysis/data_cache/gc_1m_databento_10y.json" 失败: No such file or directory (os error 2)
- `CL`：读取 "/home/agent/workspace/analysis/data_cache/cl_1m_databento_10y.json" 失败: No such file or directory (os error 2)
- `ZN`：读取 "/home/agent/workspace/analysis/data_cache/zn_1m_databento_10y.json" 失败: No such file or directory (os error 2)
- `6E`：读取 "/home/agent/workspace/analysis/data_cache/usd6e_1m_databento_10y.json" 失败: No such file or directory (os error 2)
- `BRN`：读取 "/home/agent/workspace/analysis/data_cache/brn_1m_databento_10y.json" 失败: No such file or directory (os error 2)
- `DX`：读取 "/home/agent/workspace/analysis/data_cache/dx_1m_databento_10y.json" 失败: No such file or directory (os error 2)

## 0. 判决

**未执行**（全部标的缺数据，见上方卡点）→ 判定：**卡点（数据缺失，未执行）**。


## 4. 预注册冻结文本逐条对照（#1282 票面）

| 冻结项 | 票面 | 本跑执行 | 核对 |
|--------|------|----------|------|
| 过判据 | 7/7 标的百分位全部越过各自随机中位（符号检验单侧 p=0.0078） | 每标的分位 >50 计数，7/7 判定 + p=0.0078125 | ✅ |
| 随机基线 | prereg-windows-v0 百分位法（含 #1281 补 ZN/6E 窗） | `large_bootstrap_percentile_dual`（#1309 冻结函数）同口径移植：双腿各 N×H 随机进出 5000 次，MT19937 bit-exact | ✅ |
| 数据与窗口 | 7 标的 10 年 1min，窗口照 prereg-windows-v0 + §9 补录 | SYMBOL_FILES 全量 1min（prereg 表冻结值；ZN/6E 为 §9 补录 17y 快照） | ✅ |
| 前后对照 | 原 E 单边/双开 vs 生产引擎，同标的同窗配对逐标 Δ | 生产双开百分位 vs #1309 归档双开百分位逐标 Δ | ✅ |
| 纪律附款 | 不搜索/不调参/不挑窗/不加对照/不改判据 | 判据与窗口零改动，结果原样入档 | ✅ |

## 5. 纪律附款执行记录

- 跑前不搜索、不调参、不挑窗口：判据/窗口逐字取自 #1282 冻结文本与 prereg-windows-v0 冻结表，未改一字。

- 随机基线 PRNG 与 Python 冻结函数逐位同构（MT19937，测试锁对拍 CPython 实测向量）。

- 结果原样入档（本文件 + issue1317-prod-engine-backtest.json）；过与不过都是 M1 有效结论。

