//! ★重（Chong）——持久操作单位一等实体（SPEC #847 S1，实施票 #879）。
//!
//! ## 教义正本锚
//!
//! **重 = ⟨标的 S，操作级别 ℓ，专属筹码 Q⟩**，三要件缺一不可（[ADR 0010] §一，#842 R-1/R-11）：
//!
//! > `040-第40课.md:20`【正文】「可以变成 N 重层次的操作，**每一重都对应着一定的资金与筹码**」
//!
//! **重是持久的**：一重按自己的级别在自己的标的上反复进出，每次进出生成一个声部，
//! **声部死而重不死**（`CONTEXT.md` 的「声部」是一次性实体，出场即终结；二者不是同一物，
//! 重是声部的容器，不是声部的替代）。
//!
//! ## 本模块落的四条裁定
//!
//! 1. **键 = (标的, 操作级别)，成本状态不进键**（[ADR 0013] 裁定二）：成本状态是重的
//!    **状态**（会变），键标识「这是哪一重」（不能变）——否则同一重降成本前后会变成两重。
//! 2. **各重的钱独立**（[ADR 0013] 裁定一）：N 份专属筹码 + 一个全局上限（**不等式**，
//!    不是把一份钱切 N 块）；各重开户时拿到自己那份筹码，互不挪用。
//! 3. **各重保证金逐仓分开**（#842 R-8/R-12，[ADR 0014] 裁定二）：`posted = Σₖ nₖ/L_maxₖ`
//!    每层按各自极性**全额计提**，净额省下的差额**不得**折算为可部署名义——省下的钱
//!    不许再用，这本身就是闸门（不另设仲裁或额度消耗机制）。
//! 4. **重内单向 / 重间不仲裁**（[ADR 0014] 裁定一/二）：一重内部塔的各结构级别合成后
//!    必须同向，只在 `[0, 满仓]` 加减、**不穿零**——合成层的实装见
//!    [`super::coverage::enforce_chong_unidirectional`]；两重相反时两边都执行，唯一跨重
//!    约束 = 逐仓计提 + 全局不等式上限。
//!
//! [ADR 0010]: ../../../../../docs/adr/0010-chong-persistent-operating-unit.md
//! [ADR 0013]: ../../../../../docs/adr/0013-partition-machine-bidirectional-form.md
//! [ADR 0014]: ../../../../../docs/adr/0014-intra-chong-unidirectional-inter-chong-no-arbitration.md

use std::collections::BTreeMap;

/// 重的键 = **(标的, 操作级别)**（ADR 0013 裁定二：成本状态不进键）。
///
/// `Ord` 派生保 `BTreeMap` 确定序（bit-exact 可复现，同 `level_ledger` 的确定序纪律）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChongKey {
    /// 标的 S（重绑死标的：本仓只做第一利润最大定理，不做换股——`CONTEXT.md`「重」条目）。
    pub symbol: String,
    /// 操作级别 ℓ（外生参数，与塔的结构级别同名异物——#842 R-2）。
    pub op_level: u8,
}

/// 重 = ⟨标的 S，操作级别 ℓ，专属筹码 Q⟩（ADR 0010 §一）。
///
/// 持久实体：跨越多次建仓而不终结。成本/盈亏是**状态**（随操作变），不进键。
#[derive(Debug, Clone, PartialEq)]
pub struct Chong {
    /// 专属筹码 Q（名义美元）——各重的钱独立，互不挪用（ADR 0013 裁定一）。
    quota_usd: f64,
    /// 当前持仓（有符号 lot：正=多 / 负=空 / 0=空仓）。
    ///
    /// **重内单向**（ADR 0014 裁定一）：一份专属筹码只有一个持仓状态——同一时刻该重的
    /// 所有成分恒同号；要同时持多与持空需要两份筹码，那就是两个重。该不变式由合成层
    /// [`super::coverage::enforce_chong_unidirectional`] 保证，本字段只承载结果。
    position_lots: f64,
    /// 成本基（状态，不进键）：三阶段（降成本→退本金→增股数）的锚，实例单位 = 重
    /// （ADR 0013 裁定二）。本票只立实体与键，三阶段改挂归 SPEC #847 S2。
    cost_basis_usd: f64,
    /// 累计已实现盈亏（状态，不进键）。
    realized_pnl_usd: f64,
}

