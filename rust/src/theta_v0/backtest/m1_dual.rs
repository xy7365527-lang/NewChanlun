//! M1 多空双开百分位测量（#1317，#1282 预注册判据）——生产引擎交易流上的**同口径**移植。
//!
//! #1282 预注册判据（冻结，跑前不改一字）：**7/7 标的百分位全部越过各自随机中位**
//! （符号检验单侧 p=0.0078125）。第一轮（#1309）/第二轮（#1312）在 analysis 简化代理
//! （`p3_random_gate_control.large_bootstrap_percentile_dual`）上测；本模块是**同一方法**
//! 在生产引擎交易流上的移植——「同预注册判据测量」的判据面一字不动，只换数据来源。
//!
//! ## 口径（与 Python 逐字同构，含 PRNG）
//!
//! - 真策略组合复利 = （长腿复利% + 空腿复利%）/ 2（双腿各满仓，`compute_dual_metrics`）。
//! - 随机基线 = 双腿各 N 笔 × 平均持有 H 根，entry 均匀随机（有放回、允许重叠），
//!   长腿收益 `close[e+H]/close[e]`、空腿收益 `close[e]/close[e+H]`，双腿各自复利后
//!   （长+空）/2 合并——与 `simulate_one_random_dual_gate` 逐字同构。
//! - 百分位 = `P(随机 ≥ 真实)` 的补（5000 次独立采样，`BASE_SEED + 2000 + k`）。
//! - **PRNG bit-exact**：CPython `random.Random(int)` = MT19937（`init_genrand(19650218)`
//!   + `init_by_array([seed])`）+ `_randbelow` 拒绝采样。本模块逐位复刻（测试锁对拍
//!   CPython 实测向量）⟹ 相同输入下 Rust 与 Python `large_bootstrap_percentile_dual`
//!   产出**逐位相同**的百分位读数——「没改判据」的最强机械证据。
//!
//! ## 认识论等级
//!
//! - 移植正确性：L0/L1（随机基线算法与 Python 冻结函数逐位等价，测试锁对拍）。
//! - 回测结论：L2（真实数据 7 期货 1min，生产引擎交易流，含否定性结果）。

use super::metrics::TradeRecord;

/// 固定基种子（`p3_random_gate_control.BASE_SEED`，#1282 冻结）。
pub const BASE_SEED: u32 = 20260609;
/// 双开 bootstrap 种子偏移（`p3_random_gate_control.DUAL_SEED_OFFSET`，#1282 冻结）。
pub const DUAL_SEED_OFFSET: u64 = 2000;
/// 百分位/p 值的稳定估计轮数（`p3_random_gate_control.N_BOOTSTRAP_LARGE`，#1282 冻结）。
pub const N_BOOTSTRAP_LARGE: usize = 5000;
/// P3 随机门控触发阈值：N < 10 统计功效过低，不跑随机门控（同 `m1_e_futures_dual_backtest`）。
pub const MIN_N_FOR_P3: usize = 10;

/// 秩统计结果（与 `p3_random_gate_control.Percentile` 逐字段同构）。
#[derive(Debug, Clone)]
pub struct Percentile {
    pub n_runs: usize,
    /// P(随机 ≥ 真实)：≈0.5 无择时；≈0 门控胜；≈1 门控负。
    pub p_random_ge_e: f64,
    /// E 在随机分布中的百分位（= (1 − p) × 100）。
    pub e_percentile: f64,
    /// 随机门控复利中位数（稳健中心）。
    pub rnd_median: f64,
    pub rnd_mean: f64,
    pub rnd_std: f64,
}

/// 生产交易流拆出的双开腿口径（与 `compute_dual_metrics` 同构）。
#[derive(Debug, Clone)]
pub struct DualLegMetrics {
    pub n_long: usize,
    pub n_short: usize,
    /// 平均持有根数（f64，喂 bootstrap 前 `round` 取整——同 `_avg_hold` + `int(round(·))`）。
    pub hold_long: f64,
    pub hold_short: f64,
    /// 长腿复利 %（`(Π exit/entry − 1) × 100`）。
    pub long_compound: f64,
    /// 空腿复利 %（`(Π entry/exit − 1) × 100`）。
    pub short_compound: f64,
    /// 组合复利 % =（长腿复利 + 空腿复利）/ 2。
    pub combined_compound: f64,
}

