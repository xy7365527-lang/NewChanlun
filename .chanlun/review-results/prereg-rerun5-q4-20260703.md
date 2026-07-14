# 预注册：rerun5 q4 口径 alpha 重跑（Task #146 ③，goal g-type1-canonical-centers a5）

**冻结时间**：2026-07-03（本文件 commit = 冻结点；跑数先于本 commit ⟹ 直接 fail）。
**继承**：`prereg-q4-fullpi-20260703.md`（388a9ebc16）七项硬要求 ①-⑦ **全文继承不改**——
gross_cap=true / μ 新口径功效 / 桶键三处同批 / CME-simple margin / TW 两类分叉 / LCB_OOS
三态判据（z_α=1.645，VALIDATED ⟺ powered ∧ LCB>0 ∧ perm_p<0.05）/ 四臂配置。
**跑批入口**：`wverify_run.rs::q4_fullpi_policy`（R2 臂位=旧 prereg §⑥ 表）+ R1/R3/R4 同批。

## 新增冻结项（本轮特有，旧 prereg 之上叠加）

1. **canonical 中枢链版本标记**：信号生产链 = 修复序 #142（§5 中枢延伸）→ #143（§6 走势
   分解+Q8）→ #144（局部趋势门+Type1Cand）→ #145（ac4 A/C 类型化+Q7 方向）→ **Q7-#1 裁定C**
   （fallback 单元非方向锚，codex-q7-fallback-20260703）。冻结 commit：**cdddaa78ac**（裁定C
   实装）+ 1464634a2f（探针接生产锚+dx 死门重封 160/657）。跑数所在 HEAD 必须包含这两个 commit。
2. **一类信号集（裁定C 后新基线，BTC 全历史 4613599 bar，level_signal_census_btc 实测）**：
   L0 575（b244/s331，与 #145 逐位不变=L0 豁免实证）/ L1 18（b7/s11）/ L2 14（b5/s9）/
   L3 7（b3/s4）/ L4 0，**一类总计 614**（旧 prereg 时代=1057）。任务描述中的「一类 1057
   新信号集」以本实测 614 订正——1057 是裁定C 前基线，其中 L≥1 的 77-95% 为 fallback 锚伪信号。
3. **与 q4 旧结果差分**：q4 旧跑（#135）信号集=1057 语义。本轮所有桶级差分按「裁定C 前/后」
   双列呈现；旧结论（wverify PASS 桶/663/667 μ̂ 表/perm_p）的有效域=裁定C 前语义（谱系素材
   已交 genealogist）。
4. **已知开放问题（诚实声明，非阻塞）**：dx 真封① 违反（sig_post_sum < n_signals，差 14）
   ——判别实验证实为 #142-#145 后既有潜伏（父 commit 9b06a7265c 差 17），dx 手写门探针未同步
   生产语义（675号探针分叉），Task #147 修复。alpha 跑批走生产 collect_signals 侧，不消费该
   探针；若 #147 修复发现生产侧漏/多信号，本 prereg 结果须随之重估（fail 条件 5）。

## 可证伪 fail 条件（继承 4 条 + 新增）

1-4. 同旧 prereg（跑数先于冻结 / 冻结后改文 / 主桶 perm_p=1.000 / R6 C3 非 0 上浮）。
   R6 基线更新：dx 死门新基线 routed=160/sub_bsp_type3_total=657（裁定C 后，1464634a2f）；
   C3 breakout 计数 lvl1 routed=237 new_center_exists=0 breakout_ok=0（300K 窗，恒 0 维持）。
5. #147 修复若定位到生产 collect_signals 侧缺陷 ⟹ 本轮 alpha 结果整体降级 INCONCLUSIVE，重跑。

## 认识论等级

冻结文档本身 L0（判据声明）；跑数产出 L2（BTC 单标的真实数据）/R4 为 L3（跨标的）。
否定性结果照实入结果包（rerun5-20260703.md）。

## 影响声明

冻结一类信号集 614 语义与判据版本；不改任何代码；alpha 跑数必须在本 commit 之后执行。
