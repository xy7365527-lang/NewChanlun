// #1394 r3：r2 原样真实输入的永久回归；不改原 Fixture 或生产 facts。
use super::*;
type Row = (&'static str, i64, i64, i64, i64, i64, i64);

fn audit_summary(audited: &state::AuditedState) -> Value {
    json!({
        "raw":audited.raw,
        "cuts":audited.cuts.iter().map(|cut| json!({
            "metadata":format!("{:?}", cut.metadata),
            "roots":format!("{:?}", cut.roots),
            "snapshot":cut.snapshot,"delta":cut.delta
        })).collect::<Vec<_>>(),
        "objects":format!("{:?}", audited.objects),
        "witnesses":format!("{:?}", audited.auxiliary.witnesses),
        "relations":format!("{:?}", audited.auxiliary.relations),
        "observations":format!("{:?}", audited.auxiliary.observations)
    })
}

fn run_normal(scenario: &'static str, rows: [Row; 4]) {
    let mut fixture = Fixture::new();
    for (offset, (id, coord, open, high, low, close, revision)) in rows.into_iter().enumerate() {
        let step = offset + 1;
        let (envelope, token) =
            fixture.accept_begin(step as i64, (id, coord, open, high, low, close), revision);
        fixture
            .publish(&envelope, &token)
            .unwrap_or_else(|error| panic!("{scenario} step={step} publish: {error}"));
        let tx = fixture.conn.transaction().unwrap();
        let audited = state::audit(&tx)
            .unwrap_or_else(|error| panic!("{scenario} step={step} ordinary: {error}"));
        assert_eq!(audited.cuts.len(), step);
        let live_groups = audited
            .objects
            .iter()
            .filter(|object| {
                object.kind == content_producer::CC005_GROUP_KIND
                    && object.life.withdrawn_generation.is_none()
            })
            .count();
        let expected_groups = match scenario {
            "较早坐标修订" if step == 2 || step == 3 => 2,
            "正常首来源修订使未定后缀重新完成映射" if step == 4 => 3,
            _ => 1,
        };
        assert_eq!(
            live_groups, expected_groups,
            "{scenario} step={step}真实组数"
        );
        // r2 两臂失败现场的生成根；修复只改变审计接收，不改变发布原件。
        let frozen = match (scenario, step) {
            ("乱序新身份且前三根同OHLC", 2) => {
                Some("sc1-2342365b8de7509c2e3d2822fcc492520b26c03099b8cefb25744cf1d0856906")
            }
            ("顺序新身份且前三根同OHLC", 2) => {
                Some("sc1-2c0906dec5026f651a000a2d2dfd317b938d90b36d16dd216ec03238bd16207a")
            }
            ("较早坐标修订", 4) => {
                Some("sc1-408fd57330c482faf31e170f5e5967ce678872191b480711a3a06ab3cd445f0c")
            }
            _ => None,
        };
        if let Some(root) = frozen {
            assert_eq!(
                audited.cuts.last().unwrap().roots.generated_state.root(),
                root
            );
        }
        eprintln!(
            "{scenario} step={step} publish+ordinary OK root={}",
            audited.cuts.last().unwrap().roots.generated_state.root()
        );
    }
    let tx = fixture.conn.transaction().unwrap();
    let ordinary = state::audit(&tx).unwrap();
    #[cfg(feature = "s_capture_native_testonly")]
    {
        let native = state::audit_native_current(&tx).unwrap();
        assert_eq!(audit_summary(&ordinary), audit_summary(&native));
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    {
        let profile = super::super::q_allocation::Profile {
            budget: sc::capture_allocation::OwnedBudget::new(50 * 1024 * 1024),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        };
        let (owned, source) =
            state::audit_bounded_with_allocation(&tx, None, None, &profile).unwrap();
        assert_eq!(audit_summary(&ordinary), audit_summary(&owned));
        assert!(profile.budget.snapshot().live_requested_layout_bytes > 0);
        drop(owned);
        drop(source);
        let meter = profile.budget.snapshot();
        assert_eq!(meter.live_requested_layout_bytes, 0);
        assert_eq!(meter.allocated_layouts, meter.released_layouts);
    }
    // 同一已发布原件再次冷读，验证 replay 结果不依赖上次审计或临时索引。
    let replay = state::audit(&tx).unwrap();
    assert_eq!(audit_summary(&ordinary), audit_summary(&replay));
}

#[test]
fn pending_source_unordered_same_ohlc() {
    run_normal(
        "乱序新身份且前三根同OHLC",
        [
            ("a", 20, 6, 12, 3, 7, 1),
            ("b", 0, 6, 12, 3, 7, 1),
            ("c", 10, 6, 12, 3, 7, 1),
            ("d", 30, 8, 17, 6, 12, 1),
        ],
    );
}

#[test]
fn pending_source_ordered_same_ohlc() {
    run_normal(
        "顺序新身份且前三根同OHLC",
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 6, 12, 3, 7, 1),
            ("c", 20, 6, 12, 3, 7, 1),
            ("d", 30, 8, 17, 6, 12, 1),
        ],
    );
}

#[test]
fn pending_source_earlier_revision() {
    run_normal(
        "较早坐标修订",
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 8, 16, 6, 12, 1),
            ("a", 0, 8, 13, 4, 7, 2),
            ("a", 0, 8, 17, 6, 12, 3),
        ],
    );
}

#[test]
fn pending_source_revision_resolves_initial_suffix() {
    run_normal(
        "正常首来源修订使未定后缀重新完成映射",
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 6, 12, 3, 7, 1),
            ("c", 20, 8, 17, 6, 12, 1),
            ("a", 0, 4, 9, 1, 6, 2),
        ],
    );
}

#[test]
fn pending_source_incoming_encloses_seed() {
    run_normal(
        "正常后根包含初始根且后继保持未定",
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 7, 14, 2, 8, 1),
            ("c", 20, 8, 16, 4, 12, 1),
            ("d", 30, 9, 17, 5, 14, 1),
        ],
    );
}
