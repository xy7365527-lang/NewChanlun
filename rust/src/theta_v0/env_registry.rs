//! C2：`theta_v0` 全部直读 env 键的单一登记处（map #743 七票序第 3 张，spec #756）。
//!
//! ## 它解决什么
//!
//! `theta_v0` 全树散落 46 个 env 键直读（fable 三轮勘察原报 45，2026-07-29 复核实数订正为
//! 46——`THETA_LINEAGE_TRACE` 经 [`lineage_book`](super::lineage_book) 内部 `env_flag` helper
//! 间接调用，字面量 grep 首轮漏检）。行为门（`*_SKIP`/`*_GATE`/`*_PRESET` 类，counterfactual
//! 反证常例——金标准锚流程——的核心工具）与观测门（dump 路径/窗口截断/剖析计时/诊断断言）混杂，
//! 无处可枚举——审计「当前有哪些行为开关、默认臂是什么」此前只能全仓 grep。
//!
//! ## 形状（spec #756 C2 形状条，票内烤定）
//!
//! **静态表 + 只读查询口**：[`REGISTRY`] 是编译期字面量数组（46 项，完备性见 [`tests`]），
//! [`all`]/[`by_kind`]/[`find`] 是运行期可枚举/可按类过滤的只读查询口。每个键的 env 名字面量
//! 下沉为本模块 `pub const`——原全仓调用点从裸字符串字面量改引用本模块常量。**零行为**：
//! 迁移只换字面量的单一持有点，不改各调用点的读取语义/默认臂/缓存机制（`.ok()`/`.is_err()`/
//! `.map(...)`/`thread_local` 缓存等各自保留原样）。
//!
//! ## 五列语义
//!
//! - **键名**：env 变量字面量（[`EnvKeyMeta::key`]，本模块 `pub const`）；
//! - **语义**：一句话数用途（[`EnvKeyMeta::semantic`]）；
//! - **行为门｜观测门**（[`EnvKeyMeta::kind`] / [`GateKind`]）：**行为门** = 翻转生产/测试判定
//!   路径的开关，改变计算结果（counterfactual 反证工具本体）；**观测门** = 只影响
//!   dump/打印/诊断断言/窗口截断，不改变主路径判定输出；
//! - **默认臂**：未置该 env（或非法值）时的语义（[`EnvKeyMeta::default_arm`]）；
//! - **所属层**（[`EnvKeyMeta::layer`]）：`classifier` / `backtest` / `lineage`
//!   （[`lineage_book`](super::lineage_book)）/ `cross`（跨层共享同一键，如 `OPSEM_DUMP_DIR`
//!   同时被 `classifier::rebase_txn` 与 `backtest::opsem_dump` 读）。层次纪律「classifier 不可读
//!   backtest 门 env」（`backtest::admission::chain_driven_level_projection` 头注释）指的是
//!   **行为门**跨层直读——层门配置面已收敛为 `nest_cert_gate_enabled()` 单一派生点，classifier
//!   侧只读 `ThetaConfig` 机制位；`cross` 标注的三个键（`OPSEM_DUMP_DIR` /
//!   `THETA_CASCADE_EPROBE` / `THETA_PROFILE_STAGES`）均为**观测门**，不违反该纪律。

/// 行为门＝翻转生产/测试判定路径的开关（counterfactual 反证工具）；
/// 观测门＝只影响 dump/打印/诊断断言/窗口截断，不改变主路径判定输出。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateKind {
    Behavior,
    Observation,
}

/// 单键登记条目——五列的机器可读形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvKeyMeta {
    pub key: &'static str,
    pub semantic: &'static str,
    pub kind: GateKind,
    pub default_arm: &'static str,
    pub layer: &'static str,
}

// ─────────────────────────────────────────────────────────────────────────────
//  键名字面量——原全仓散落的 env::var("...")/var_os("...") 字面量的唯一持有点。
//  调用点改引用这些常量，字面量本身不变（零行为）。
// ─────────────────────────────────────────────────────────────────────────────

// -- lineage（theta_v0/lineage_book.rs）--
pub const THETA_REBASE_MIGRATE_SKIP: &str = "THETA_REBASE_MIGRATE_SKIP";
pub const THETA_REBASE_MIGRATE_STRICT: &str = "THETA_REBASE_MIGRATE_STRICT";
pub const THETA_LINEAGE_TRACE: &str = "THETA_LINEAGE_TRACE";