/// 从生产 `TradeRecord`（逐 fill 平仓事件）+ close 价格序列拆双开腿口径。
///
/// 每笔记录按 `long` 字段分腿；单笔收益口径 = close 比值（长 `exit/entry`、空 `entry/exit`），
/// 与 `compute_dual_metrics` 的 `pnl_pct = exit/entry − 1` 逐字同构（零成本、每腿满仓，
/// `qty` 不参与——随机基线同款「相同暴露」口径）。`prices` 为 close 口径（量化 tick ×
/// tick_size ≈ 原始 close）。
pub fn dual_leg_metrics(prices: &[f64], trades: &[TradeRecord]) -> DualLegMetrics {
    let mut n_long = 0usize;
    let mut n_short = 0usize;
    let mut hold_long_sum = 0usize;
    let mut hold_short_sum = 0usize;
    let mut long_eq = 1.0f64;
    let mut short_eq = 1.0f64;

    for t in trades {
        // 有效域：bar 下标在 prices 内（生产 runner 自产，恒真；防御性护栏）。
        if t.entry_bar >= prices.len() || t.exit_bar >= prices.len() || t.entry_bar >= t.exit_bar {
            continue;
        }
        let entry_px = prices[t.entry_bar];
        let exit_px = prices[t.exit_bar];
        if t.long {
            n_long += 1;
            hold_long_sum += t.hold_bars;
            long_eq *= exit_px / entry_px;
        } else {
            n_short += 1;
            hold_short_sum += t.hold_bars;
            short_eq *= entry_px / exit_px;
        }
    }

    let hold_long = if n_long > 0 {
        hold_long_sum as f64 / n_long as f64
    } else {
        0.0
    };
    let hold_short = if n_short > 0 {
        hold_short_sum as f64 / n_short as f64
    } else {
        0.0
    };
    let long_compound = (long_eq - 1.0) * 100.0;
    let short_compound = (short_eq - 1.0) * 100.0;

    DualLegMetrics {
        n_long,
        n_short,
        hold_long,
        hold_short,
        long_compound,
        short_compound,
        combined_compound: (long_compound + short_compound) / 2.0,
    }
}

// ──────────────────────────────────────────────────────────────────────────────
//  MT19937（CPython `random.Random` 逐位复刻，测试锁对拍 CPython 实测向量）
// ──────────────────────────────────────────────────────────────────────────────

const MT_N: usize = 624;

/// Mersenne Twister MT19937，种子路径逐位复刻 CPython `random.Random(int)`：
/// `init_genrand(19650218)` → `init_by_array([seed])`。本仓只需 int seed ∈ u32（bootstrap
/// 种子 = `BASE_SEED + 2000 + k`，k ≤ 4999 ⟹ < 2^32，key 恒为单元素数组）。
struct Mt19937 {
    mt: [u32; MT_N],
    index: usize,
}

impl Mt19937 {
    fn new(seed: u32) -> Self {
        let mut rng = Mt19937 {
            mt: [0u32; MT_N],
            index: MT_N,
        };
        rng.init_genrand(19650218);
        rng.init_by_array(&[seed]);
        rng
    }

    /// CPython `init_genrand`（MT19937 标准初始化）。
    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..MT_N {
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        self.index = MT_N;
    }

