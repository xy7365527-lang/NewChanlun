//! 分账本头寸空间模块（工作单元 R2，C25/C26/C29）。
//!
//! 缠论完全分类的**分账本扩展**（§一–§十四）：多空作独立头寸坐标，不净额抵消。本模块与
//! `strategy::ledger`（净额账本 R=Π-A-W）、`nautilus::account_adapter`（Nautilus net_position）
//! **正交并置**——分账本头寸空间 P^sep 是与净额账本不同有效域的新代数结构，承载"每声部多/空两
//! 独立腿"。净额映射 Net 是从 P^sep 到 ℤ 的**有损投影**（双开 (Q,Q)↦0 退化，230号直积退化谱系）。
//!
//! ## 子模块
//! - [`separate`]：C25 分账本头寸空间 P^sep（`Leg`/`SepPosition`）+ C26 净额映射 `Net`（有损）+
//!   C29 分账本吃到 `Eat^sep`（向上元素多头腿覆盖、向下空头腿覆盖、不抵消）+ C32/C39 不删父算子。
//!
//! ## 契约锚（只读，不改 .lean）
//! - C25/C26 → `formal/Origin/SeparateLedger.lean`（`Leg`/`SepPosition`/`legNet`/`Net`）。
//! - C29 → spec §四 页13（`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`）+
//!   对照 `formal/Origin/VoiceEat.lean`（声部版 `Eat`，C06）。
//!
//! ## 认识论等级（formalization-validity-domain / 231号）
//! 全模块 **L0/L1**（结构镜像 = 验证管线正确性，零信息增量）——**不**声明分账本实盘有效/双开盈利
//! （那是 L2 EmpiricalDomain）。spec §十四：数学名称是"分账本声部级全元素覆盖"，**不是**"净资产级每笔盈利"。

pub mod separate;