// -- classifier --
pub const THETA_T3INC_SKIP: &str = "THETA_T3INC_SKIP";
pub const THETA_CASCADE_FULLCLEAR: &str = "THETA_CASCADE_FULLCLEAR";
pub const CENSUS_WINDOW: &str = "CENSUS_WINDOW";
pub const DIAG_CANDCACHE: &str = "DIAG_CANDCACHE";
pub const DIAG_L0UNITS: &str = "DIAG_L0UNITS";

// -- cross（classifier + backtest 共读同一键）--
pub const OPSEM_DUMP_DIR: &str = "OPSEM_DUMP_DIR";
pub const THETA_CASCADE_EPROBE: &str = "THETA_CASCADE_EPROBE";
pub const THETA_PROFILE_STAGES: &str = "THETA_PROFILE_STAGES";

// -- backtest：行为门 --
pub const VOICE_EXEC: &str = "VOICE_EXEC";
pub const THETA_NEST_CERT_GATE: &str = "THETA_NEST_CERT_GATE";
pub const THETA_ENTRY_STOP_RECHECK_SKIP: &str = "THETA_ENTRY_STOP_RECHECK_SKIP";
pub const THETA_HISTBIND_REROUTE_SKIP: &str = "THETA_HISTBIND_REROUTE_SKIP";
pub const THETA_CENTER_OSCILLATION: &str = "THETA_CENTER_OSCILLATION";
pub const KAPPA_BARRIER_NUM: &str = "KAPPA_BARRIER_NUM";
pub const KAPPA_BARRIER_DEN: &str = "KAPPA_BARRIER_DEN";
pub const ENFORCE_GROSS_CAP: &str = "ENFORCE_GROSS_CAP";
pub const THETA_DIR_PRESET: &str = "THETA_DIR_PRESET";
pub const M7_WITNESS_A10: &str = "M7_WITNESS_A10";

// -- backtest：观测门（dump 路径/sidecar/剖析）--
pub const THETA_OTHERWISE_DOMAIN_SIDECAR: &str = "THETA_OTHERWISE_DOMAIN_SIDECAR";
pub const THETA_STRICT_NEST_SIDECAR: &str = "THETA_STRICT_NEST_SIDECAR";
pub const OPSEM_GAMMA_DUMP_DIR: &str = "OPSEM_GAMMA_DUMP_DIR";
pub const T5A_CHAIN_DUMP_DIR: &str = "T5A_CHAIN_DUMP_DIR";
pub const T5A_CHAIN_DUMP_PATH: &str = "T5A_CHAIN_DUMP_PATH";
pub const M8_WIN_FILTER: &str = "M8_WIN_FILTER";
pub const ENTRY_STOP_REVERSE_DUMP: &str = "ENTRY_STOP_REVERSE_DUMP";
pub const DELTAFREE_DUMP: &str = "DELTAFREE_DUMP";
pub const THETA_V0_SHADOW_DIVERGENCE_PATH: &str = "THETA_V0_SHADOW_DIVERGENCE_PATH";
pub const M8_FEE_DATUM: &str = "M8_FEE_DATUM";
pub const M8_LEVEL_CAP: &str = "M8_LEVEL_CAP";
pub const M8_SYMBOL: &str = "M8_SYMBOL";
pub const M8_REPORT_PATH: &str = "M8_REPORT_PATH";

// -- backtest：test-only 窗口/参数脚手架（观测门）--
pub const ECON_LEDGER_CSV: &str = "ECON_LEDGER_CSV";
pub const ECON_C3_OVERLAP_PROBE: &str = "ECON_C3_OVERLAP_PROBE";
pub const ECON_L2_MAX_BARS: &str = "ECON_L2_MAX_BARS";
pub const ECON_L2_TRAIN_FRAC: &str = "ECON_L2_TRAIN_FRAC";
pub const ECON_WF_WINDOWS: &str = "ECON_WF_WINDOWS";
pub const PERM_XPROC_SEED: &str = "PERM_XPROC_SEED";
pub const S2_BARS: &str = "S2_BARS";
pub const S3_BARS: &str = "S3_BARS";
pub const A0_PROFILE_BARS: &str = "A0_PROFILE_BARS";
pub const A3_PROFILE_BARS: &str = "A3_PROFILE_BARS";
pub const BITEXACT_BARS: &str = "BITEXACT_BARS";
pub const M7_WITNESS_BARS: &str = "M7_WITNESS_BARS";
pub const NEST_GATE_SMOKE_BARS: &str = "NEST_GATE_SMOKE_BARS";
pub const NEST_GATE_SMOKE_START: &str = "NEST_GATE_SMOKE_START";
pub const T1_PROBE_BARS: &str = "T1_PROBE_BARS";
pub const T1_PROBE_START: &str = "T1_PROBE_START";