impl Chong {
    /// 开户：一重拿到自己那份专属筹码（Q>0）。
    pub fn new(quota_usd: f64) -> Result<Self, String> {
        if !quota_usd.is_finite() || quota_usd <= 0.0 {
            return Err(format!("重的专属筹码必须为正有限值，got {quota_usd}"));
        }
        Ok(Self {
            quota_usd,
            position_lots: 0.0,
            cost_basis_usd: 0.0,
            realized_pnl_usd: 0.0,
        })
    }

    pub fn quota_usd(&self) -> f64 {
        self.quota_usd
    }

    pub fn position_lots(&self) -> f64 {
        self.position_lots
    }

    pub fn cost_basis_usd(&self) -> f64 {
        self.cost_basis_usd
    }

    pub fn realized_pnl_usd(&self) -> f64 {
        self.realized_pnl_usd
    }
}

/// 重簿：N 个重 + 一个全局名义上限（**不等式**约束，ADR 0013 裁定一）。
///
/// 当前生产只有一个重（一条主干塔 + 一个 π loop，无操作级别概念——SPEC #847 S3 才给塔
/// 加操作级别挂载点），键用 [`ChongBook::legacy_single_key`] 占位；簿的 N 重语义（逐仓
/// 计提 + 全局不等式）在类型层已定型，多重实例化随 S3 一并落地。
#[derive(Debug, Clone)]
pub struct ChongBook {
    chongs: BTreeMap<ChongKey, Chong>,
    /// 全局名义上限（美元）：Σ 各重已用 ≤ 上限。不是把一份钱切 N 块的等式配额。
    global_notional_cap_usd: f64,
}

impl ChongBook {
    /// 单重簿的占位标的面（#879）：fill loop 不持有标的名（runner 层才有 `Dataset.symbol`），
    /// 而单重簿内符号维度**无任何消费**（价格/杠杆经按键闭包供给，单重簿只有一个键）。
    /// 多重落地时簿的构造上移到 runner（那里有 symbol），本常量随之退役。
    pub const LEGACY_SYMBOL: &'static str = "<legacy-single-chong>";

    /// 现状单重占位键（#879）：全账户 = 一个重。操作级别维度取 0 仅为占位——
    /// **塔目前没有操作级别概念**（#826 实测每级延伸窗口占比 48%–55%，一级都没关过），
    /// 真操作级别键随 SPEC #847 S3 落地后替换。**不得以该占位值推理任何级别语义。**
    pub fn legacy_single_key(symbol: &str) -> ChongKey {
        ChongKey {
            symbol: symbol.to_string(),
            op_level: 0,
        }
    }

    pub fn new(global_notional_cap_usd: f64) -> Result<Self, String> {
        if !global_notional_cap_usd.is_finite() || global_notional_cap_usd <= 0.0 {
            return Err(format!(
                "全局名义上限必须为正有限值，got {global_notional_cap_usd}"
            ));
        }
        Ok(Self {
            chongs: BTreeMap::new(),
            global_notional_cap_usd,
        })
    }

    /// 开户注册一重（键已存在 = 重复开户，fail-loud）。
    pub fn register(&mut self, key: ChongKey, quota_usd: f64) -> Result<(), String> {
        if self.chongs.contains_key(&key) {
            return Err(format!(
                "重键重复注册：{key:?}（键 = (标的, 操作级别)，成本状态不进键）"
            ));
        }
        self.chongs.insert(key, Chong::new(quota_usd)?);
        Ok(())
    }

    pub fn get(&self, key: &ChongKey) -> Option<&Chong> {
        self.chongs.get(key)
    }

    pub fn len(&self) -> usize {
        self.chongs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chongs.is_empty()
    }

