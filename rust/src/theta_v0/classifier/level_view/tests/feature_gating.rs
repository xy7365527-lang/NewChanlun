use super::*;
use super::fixtures::{extended_windows, query, trend_block};

/// ★R1 负例（037:18 必要合取）：同一夹具但 as_of=129 截断回试腿 ⟹ c 内无三买（T3 构造性
/// 不成立），即便面积/T2 俱备也不得 Completed——锁定「全合取缺三买即未确认」边界。
#[test]
fn auto_pairing_pending_when_third_buy_not_established() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let block = trend_block(Some(Direction::Up));
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();
    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query(C2VersionTuple::auto_pairing()), // as_of=129：回试腿（end=139）被截断
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &[block],
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        },
    )
    .unwrap();
    assert_eq!(view.pairs.len(), 1);
    assert!(
        matches!(
            view.moves[0].completion,
            CompletionStatus::Pending {
                reason: PendingReason::TerminalLegNotDivergent,
                ..
            }
        ),
        "c 内无三买（037:18）⟹ 全合取不成立 ⟹ 不得 TerminalDivergence"
    );
}

#[test]
fn feature_default_false_and_cache_key_stable() {
    assert!(!C2LevelViewConfig::default().enabled);
    let q = query(C2VersionTuple::auto_pairing());
    let empty_windows = Vec::new();
    let disabled = assemble_level_view(
        C2LevelViewConfig::default(),
        q,
        LevelViewMaterial {
            projection: ProjectionMaterial::Unprojected(&empty_windows),
            move_blocks: &[],
            lower_legs: &[],
            hist: &[],
            dif: &[],
            close_src: &[],
        },
    );
    assert_eq!(disabled, Err(LevelViewError::FeatureDisabled));

    let a = C2CacheKey::from_query(&q).unwrap();
    let b = C2CacheKey::from_query(&q).unwrap();
    assert_eq!(a, b);
    assert!(a.as_str().contains("dir=central-ggdd-v1"));
    assert!(a.as_str().contains("pair=move-block-ac-v1"));
    assert!(a
        .as_str()
        .contains("projection=extended-to-exact-three-v3-carried-only"));
    let stream_a = C2PersistenceKey::from_query(&q).unwrap();
    let mut later = q;
    later.as_of += 1;
    let stream_b = C2PersistenceKey::from_query(&later).unwrap();
    assert_eq!(
        stream_a, stream_b,
        "freeze event stream key 必须跨 as_of 稳定"
    );
}