    /// CPython `init_by_array`（key_length = 1 时 `j` 恒 0，`+ init_key[j] + j` 退化为
    /// `+ seed`；`wrapping_sub(i)` = 64 位 wrap 截低 32 位 = 32 位 wrap）。
    fn init_by_array(&mut self, key: &[u32]) {
        let key_length = key.len();
        let mut i = 1usize;
        let mut j = 0usize;
        let mut k = MT_N.max(key_length);
        while k > 0 {
            self.mt[i] = (self.mt[i]
                ^ ((self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_mul(1_664_525u32)))
            .wrapping_add(key[j])
            .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= MT_N {
                self.mt[0] = self.mt[MT_N - 1];
                i = 1;
            }
            if j >= key_length {
                j = 0;
            }
            k -= 1;
        }
        let mut k = MT_N - 1;
        while k > 0 {
            self.mt[i] = (self.mt[i]
                ^ ((self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_mul(1_566_083_941u32)))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= MT_N {
                self.mt[0] = self.mt[MT_N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x80000000u32;
        self.index = MT_N;
    }

    fn twist(&mut self) {
        for i in 0..MT_N {
            let y = (self.mt[i] & 0x80000000u32) | (self.mt[(i + 1) % MT_N] & 0x7fffffff);
            let mut next = y >> 1;
            if y & 1 == 1 {
                next ^= 0x9908b0dfu32;
            }
            self.mt[i] = self.mt[(i + 397) % MT_N] ^ next;
        }
        self.index = 0;
    }

    fn genrand_uint32(&mut self) -> u32 {
        if self.index >= MT_N {
            self.twist();
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680u32;
        y ^= (y << 15) & 0xefc60000u32;
        y ^= y >> 18;
        y
    }

    /// CPython `getrandbits(k)` 快路径（k ≤ 32）。
    fn getrandbits(&mut self, k: u32) -> u32 {
        if k == 0 {
            return 0;
        }
        self.genrand_uint32() >> (32 - k)
    }

    /// CPython `_randbelow_with_getrandbits`（拒绝采样）。
    fn randbelow(&mut self, n: u32) -> u32 {
        let k = 32 - n.leading_zeros();
        let mut r = self.getrandbits(k);
        while r >= n {
            r = self.getrandbits(k);
        }
        r
    }

    /// CPython `Random.randint(a, b)`。
    fn randint(&mut self, a: u32, b: u32) -> u32 {
        self.randbelow(b - a + 1) + a
    }
}

/// 单腿随机事件（`p3_random_gate_control._leg_events` 逐字）：n 笔 × hold 根，entry
/// 均匀随机（有放回），返回 `(exit_bar, multiplier)` 按 entry 升序。
fn leg_events(
    prices: &[f64],
    n: usize,
    hold: usize,
    short: bool,
    rng: &mut Mt19937,
) -> Vec<(usize, f64)> {
    if n == 0 || prices.len() <= hold {
        return Vec::new();
    }
    let max_entry = prices.len() - hold - 1;
    let mut entries: Vec<usize> = (0..n)
        .map(|_| rng.randint(0, max_entry as u32) as usize)
        .collect();
    entries.sort_unstable();
    entries
        .into_iter()
        .map(|e| {
            let m = if short {
                prices[e] / prices[e + hold]
            } else {
                prices[e + hold] / prices[e]
            };
            (e + hold, m)
        })
        .collect()
}

/// 多空双开随机基线单次模拟（`simulate_one_random_dual_gate` 逐字，仅返回组合复利%——
/// 百分位只消费复利，不消费 max_dd）。
fn simulate_one_random_dual_gate(
    prices: &[f64],
    n_long: usize,
    hold_long: usize,
    n_short: usize,
    hold_short: usize,
    rng: &mut Mt19937,
) -> f64 {
    let long_events = leg_events(prices, n_long, hold_long, false, rng);
    let short_events = leg_events(prices, n_short, hold_short, true, rng);
    let mut events: Vec<(usize, f64, bool)> =
        Vec::with_capacity(long_events.len() + short_events.len());
    for (b, m) in long_events {
        events.push((b, m, true));
    }
    for (b, m) in short_events {
        events.push((b, m, false));
    }
    // 按 exit_bar 升序稳定排序（long 事件先于 short 事件入列 ⟹ 同 exit 时 long 在前，
    // 与 Python `events.sort(key=..)` 稳定排序一致）；乘法可交换，但复利值对乘法顺序
    // 敏感（浮点结合律），事件序一致 ⟹ 逐位一致。
    events.sort_by_key(|e| e.0);
    let mut long_eq = 1.0f64;
    let mut short_eq = 1.0f64;
    for (_, mult, is_long) in events {
        if is_long {
            long_eq *= mult;
        } else {
            short_eq *= mult;
        }
    }
    ((long_eq - 1.0) + (short_eq - 1.0)) / 2.0 * 100.0
}

/// 多空双开大样本 bootstrap → 真策略在随机双开分布中的秩（`large_bootstrap_percentile_dual`
/// 逐字；`n_runs` 参数化以便测试锁用小样本对拍 CPython）。
pub fn large_bootstrap_percentile_dual(
    prices: &[f64],
    n_long: usize,
    hold_long: usize,
    n_short: usize,
    hold_short: usize,
    e_compound: f64,
    n_runs: usize,
) -> Percentile {
    let mut comps: Vec<f64> = Vec::with_capacity(n_runs);
    let mut n_ge = 0usize;
    for k in 0..n_runs {
        let mut rng = Mt19937::new((BASE_SEED as u64 + DUAL_SEED_OFFSET + k as u64) as u32);
        let comp =
            simulate_one_random_dual_gate(prices, n_long, hold_long, n_short, hold_short, &mut rng);
        comps.push(comp);
        if comp >= e_compound {
            n_ge += 1;
        }
    }
    let p = n_ge as f64 / comps.len() as f64;

    let mut sorted = comps.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let median = if n == 0 {
        0.0
    } else if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };
    let mean = if comps.is_empty() {
        0.0
    } else {
        comps.iter().sum::<f64>() / comps.len() as f64
    };
    let rnd_std = if comps.len() < 2 {
        0.0
    } else {
        (comps.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (comps.len() - 1) as f64)
            .sqrt()
    };

    Percentile {
        n_runs,
        p_random_ge_e: p,
        e_percentile: (1.0 - p) * 100.0,
        rnd_median: median,
        rnd_mean: mean,
        rnd_std,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用 synthetic closes（与 CPython 对拍脚本逐位相同）。
    fn synthetic_closes() -> Vec<f64> {
        (0..100)
            .map(|i| 100.0 + 0.1 * i as f64 + 0.3 * ((i * 7) % 5) as f64)
            .collect()
    }

    /// MT19937 `getrandbits(32)` 对拍 CPython `random.Random(seed).getrandbits(32)`。
    #[test]
    fn mt19937_getrandbits_matches_cpython() {
        let mut r = Mt19937::new(20260609);
        assert_eq!(
            [r.genrand_uint32(), r.genrand_uint32(), r.genrand_uint32()],
            [1167368707, 702297164, 2136653691],
            "seed=20260609 前三个 genrand_uint32 与 CPython 逐位一致"
        );
        let mut r2 = Mt19937::new(20262609);
        assert_eq!(
            [
                r2.genrand_uint32(),
                r2.genrand_uint32(),
                r2.genrand_uint32()
            ],
            [3528085264, 2182703557, 722035055],
            "seed=20262609（BASE_SEED+DUAL_SEED_OFFSET）与 CPython 逐位一致"
        );
    }

    /// `randint(0, 100)` 对拍 CPython（拒绝采样路径）。
    #[test]
    fn mt19937_randint_matches_cpython() {
        let mut r = Mt19937::new(20262609);
        let draws: Vec<u32> = (0..5).map(|_| r.randint(0, 100)).collect();
        assert_eq!(
            draws,
            [65, 21, 36, 64, 65],
            "randint 序列与 CPython 逐位一致"
        );
    }

    /// `simulate_one_random_dual_gate` 单次模拟对拍 CPython 实测值。
    #[test]
    fn simulate_one_random_dual_matches_cpython() {
        let closes = synthetic_closes();
        let expected = [
            -0.33729459678484286,
            0.3204550749844326,
            0.34545454109050633,
            0.35032295385495904,
        ];
        for (k, exp) in expected.iter().enumerate() {
            let mut rng = Mt19937::new((BASE_SEED as u64 + DUAL_SEED_OFFSET + k as u64) as u32);
            let got = simulate_one_random_dual_gate(&closes, 3, 5, 2, 7, &mut rng);
            assert!(
                (got - exp).abs() < 1e-12,
                "k={k} 组合复利 {got} 应等于 CPython {exp}"
            );
        }
    }

    /// `large_bootstrap_percentile_dual` 小样本对拍 CPython 实测（n_runs=100）。
    #[test]
    fn large_bootstrap_percentile_matches_cpython() {
        let closes = synthetic_closes();
        let p = large_bootstrap_percentile_dual(&closes, 3, 5, 2, 7, 0.3, 100);
        assert_eq!(p.n_runs, 100);
        assert!(
            (p.p_random_ge_e - 0.65).abs() < 1e-12,
            "p 对拍 CPython 0.65：{}",
            p.p_random_ge_e
        );
        assert!(
            (p.e_percentile - 35.0).abs() < 1e-9,
            "百分位对拍 CPython 35.0"
        );
        assert!(
            (p.rnd_median - 0.3297242392608868).abs() < 1e-12,
            "rnd_median 对拍：{}",
            p.rnd_median
        );
        assert!(
            (p.rnd_mean - 0.050419927198168735).abs() < 1e-12,
            "rnd_mean 对拍：{}",
            p.rnd_mean
        );
        assert!(
            (p.rnd_std - 0.42742630883344923).abs() < 1e-12,
            "rnd_std 对拍：{}",
            p.rnd_std
        );
    }

    /// 退化臂：无交易（双腿 N=0）⟹ 组合复利 0、随机分布恒 0 ⟹ p=1、百分位 0。
    #[test]
    fn degenerate_no_trades() {
        let closes = synthetic_closes();
        let p = large_bootstrap_percentile_dual(&closes, 0, 5, 0, 7, 0.0, 50);
        assert_eq!(p.n_runs, 50);
        assert!((p.p_random_ge_e - 1.0).abs() < 1e-12, "全随机 ≥ 0 ⟹ p=1");
        assert!((p.e_percentile - 0.0).abs() < 1e-12, "百分位 0");
        assert!((p.rnd_median - 0.0).abs() < 1e-12 && (p.rnd_std - 0.0).abs() < 1e-12);
    }

    /// `dual_leg_metrics`：合成交易的长/空复利 + 组合复利口径。
    #[test]
    fn dual_leg_metrics_compound_caliber() {
        // prices: 100 → 110（+10%）→ 100（回 100）。
        let prices = vec![100.0, 110.0, 100.0];
        let trades = vec![
            // 多：entry 0 → exit 1（100→110，+10%）。
            TradeRecord {
                entry_bar: 0,
                exit_bar: 1,
                hold_bars: 1,
                qty: 1.0,
                long: true,
                forced_close: false,
            },
            // 空：entry 1 → exit 2（110→100 下跌，做空 110/100−1 = +10%）。
            TradeRecord {
                entry_bar: 1,
                exit_bar: 2,
                hold_bars: 1,
                qty: 1.0,
                long: false,
                forced_close: false,
            },
        ];
        let m = dual_leg_metrics(&prices, &trades);
        assert_eq!(m.n_long, 1);
        assert_eq!(m.n_short, 1);
        assert!((m.long_compound - 10.0).abs() < 1e-12, "长腿 +10%");
        assert!(
            (m.short_compound - 10.0).abs() < 1e-12,
            "空腿 +10%（110→100）"
        );
        assert!(
            (m.combined_compound - 10.0).abs() < 1e-12,
            "组合 =（10+10）/2"
        );
        assert!((m.hold_long - 1.0).abs() < 1e-12 && (m.hold_short - 1.0).abs() < 1e-12);
    }
}