    /// 更新一重的持仓（有符号 lot）。未知键 = 未开户先持仓，fail-loud。
    pub fn set_position_lots(&mut self, key: &ChongKey, lots: f64) -> Result<(), String> {
        let c = self
            .chongs
            .get_mut(key)
            .ok_or_else(|| format!("未注册的重键持仓更新：{key:?}"))?;
        c.position_lots = lots;
        Ok(())
    }

    /// **各重保证金逐仓分开**（#842 R-8/R-12，ADR 0014 裁定二）：
    /// `posted = Σₖ |nₖ|·pxₖ/L_maxₖ`——每层按**各自极性全额计提**。
    ///
    /// 反向两重（一多一空同标的同手数）posted = 两腿之和，**不是**净额 0；净额省下的
    /// 差额不得折算为可部署名义（见 [`Self::deployable_notional_usd`]）。
    pub fn posted_margin_usd(
        &self,
        px_of: &dyn Fn(&ChongKey) -> f64,
        l_max_of: &dyn Fn(&ChongKey) -> f64,
    ) -> f64 {
        self.chongs
            .iter()
            .map(|(k, c)| {
                let l_max = l_max_of(k);
                debug_assert!(
                    l_max > 0.0,
                    "#879：L_max 必须为正（杠杆上限），got {l_max}（{k:?}）"
                );
                c.position_lots.abs() * px_of(k) / l_max
            })
            .sum()
    }

    /// 各重名义全额合计 `Σₖ |nₖ|·pxₖ`（逐仓口径的名义敞口；风控门的保证金输入，
    /// 替代「全账户净 lot × mark」——#834 定 `admission.rs` 落点随本票一并改）。
    pub fn posted_notional_usd(&self, px_of: &dyn Fn(&ChongKey) -> f64) -> f64 {
        self.chongs
            .iter()
            .map(|(k, c)| c.position_lots.abs() * px_of(k))
            .sum()
    }

