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
        // #1393 W（raw-initialization-w/1，S3 写者唯一规则）：等待期零正式组；首个相邻严格非包含对
        // 出现时从 raw0 按其方向重折（逐例手算，(high,low) 按 source_coord 序）：
        // - 同 OHLC 三根互含，第 4 根 (17,6) 对 (12,3) 严格向上 ⟹ 第 4 步重折为 2 组；
        // - 较早修订：(12,3)→(16,6) 严格向上 2 组；a 改 (13,4) 仍严格向上 2 组；a 改 (17,6) 包含 b ⟹ 回到等待期；
        // - 首来源修订：a=b 互含；c (17,6) 严格向上 ⟹ 2 组；a 改 (9,1) 后 a→b 严格向上 ⟹ 3 组；
        // - 后根包含初始根：b (14,2) 包含 a；c (16,4) 对 b 严格向上 ⟹ UP 重折 {a,b},{c} 2 组；d 再一组 3 组。
        let expected_groups = match (scenario, step) {
            ("乱序新身份且前三根同OHLC" | "顺序新身份且前三根同OHLC", 4) => 2,
            ("较早坐标修订", 2 | 3) => 2,
            ("正常首来源修订使未定后缀重新完成映射", 3) => 2,
            ("正常首来源修订使未定后缀重新完成映射", 4) => 3,
            ("正常后根包含初始根且后继保持未定", 3) => 2,
            ("正常后根包含初始根且后继保持未定", 4) => 3,
            _ => 0,
        };
        assert_eq!(
            live_groups, expected_groups,
            "{scenario} step={step}真实组数"
        );
        // r2 两臂失败现场的生成根；修复只改变审计接收，不改变发布原件。
        // #1394 合入 #1405 后重冻：v3 写者规则升 `s2-axis-quantifiers+new-bi/2`（mod.rs 写入
        // meta.rule_revision），TB-02-A/B 发布内容按 #1343 同价裁定改变；三处生成根为合并后实测值，
        // 与 `813fc75fa5`、`199caedfd0`、`dccb69793c` 三点实测逐值相同（变化来自 #1405 适配合并本身）。
        // 旧值（new-bi/1）：乱序 2342365b…、顺序 2c0906de…、较早修订 408fd573…。
        // #1393×#1419 重冻：写者规则改 W 后继串（S3 唯一写者规则）、W 初始化对象与 C/D/E/F 结构域进入生成态；
        // 三处根为 nc-remote 实测（33a25f746c，组数先经上方逐例手算断言通过）。旧值（new-bi/2）：乱序 bf079f33…、
        // 顺序 91761b3f…、较早修订 c3961d53…。
        let frozen = match (scenario, step) {
            ("乱序新身份且前三根同OHLC", 2) => {
                Some("sc1-df7f1dbcbc367debe6b9ac60266642f470ad3406dc377f5c1500e476438b37d9")
            }
            ("顺序新身份且前三根同OHLC", 2) => {
                Some("sc1-0f2e6a44172e30a311c000db204e35aa0d2eb93f4b14ec95554fab5c9d7dd804")
            }
            ("较早坐标修订", 4) => {
                Some("sc1-2ebf6bd87172b57b300444b6fbc9d31c22557a61cb5d13f1bda2a0213cece0c2")
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

#[test]
fn bounded2_revision_restore_preserves_archived_cuts() {
    fn run(layout: super::super::super::layout::Layout) -> Result<(), String> {
        super::super::super::layout::with_forced_layout_for_test(Some(layout), || {
            let mut fixture = Fixture::with_clock_events(6, 3);
            let rows = crate::tests::tb02b_oracle_rows();
            assert_eq!(rows.len(), 14);
            let (envelope, token) = fixture.accept_begin_rows(1, &rows, 1);
            fixture.publish(&envelope, &token)?;
            let cuts = |f: &mut Fixture| -> Vec<Value> {
                let tx = f.conn.transaction().unwrap();
                let state = state::audit(&tx).unwrap();
                state.cuts.iter().map(|cut| json!({
                    "metadata": format!("{:?}", cut.metadata),
                    "roots": format!("{:?}", cut.roots),
                    "snapshot": cut.snapshot, "delta": cut.delta
                })).collect()
            };
            let original_cuts = cuts(&mut fixture);
            let original_batches = {
                let tx = fixture.conn.transaction().unwrap();
                let state = state::audit(&tx).unwrap();
                state.objects.iter().filter(|o| o.kind == content_producer::CC007_DEVELOPMENT_BATCH_KIND)
                    .map(|o| o.id.clone()).collect::<Vec<_>>()
            };
            assert!(!original_batches.is_empty(), "夹具必须有真实分型及后续单元");
            let row = rows[10];
            let revised = (row.0, row.1, row.2 + 1, row.3 + 1, row.4 + 1, row.5 + 1);
            let (envelope, token) = fixture.accept_begin(2, revised, 2);
            fixture.publish(&envelope, &token)?;
            let before_restore = cuts(&mut fixture);
            assert_eq!(&before_restore[..1], original_cuts.as_slice());
            let withdrawn = {
                let tx = fixture.conn.transaction().unwrap();
                let state = state::audit(&tx).unwrap();
                state.objects.iter().filter(|o| original_batches.contains(&o.id) && o.life.withdrawn_generation.is_some())
                    .map(|o| o.id.clone()).collect::<Vec<_>>()
            };
            assert!(!withdrawn.is_empty(), "修订必须撤回旧批记录");
            let (envelope, token) = fixture.accept_begin(3, row, 3);
            fixture.publish(&envelope, &token)?;
            let after_restore = cuts(&mut fixture);
            assert_eq!(&after_restore[..before_restore.len()], before_restore.as_slice(), "全部归档 cut 的 AsKnown 内容不变");
            let tx = fixture.conn.transaction().unwrap();
            let state = state::audit(&tx).unwrap();
            for id in &withdrawn {
                let object = state.objects.iter().find(|o| &o.id == id).unwrap();
                assert!(object.life.withdrawn_generation.is_some(), "旧身份不得复活");
            }
            let active_batches: Vec<_> = state.objects.iter().filter(|o| o.kind == content_producer::CC007_DEVELOPMENT_BATCH_KIND && o.life.withdrawn_generation.is_none()).collect();
            assert!(active_batches.iter().any(|o| !original_batches.contains(&o.id)), "恢复必须生成新 batch 身份");
            Ok(())
        })
    }
    let failure = run(super::super::super::layout::Layout::Bounded1).unwrap_err();
    assert!(failure.contains("IdentityConflict"), "旧单元机制应复现撤回身份冲突：{failure}");
    run(super::super::super::layout::Layout::Bounded2).unwrap();
}
