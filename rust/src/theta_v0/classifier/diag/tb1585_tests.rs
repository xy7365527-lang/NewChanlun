//! #1585（X-38）：诊断件直读 `l0.segments` 的两个入口在段端门 fail-closed 标记下拒读。
//!
//! 负控：同一 L0 注入标记后，配无标记层的非空分类输出（错配输入）调用 ⟹ 两入口均无产出。
//! 正控：无标记时同一组输入（随机 K 线若干 seed 合计）确有产出，负控不是空转。
use super::super::{classify, Classification};
use super::{cand_delta_tower, cp_recall_upper_bound_audit};
use crate::theta_v0::config::ThetaConfig;
use crate::theta_v0::parser::{parse_layer, tb1581_tests::tb1581_bars};

#[test]
fn tb1585_diag_entries_refuse_parse_invalid_layer() {
    let cfg = ThetaConfig::default();
    let (mut events, mut recall) = (0usize, 0usize);
    for seed in 0..6u64 {
        let layer = parse_layer(&tb1581_bars(0x1581_0000 + seed, 4_000), &cfg);
        assert_eq!(layer.segments_parse_invalid_from, None, "生产口径无门失败");
        let out = classify(&layer, &cfg, &[]);
        assert_ne!(out.classification, Classification::default());
        events += cand_delta_tower(&layer, &out.classification, &out.tower, &cfg)
            .iter()
            .map(Vec::len)
            .sum::<usize>();
        recall += cp_recall_upper_bound_audit(&layer, &out.classification, &out.tower).len();

        let mut marked = layer.clone();
        marked.segments_parse_invalid_from = Some(0);
        assert!(
            cand_delta_tower(&marked, &out.classification, &out.tower, &cfg).is_empty(),
            "seed={seed}：标记层 Cand^δ 驱动器拒读"
        );
        assert!(
            cp_recall_upper_bound_audit(&marked, &out.classification, &out.tower).is_empty(),
            "seed={seed}：标记层召回审计拒读"
        );
    }
    assert!(events > 0, "正控：无标记时 Cand^δ 驱动器有事件");
    assert!(recall > 0, "正控：无标记时召回审计有行");
}