    /// 可部署名义 = 全局上限 − Σ 各重已计提（**不等式**扣减）。
    ///
    /// 净额省下的差额**不回流**：反向两重净敞口为 0 时，可部署仍按两腿全额扣——
    /// 「省下的钱不许再用」即跨重唯一闸门（ADR 0014 裁定二，不另设仲裁/额度消耗机制）。
    pub fn deployable_notional_usd(
        &self,
        px_of: &dyn Fn(&ChongKey) -> f64,
        l_max_of: &dyn Fn(&ChongKey) -> f64,
    ) -> f64 {
        (self.global_notional_cap_usd - self.posted_margin_usd(px_of, l_max_of)).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(sym: &str, lvl: u8) -> ChongKey {
        ChongKey {
            symbol: sym.to_string(),
            op_level: lvl,
        }
    }

    const PX: f64 = 100.0;
    const L_MAX: f64 = 10.0;

    fn px_of(_: &ChongKey) -> f64 {
        PX
    }
    fn l_max_of(_: &ChongKey) -> f64 {
        L_MAX
    }

    /// AC1-a：对称多空两腿各自全额计提——反向两重 posted = 两腿之和，不是净额 0。
    #[test]
    fn posted_margin_symmetric_legs_full_polarity() {
        let mut book = ChongBook::new(1_000_000.0).unwrap();
        let long_k = key("BTC", 1);
        let short_k = key("BTC", 2);
        book.register(long_k.clone(), 500_000.0).unwrap();
        book.register(short_k.clone(), 500_000.0).unwrap();
        // 对称：各 50 lot（一多一空），同价同杠杆上限。
        book.set_position_lots(&long_k, 50.0).unwrap();
        book.set_position_lots(&short_k, -50.0).unwrap();
        // 净额口径：|50 + (−50)| × px / L = 0（被否决的旧口径）。
        // 逐仓口径：两腿各自全额 50×100/10 = 500，posted = 1000。
        let posted = book.posted_margin_usd(&px_of, &l_max_of);
        assert_eq!(
            posted, 1_000.0,
            "#879：各重保证金逐仓分开——反向两腿各自全额计提，不得净额抵消"
        );
        // 名义合计同理：Σ|nₖ|·px = 10,000，非 |Σ nₖ|·px = 0。
        assert_eq!(book.posted_notional_usd(&px_of), 10_000.0);
    }

    /// AC1-b：净额省下的差额不得折算为可部署名义。
    #[test]
    fn net_savings_not_deployable() {
        let cap = 2_000.0;
        let mut book = ChongBook::new(cap).unwrap();
        let long_k = key("BTC", 1);
        let short_k = key("BTC", 2);
        book.register(long_k.clone(), 1_000.0).unwrap();
        book.register(short_k.clone(), 1_000.0).unwrap();
        book.set_position_lots(&long_k, 50.0).unwrap();
        book.set_position_lots(&short_k, -50.0).unwrap();
        // posted = 500 + 500 = 1000 ⟹ 可部署 = 2000 − 1000 = 1000。
        // 若按净额（0）计提则可部署 = 2000——那 1000 差额即「净额省下的钱」，
        // 按裁定**不得**回到可部署名义。
        assert_eq!(book.deployable_notional_usd(&px_of, &l_max_of), 1_000.0);
        // 平仓一空腿后：posted = 500，可部署 = 1500（只回流真平仓的那条腿）。
        book.set_position_lots(&short_k, 0.0).unwrap();
        assert_eq!(book.deployable_notional_usd(&px_of, &l_max_of), 1_500.0);
    }

    /// AC1-c：键 = (标的, 操作级别)，成本状态不进键——成本变化不改「这是哪一重」。
    #[test]
    fn cost_state_not_in_key() {
        let mut book = ChongBook::new(1_000_000.0).unwrap();
        let k = key("BTC", 3);
        book.register(k.clone(), 100_000.0).unwrap();
        // 同一键：成本状态演进（降成本）后仍定位到同一重。
        {
            let c = book.chongs.get_mut(&k).unwrap();
            c.cost_basis_usd = 42_000.0; // 状态变
            c.realized_pnl_usd = 8_000.0;
        }
        let c = book.get(&k).expect("成本变化后同键必须仍是同一重");
        assert_eq!(c.cost_basis_usd(), 42_000.0);
        assert_eq!(c.realized_pnl_usd(), 8_000.0);
        // 键的构成只有 (标的, 操作级别)：不同操作级别 = 另一重。
        let other = key("BTC", 4);
        assert!(book.get(&other).is_none());
        assert_ne!(k, other);
    }

    /// 各重的钱独立：重复开户 fail-loud；专属筹码必须为正。
    #[test]
    fn register_independent_quota_fail_loud() {
        let mut book = ChongBook::new(1_000_000.0).unwrap();
        let k = key("BTC", 1);
        book.register(k.clone(), 100_000.0).unwrap();
        assert!(book.register(k.clone(), 1.0).is_err(), "重复键开户必须拒绝");
        assert!(
            book.register(key("BTC", 9), 0.0).is_err(),
            "零筹码开户必须拒绝"
        );
        assert!(book.register(key("BTC", 9), f64::NAN).is_err());
        // 未开户先持仓 fail-loud。
        assert!(book.set_position_lots(&key("ETH", 0), 1.0).is_err());
    }

    /// 单重退化 = 旧「全账户净 lot」同值（只有一个重时逐仓口径与净额口径重合——
    /// 这正是 `admission.rs` 旧行「只有一个重时的正确实现」的语义刻录）。
    #[test]
    fn single_chong_degenerates_to_account_net() {
        let mut book = ChongBook::new(100_000.0).unwrap();
        let k = ChongBook::legacy_single_key("BTC");
        book.register(k.clone(), 100_000.0).unwrap();
        book.set_position_lots(&k, -7.5).unwrap();
        assert_eq!(book.posted_notional_usd(&px_of), 7.5 * PX);
        assert_eq!(book.posted_margin_usd(&px_of, &l_max_of), 7.5 * PX / L_MAX);
    }
}