// ─────────────────────────────────────────────────────────────────────────────
//  静态表——五列全键在案。
// ─────────────────────────────────────────────────────────────────────────────

pub static REGISTRY: &[EnvKeyMeta] = &[
    EnvKeyMeta {
        key: THETA_REBASE_MIGRATE_SKIP,
        semantic: "谱系簿建簿+挂起迁移总开关（#679 D1b，反证臂：回到 D1a 前旧行为）",
        kind: GateKind::Behavior,
        default_arm: "未设/非\"1\" ⟹ 启用（建簿默认真，与 OPSEM_DUMP_DIR 落盘解耦）",
        layer: "lineage",
    },
    EnvKeyMeta {
        key: THETA_REBASE_MIGRATE_STRICT,
        semantic: "谱系簿读法切换：严格读法（追下级 lineage）vs 宽读法（只比原始种子 ordinal）",
        kind: GateKind::Behavior,
        default_arm: "未设 ⟹ 宽读法（#679 用户裁定后生产默认）；=1 ⟹ 严格读法（对照留档）",
        layer: "lineage",
    },
    EnvKeyMeta {
        key: THETA_LINEAGE_TRACE,
        semantic: "逐次重基迁移/拒迁明细打到 stderr（纯诊断，不进产物文件，不参与任何判据）",
        kind: GateKind::Observation,
        default_arm: "未设/非\"1\" ⟹ 不打印",
        layer: "lineage",
    },
    EnvKeyMeta {
        key: THETA_T3INC_SKIP,
        semantic: "T3-in-c 合取跳过——关闭后一类点回退 #606 前旧行为（仍置一类 bit）",
        kind: GateKind::Behavior,
        default_arm: "未设 ⟹ 启用 T3-in-c 合取（现行行为）；=1 ⟹ 回退旧行为",
        layer: "classifier",
    },
    EnvKeyMeta {
        key: THETA_CASCADE_FULLCLEAR,
        semantic: "cascade 全清对照（test-only A/B）：强制整塔前缀清空替代增量保留前缀路径",
        kind: GateKind::Behavior,
        default_arm: "未设 ⟹ off（走增量路径）；=1 ⟹ 全清对照（须 bit_exact_per_bar 仍绿）",
        layer: "classifier",
    },
    EnvKeyMeta {
        key: CENSUS_WINDOW,
        semantic: "census 测试可选日期窗（\"起,止\"），检验 type1 对水平线依赖性（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 全历史",
        layer: "classifier",
    },
    EnvKeyMeta {
        key: DIAG_CANDCACHE,
        semantic: "缓存序列版 vs 全量重算版 Cand^δ 事件一致性断言（生产路径可达的诊断门）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 不对拍断言（直通）",
        layer: "classifier",
    },
    EnvKeyMeta {
        key: DIAG_L0UNITS,
        semantic: "l0_units_cache 复用版 vs 全量 segment_to_unit 一致性对拍（生产路径可达）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 不对拍（零开销直通）",
        layer: "classifier",
    },
    EnvKeyMeta {
        key: OPSEM_DUMP_DIR,
        semantic: "opsem/rebase_txn 构造证书落盘目录——classifier(rebase_txn::sink) 与 \
                   backtest(opsem_dump::at_dir) 共享同一目录键（跨层，均只影响落盘，不改判据）",
        kind: GateKind::Observation,
        default_arm: "未设/空 ⟹ 不落盘",
        layer: "cross",
    },
    EnvKeyMeta {
        key: THETA_CASCADE_EPROBE,
        semantic: "cascade 放行条件3 falsification 探针（保留前缀比例打印+门控断言，test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 不启用（探针关，不打印）",
        layer: "cross",
    },
    EnvKeyMeta {
        key: THETA_PROFILE_STAGES,
        semantic: "阶段计时插桩启用（classifier::stage_profile 为规范存取点，backtest 测试侧另有\
                   独立直读，均只影响计时/打印）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 不计时（零开销直通）",
        layer: "cross",
    },
    EnvKeyMeta {
        key: VOICE_EXEC,
        semantic: "W1 gate：声部独立执行臂 vs 净额臂（churn 修复）",
        kind: GateKind::Behavior,
        default_arm: "未设/非\"1\" ⟹ 净额臂（bit-exact 回归锁）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: THETA_NEST_CERT_GATE,
        semantic: "π 开仓准入区间套证书门——T3 起严格链判定为唯一 nest 判定源",
        kind: GateKind::Behavior,
        default_arm: "未设/非\"1\" ⟹ π 门整体跳过（全路径 bit-exact 不变）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: THETA_ENTRY_STOP_RECHECK_SKIP,
        semantic: "entry-stop 复核跳过（反证臂，仅供审计复跑，锚重订常例）",
        kind: GateKind::Behavior,
        default_arm: "未设/非\"1\" ⟹ 复核开（生产臂）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: THETA_HISTBIND_REROUTE_SKIP,
        semantic: "histbind 重路由跳过（反证臂，仅供审计复跑，锚重订常例）",
        kind: GateKind::Behavior,
        default_arm: "未设/非\"1\" ⟹ 重路由开（生产臂）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: THETA_CENTER_OSCILLATION,
        semantic: "中枢震荡短差门激活（#357 验收③，config.center_oscillation.enabled）",
        kind: GateKind::Behavior,
        default_arm: "未设/非\"1\" ⟹ false（m8_e2e 默认关轨迹逐字节不变）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: KAPPA_BARRIER_NUM,
        semantic: "κ 障碍分子（env > config > baseline 优先序，A10 附则A 接口冻结）",
        kind: GateKind::Behavior,
        default_arm: "未设/不可解析 ⟹ 落 config.risk_policy；config=None ⟹ baseline κ=0",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: KAPPA_BARRIER_DEN,
        semantic: "κ 障碍分母（配对 KAPPA_BARRIER_NUM，非正分母 panic 快失败）",
        kind: GateKind::Behavior,
        default_arm: "同 KAPPA_BARRIER_NUM",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: ENFORCE_GROSS_CAP,
        semantic: "毛敞口上限强制生效开关（残差/π^full 跑批测毛 cap 效应）",
        kind: GateKind::Behavior,
        default_arm: "未设/非\"true\" ⟹ 不激活",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: THETA_DIR_PRESET,
        semantic: "方向预设 Follow/Adversary/Neutral（g3 三套 OOS 入口，η 冻结 #135）",
        kind: GateKind::Behavior,
        default_arm: "未设 ⟹ Neutral",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: M7_WITNESS_A10,
        semantic: "A10 成本模型注入（margin=CME-simple+三常费率，与 m8_e2e 同函数同源，test-only）",
        kind: GateKind::Behavior,
        default_arm: "未设/非\"1\" ⟹ 零成本旧路径 bit-exact 不动",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: THETA_OTHERWISE_DOMAIN_SIDECAR,
        semantic: "一类点 T3-in-c 分级 sidecar 全窗汇总（只读旁路，不参与订单/候选/风控/账本）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 关",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: THETA_STRICT_NEST_SIDECAR,
        semantic: "严格 nest 证书 sidecar 汇总（只读旁路，生产 π 重放同帧旁路产出）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 关",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: OPSEM_GAMMA_DUMP_DIR,
        semantic: "γ dump 落盘目录",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 不落盘",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: T5A_CHAIN_DUMP_DIR,
        semantic: "T5a 分窗链 dump 目录（每窗一文件 <dir>/t5a_chain_dump_<tag>.jsonl）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ no-op",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: T5A_CHAIN_DUMP_PATH,
        semantic: "T5a 链 dump 单文件路径（惰性打开，每线程一次）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 不打开（保持无写入器）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: M8_WIN_FILTER,
        semantic: "m8 批跑窗口选择/dump 窗口标签——只跳过其他窗，窗内行为逐字节不变",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 全窗（wverify_run 侧 fallback tag=\"wf8\"）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: ENTRY_STOP_REVERSE_DUMP,
        semantic: "entry-stop 反向逐行 dump 文件路径",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 不写",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: DELTAFREE_DUMP,
        semantic: "δ-free 逐信号台账 dump 路径（输出产物，在线主裁决不消费本文件）",
        kind: GateKind::Observation,
        default_arm: "写侧 dump_deltafree_pertrade（wverify_run.rs:270）未设/空 ⟹\
                       /tmp/wv_full_zdecision.tsv；读侧 deltafree_exact_recompute\
                       （wverify_run.rs:2950，离线复算入口）未设 ⟹\
                       /tmp/finalpha/deltafree_pertrade.tsv（读侧 fallback，非写侧真值）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: THETA_V0_SHADOW_DIVERGENCE_PATH,
        semantic: "shadow 分歧记录文件路径（var_os，双 fill 语义对拍侧信道）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ None（不记）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: M8_FEE_DATUM,
        semantic: "m8 费率 datum spec 注入（未设 ⟹ 臂R 逐位不变；设 ⟹ 升臂D 标定档，#758 issue766 \
                   随 m8.rs 诊断件恢复一并补登）",
        kind: GateKind::Behavior,
        default_arm: "未设 ⟹ no-op（臂R bit-exact 中性）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: M8_LEVEL_CAP,
        semantic: "m8 级别帽臂开关，注入 #310 既有 level_weights（#758 issue766 随 m8.rs 诊断件恢复一并补登）",
        kind: GateKind::Behavior,
        default_arm: "未设 ⟹ 不开帽",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: M8_SYMBOL,
        semantic: "m8 报表品种路由（BTC/OKLO，未知 symbol fail-loud；#758 issue766 随 m8.rs 诊断件恢复一并补登）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ BTC",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: M8_REPORT_PATH,
        semantic: "m8 端到端四层报告落盘路径（#758 issue766 随 m8.rs 诊断件恢复一并补登）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ /tmp/m8_e2e_all_systems_oos.md",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: ECON_LEDGER_CSV,
        semantic: "逐信号台账 CSV 导出路径（#666，15 列含 sigma_higher，test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 固定 /tmp 路径",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: ECON_C3_OVERLAP_PROBE,
        semantic: "C3 L1 零命中根因判别只读旁路探针——不写 Classification 正常输出（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设/非\"1\" ⟹ 关",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: ECON_L2_MAX_BARS,
        semantic: "L2 econ 分析截断窗（O(n²) 全量 461 万 bar 太重，test-only）",
        kind: GateKind::Observation,
        default_arm: "两臂并存（票 #746 复核）：多数调用点未设 ⟹ 300_000（MAX_BARS/\
                       MAX_BARS_DEFAULT 硬编码 fallback）；l2_depth_distribution_dx\
                       （econ_positive.rs:5253）与 l2_btc_earning_shares_unreachable_hwm_debearing\
                       （runner_tests.rs:6031）两处未设 ⟹ usize::MAX（不截断，跑全历史）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: ECON_L2_TRAIN_FRAC,
        semantic: "train(前 frac)/holdout(后 1-frac) 时间切分比例（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 调用点硬编码 fallback 比例",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: ECON_WF_WINDOWS,
        semantic: "walk-forward 滚动窗口数（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 调用点硬编码 fallback",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: PERM_XPROC_SEED,
        semantic: "跨进程置换检验 worker 种子——仅置位时该 worker 测试运行",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ no-op（普通 cargo test 下不跑）",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: S2_BARS,
        semantic: "S2 段级 GN 统计测试窗口截断（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 全量",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: S3_BARS,
        semantic: "S3 capture-OOS 测试窗口截断（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 全量",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: A0_PROFILE_BARS,
        semantic: "A0 克隆簇占比 profile 测试窗口（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 调用点硬编码 fallback，取可用量上限",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: A3_PROFILE_BARS,
        semantic: "A3 stage 计时测试窗口（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 调用点硬编码 fallback，取可用量上限",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: BITEXACT_BARS,
        semantic: "bit-exact 逐 bar 双跑（增量+legacy 对照）测试窗口截断（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 调用点硬编码 fallback",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: M7_WITNESS_BARS,
        semantic: "M7 witness 测试窗口截断（461 万 bar 全量前缀重分类极重，test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 全量",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: NEST_GATE_SMOKE_BARS,
        semantic: "nest-gate smoke 测试窗口截断（test-only）",
        kind: GateKind::Observation,
        default_arm: "两臂并存（票 #746 复核）：nest_gate_diag_terminal_events\
                       （runner_tests.rs:4865）未设 ⟹ 50_000；nest_gate_smoke_stats\
                       （runner_tests.rs:5168）未设 ⟹ 20_000",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: NEST_GATE_SMOKE_START,
        semantic: "nest-gate smoke 测试窗口起点（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 尾部 n_full-max_bars",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: T1_PROBE_BARS,
        semantic: "T1 探针测试窗口截断（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 50_000",
        layer: "backtest",
    },
    EnvKeyMeta {
        key: T1_PROBE_START,
        semantic: "T1 探针测试窗口起点（test-only）",
        kind: GateKind::Observation,
        default_arm: "未设 ⟹ 尾部 n_full-max_bars",
        layer: "backtest",
    },
];

