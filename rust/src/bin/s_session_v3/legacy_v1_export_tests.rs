// #1405: test-only export adapter on the exact b46 producer. Never relabel /2.
// Kept on the evidence branch; the production algorithm and old locks are unchanged.
use super::*;
use std::path::{Path, PathBuf};

const BASE: &str = "b46ba296993371cb56b2dff7ef878647dc0665ad";
const RULE: &str = "s2-axis-quantifiers+new-bi/1";

fn output(part: &str) -> PathBuf {
    let root =
        PathBuf::from(std::env::var_os("TB1405_LEGACY_OUTPUT").expect("explicit export path"));
    assert!(root.is_absolute());
    let path = root.join(part);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn write_json(path: impl AsRef<Path>, value: &Value) {
    std::fs::write(path, canonical_json(value)).unwrap();
}

fn counts_json(counts: &Counts) -> Value {
    json!({"objects": counts.objects, "groups": counts.groups,
        "windows": counts.windows, "effective": counts.effective,
        "insufficient": counts.insufficient, "outside_domain": counts.outside_domain})
}

#[test]
fn tb1405_export_legacy_v1_source_nodes() {
    assert_eq!(crate::V3_RULE_REVISION, RULE);
    let out = output("source-audit");
    let (mut nodes, state, raw, rule) = fixture();
    assert_eq!(rule, RULE);
    assert!(!nodes.is_empty() && !raw.is_empty());
    let mut bodies = Vec::new();
    let mut index = Vec::new();
    for (digest, tag, body) in nodes.iter() {
        assert_eq!(sc::frame(tag, body), *digest);
        index.push(json!({"root":sc::root_string(digest), "tag":tag,
            "offset":bodies.len().to_string(), "length":body.len().to_string(),
            "body_sha256":crate::sha256_hex(body)}));
        bodies.extend_from_slice(body);
    }
    // Raw body bytes are authoritative; JSON is only an index, never a re-encoder.
    std::fs::write(out.join("node-bodies.bin"), &bodies).unwrap();
    write_json(out.join("nodes.json"), &json!(index));
    write_json(out.join("state-ref.json"), &state.to_transport_json());
    write_json(out.join("raw.json"), &json!(raw));
    let mut rendered = Vec::new();
    sc::render_canonical(&mut nodes, &state, &mut rendered).unwrap();
    std::fs::write(out.join("state.canonical.json"), &rendered).unwrap();
    let rendered: Value = serde_json::from_slice(&rendered).unwrap();
    assert_eq!(rendered["rule_revision"], RULE);
    assert_eq!(rendered["structure_cut"], "cut-1");

    // Reimport only the frozen original bytes; do not invoke GenerationRuntime again.
    let mut imported = MemoryNodes::new();
    let disk_bodies = std::fs::read(out.join("node-bodies.bin")).unwrap();
    let disk_index: Vec<Value> =
        serde_json::from_slice(&std::fs::read(out.join("nodes.json")).unwrap()).unwrap();
    for item in disk_index {
        let offset = item["offset"].as_str().unwrap().parse::<usize>().unwrap();
        let len = item["length"].as_str().unwrap().parse::<usize>().unwrap();
        let body = &disk_bodies[offset..offset + len];
        assert_eq!(crate::sha256_hex(body), item["body_sha256"]);
        imported
            .put(
                sc::parse_root(item["root"].as_str().unwrap()).unwrap(),
                item["tag"].as_u64().unwrap().try_into().unwrap(),
                body,
            )
            .unwrap();
    }
    assert_eq!(imported.len(), nodes.len());
    let imported_state: Value =
        serde_json::from_slice(&std::fs::read(out.join("state-ref.json")).unwrap()).unwrap();
    let imported_state = ContentRef::from_transport_json(&imported_state).unwrap();
    let imported_raw: Vec<Value> =
        serde_json::from_slice(&std::fs::read(out.join("raw.json")).unwrap()).unwrap();
    let none = audit_generated(
        &mut imported,
        &imported_state,
        &imported_raw,
        tb02a::PROFILE,
        RULE,
    )
    .unwrap();
    let meter = Meter::default();
    let some = audit_generated_metered(
        &mut imported,
        &imported_state,
        &imported_raw,
        tb02a::PROFILE,
        RULE,
        None,
        Some(&meter),
    )
    .unwrap();
    assert_eq!(none, some);
    assert!(none.objects > 0 && none.groups > 0 && none.windows > 0);
    // Written last: missing completion manifest means an incomplete export, even if bytes exist.
    write_json(
        out.join("COMPLETE.json"),
        &json!({
            "format":"tb1405-legacy-source-originals/1", "producer_base":BASE,
            "rule_revision":rule, "profile_id":tb02a::PROFILE,
            "fixture":"v3::source_audit::tests::fixture", "cut":rendered["structure_cut"],
            "generation":rendered["generation"], "input_frontier":rendered["input_frontier"],
            "node_count":nodes.len(), "raw_count":raw.len(), "counts":counts_json(&none),
            "node_bodies_sha256":crate::sha256_hex(&bodies),
            "old_reader_none_some_import_pass":true,
            "formal_product_run":false, "current_reader_acceptance":false
        }),
    );
}

#[test]
fn tb1405_export_legacy_v1_persistent_database() {
    assert_eq!(crate::V3_RULE_REVISION, RULE);
    let out = output("persistent");
    let mut fixture = crate::v3::prepared::tests::Fixture::new();
    let mut inputs = Vec::new();
    for (n, row, revision) in [
        (1, ("a", 0, 6, 12, 3, 7), 1),
        (2, ("b", 10, 8, 16, 6, 12), 1),
        (3, ("c", 20, 8, 13, 4, 7), 1),
        (4, ("b", 10, 8, 17, 6, 12), 2),
    ] {
        let (envelope, token) = fixture.accept_begin(n, row, revision);
        let receipt = fixture.publish(&envelope, &token).unwrap();
        assert_eq!(receipt.generation, n);
        assert_eq!(receipt.message_receipt["kind"], "Committed");
        inputs.push(json!({"envelope":envelope, "token":token,
            "receipt":receipt.message_receipt}));
    }
    let raw = read_raw_events(&fixture.conn).unwrap();
    let rule = meta_get_opt(&fixture.conn, "rule_revision")
        .unwrap()
        .unwrap();
    assert_eq!(rule, RULE);
    let cut = meta_get_opt(&fixture.conn, "structure_cut")
        .unwrap()
        .unwrap();
    let audit = {
        let tx = fixture.conn.transaction().unwrap();
        let audited = crate::v3::state::audit(&tx).unwrap();
        assert_eq!(audited.raw.len(), 4);
        assert_eq!(audited.cuts.len(), 4);
        assert!(!audited.objects.is_empty() && !audited.auxiliary.witnesses.is_empty());
        json!({"raw":audited.raw.len(), "cuts":audited.cuts.len(),
            "objects":audited.objects.len(), "witnesses":audited.auxiliary.witnesses.len()})
    };
    let checkpoint: (i64, i64, i64) = fixture
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!(checkpoint, (0, 0, 0), "copy requires a complete checkpoint");
    let db = fixture.db_path();
    let original = std::fs::read(&db).unwrap();
    std::fs::copy(&db, out.join("s.sqlite")).unwrap();
    assert_eq!(std::fs::read(out.join("s.sqlite")).unwrap(), original);
    std::fs::copy(
        db.parent().unwrap().join("clock.json"),
        out.join("clock.json"),
    )
    .unwrap();
    write_json(out.join("raw.json"), &json!(raw));
    write_json(out.join("commits.json"), &json!(inputs));
    write_json(
        out.join("COMPLETE.json"),
        &json!({
            "format":"tb1405-legacy-persistent-originals/1", "producer_base":BASE,
            "rule_revision":rule, "cut":cut, "generation":4,
            "fixture":"v3::prepared::tests::Fixture", "audit":audit,
            "database_sha256":crate::sha256_hex(&original), "checkpoint":checkpoint,
            "database_is_exact_checkpointed_copy":true,
            "formal_product_run":false, "current_reader_acceptance":false
        }),
    );
}
