//! #648 T1 主题件：rebase_txn（自 classifier/mod.rs 内联 `mod tests` 纯移动抽离，零行为变更）。

use super::super::*;
use super::fixtures::*;

// #1392：构造证书的测试需要实际产出笔/段；显式细化合成价格，消除同价身份未定。
// 原严格价序保留、原同价被细化；不改生产判据，不用于真实成交数据。
fn seam_bars() -> Vec<super::super::super::types::Bar> {
    let vals: Vec<i64> = (0..600)
        .map(|i| {
            let f = i as f64;
            let price = 1000
                + (40.0 * (f * 0.35).sin() + 15.0 * (f * 0.11).cos() + 6.0 * (f * 1.7).sin())
                    as i64;
            price * 601 + i as i64
        })
        .collect();
    bars_from_closes(&vals)
}

/// ★#543 D1a：构造证书 seam 的**生产放置点**端到端见证（不是纯函数单测——真走
/// `classify_incremental` 逐 bar 增量路径）。
///
/// 覆盖三个放置点：
/// 1. 常态 frontier pop（`had_emitted_window` 分支，产出点①）+ 新 tail 返回/自然 emit（②③）；
/// 2. 一窗产 k 个子对象（`WinMeta.emitted`，产出点⑤——若本输入触发九段升级则 emitted>1）；
/// 3. cascade 后缀失效（产出点④）——用 **input len shrink**（同 cache 喂更短前缀）强制
///    `units.len() < lc.last_input_len` ⟹ `e=0` ⟹ `P=0` 全清分支，旧后缀必须在 clear 前被捕获。
///
/// 断言只落在「证书结构自洽 + 放置点确实被触达」上；不断言具体 relation 分布（那取决于合成
/// 输入的几何，属现场事实，由 wf8 实测报告登记）。
#[test]
fn rebase_txn_seam_emits_certificates_at_production_placement_points() {
    let cfg = super::super::super::config::ThetaConfig::default();
    let bars = seam_bars();

    rebase_txn::test_capture_start();
    let mut cache = TowerCache::new();
    for i in 5..=bars.len() {
        let l0 = super::super::super::parser::parse_layer(&bars[..i], &cfg);
        let _ = classify_incremental(&l0, &cfg, &mut cache, &[]);
    }
    // len shrink：同一 cache 喂更短前缀 ⟹ cascade P=0 全清（产出点④）。
    let l0_short = super::super::super::parser::parse_layer(&bars[..300], &cfg);
    let _ = classify_incremental(&l0_short, &cfg, &mut cache, &[]);
    let lines = rebase_txn::test_capture_take();

    assert!(
        !lines.is_empty(),
        "生产放置点一条构造证书都没产出（seam 未接通）"
    );
    let mut n_pop = 0usize;
    let mut n_cascade = 0usize;
    let mut n_continued = 0usize;
    let mut n_multi_emit = 0usize;
    for line in &lines {
        for k in [
            "\"schema\":\"rebase_transform_txn_v1\"",
            "\"txn_id\"",
            "\"bar\"",
            "\"level\"",
            "\"cause\"",
            "\"dirty_e\"",
            "\"resume_start\"",
            "\"prefix_count\"",
            "\"old_nodes\"",
            "\"new_nodes\"",
            "\"transform_edges\"",
            "\"lower_txn_id\"",
            "\"lower_edge_refs\"",
            "\"algorithm_version\"",
            "\"source_order_digest\"",
            "\"txn_digest\"",
        ] {
            assert!(line.contains(k), "证书缺字段组 {k}");
        }
        assert!(
            !line.contains("18446744073709551615"),
            "usize::MAX 哨兵须记 null"
        );
        if line.contains("\"cause\":\"frontier_pop\"") {
            n_pop += 1;
        }
        if line.contains("\"cause\":\"cascade_p0\"")
            || line.contains("\"cause\":\"cascade_prefix\"")
        {
            n_cascade += 1;
            assert!(
                line.contains("\"side\":\"old\""),
                "cascade 事务必须带被丢弃的旧后缀快照（产出点④的全部意义）"
            );
        }
        if line.contains("\"relation\":\"continued_1to1\"") {
            n_continued += 1;
        }
        if line.contains("\"emitted\":2") || line.contains("\"emitted\":3") {
            n_multi_emit += 1;
        }
    }
    eprintln!(
        "[#543 seam] txn={} frontier_pop={n_pop} cascade={n_cascade} \
             含 continued_1to1={n_continued} 一窗多产={n_multi_emit}",
        lines.len()
    );
    assert!(n_pop > 0, "常态 frontier pop 放置点未触达");
    assert!(
        n_cascade > 0,
        "cascade 后缀失效放置点未触达（len shrink 未走到 P=0 分支）"
    );
    assert!(
        n_continued > 0,
        "无一条连续边——同 seed 重扫本应产 continued_1to1"
    );
}

/// ★#543 D1a 负控：seam 未启用（无 env、无捕获）⟹ 逐 bar 增量塔的输出与启用时**逐字段相同**。
/// 这是「行为零变化」红线的单测化（wf8 关臂 cmp=0 是同一命题的现场版）。
#[test]
fn rebase_txn_seam_does_not_change_classification_output() {
    let cfg = super::super::super::config::ThetaConfig::default();
    let bars = seam_bars();

    let run = |capture: bool| {
        // ★#679 D1b：seam 在生产判径默认常开，故关臂须显式按下反证开关，否则本负控
        // 两臂都是「开」，失去区分力。
        crate::theta_v0::lineage_book::test_set_consumer(Some(capture));
        if capture {
            rebase_txn::test_capture_start();
        }
        let mut cache = TowerCache::new();
        let mut out = Vec::new();
        for i in 5..=bars.len() {
            let l0 = super::super::super::parser::parse_layer(&bars[..i], &cfg);
            let __co1 = classify_incremental(&l0, &cfg, &mut cache, &[]);
            let c = __co1.classification;
            let tower = __co1.tower;
            out.push((c, tower));
        }
        let n = if capture {
            rebase_txn::test_capture_take().len()
        } else {
            0
        };
        crate::theta_v0::lineage_book::test_set_consumer(None);
        (out, n)
    };
    let (off, _) = run(false);
    let (on, emitted) = run(true);
    assert!(emitted > 0, "开臂须真产出证书，否则本负控无区分力");
    assert_eq!(off.len(), on.len());
    for (i, (a, b)) in off.iter().zip(on.iter()).enumerate() {
        assert_eq!(
            a.0, b.0,
            "bar {i}: Classification 被观测旁路改变（行为零变化红线破）"
        );
        assert_eq!(
            a.1, b.1,
            "bar {i}: tower 快照被观测旁路改变（行为零变化红线破）"
        );
    }
}