/// 全键只读枚举（运行期可列，spec #756 C2 形状条「查询口」半条）。
pub fn all() -> impl Iterator<Item = &'static EnvKeyMeta> {
    REGISTRY.iter()
}

/// 按行为门/观测门过滤（审计「当前有哪些行为开关」的直接工具）。
pub fn by_kind(kind: GateKind) -> impl Iterator<Item = &'static EnvKeyMeta> {
    REGISTRY.iter().filter(move |m| m.kind == kind)
}

/// 按所属层过滤（层次纪律核查用）。
pub fn by_layer(layer: &'static str) -> impl Iterator<Item = &'static EnvKeyMeta> {
    REGISTRY.iter().filter(move |m| m.layer == layer)
}

/// 按键名字面量查询登记条目。
pub fn find(key: &str) -> Option<&'static EnvKeyMeta> {
    REGISTRY.iter().find(|m| m.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 表内键数锁 50（票 #746 面复核实数 46 + #758 issue766 随 m8.rs/report.rs 诊断件恢复
    /// 补登 M8_FEE_DATUM/M8_LEVEL_CAP/M8_SYMBOL/M8_REPORT_PATH 四键）。
    #[test]
    fn registry_has_46_entries() {
        assert_eq!(
            REGISTRY.len(),
            50,
            "注册表键数漂移——新增/删除 env 键须同步登记表（票 #746 复核实数 46 + #758 issue766 补 4）"
        );
    }

    #[test]
    fn registry_keys_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for m in REGISTRY.iter() {
            assert!(seen.insert(m.key), "注册表重复键：{}", m.key);
        }
    }

    /// const 集合↔REGISTRY 数组对拍（票 #746 关票条件 a）：本文件 `pub const FOO: &str = "FOO";`
    /// 形式声明的键名字面量单源持有点，与 REGISTRY 数组逐一对拍——新增 const 忘记登记 REGISTRY
    /// 照样能编译通过，唯有本测试能拦（`registry_has_46_entries` 只锁数组长度，锁不住这个）。
    #[test]
    fn const_declarations_match_registry() {
        let src = include_str!("env_registry.rs");
        let declared: Vec<&str> = src
            .lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("pub const ")?;
                let (_name, tail) = rest.split_once(": &str = \"")?;
                let (value, _) = tail.split_once("\";")?;
                Some(value)
            })
            .collect();
        assert!(
            !declared.is_empty(),
            "自解析失败——未从本文件抓到任何 pub const 声明，parser 已随格式漂移"
        );
        for key in &declared {
            assert!(
                REGISTRY.iter().any(|m| m.key == *key),
                "const 声明的键 {key} 未登记进 REGISTRY——加 const 必须同步登记五列"
            );
        }
        assert_eq!(
            declared.len(),
            REGISTRY.len(),
            "const 声明数（{}）与 REGISTRY 键数（{}）不一致——存在声明但未登记，或登记但无 const 声明的键",
            declared.len(),
            REGISTRY.len()
        );
    }

    /// 独立重跑清点命令（票 #746 grep 口径，关票条件 b 扩正则）：迁移后全仓（本文件除外）
    /// 不应再有 `env::var`/`env::var_os`/`env::set_var`/`env::remove_var` 裸字符串字面量直读，
    /// 也不应再有传给 `env_flag(...)` 这类 helper 的裸字面量（正是 `THETA_LINEAGE_TRACE` 首轮
    /// grep 漏检的模式——原正则只锁 `env::var(...)`，helper 间接调用漏检）——证据是「真收口」
    /// 而非「多一份影子表」。
    #[test]
    fn no_stray_env_literals_outside_registry() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR 未设");
        let theta_v0_dir = std::path::Path::new(&manifest_dir).join("src/theta_v0");
        let output = std::process::Command::new("grep")
            .args([
                "-rlE",
                r#"(env::(var|var_os|set_var|remove_var)|env_flag)\("[A-Z_0-9]+""#,
            ])
            .arg(&theta_v0_dir)
            .output()
            .expect("grep 不可执行（完备性单测依赖系统 grep）");
        let stray: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|p| !p.ends_with("env_registry.rs"))
            .map(|s| s.to_string())
            .collect();
        assert!(
            stray.is_empty(),
            "以下文件仍裸读 env 名字面量，应改引用 env_registry 常量：{stray:?}"
        );
    }
}
