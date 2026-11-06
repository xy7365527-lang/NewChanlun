#!/usr/bin/env python3
"""从冻结 4df 导出旧规则输入及七代历史原件。仅供 GitHub CI 临时工作树。"""
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
base = root / "rust/src/bin/s_session_v3"
path = base / "source_audit.rs"
text = path.read_text()
anchor = "    pub(super) fn fixture()"
export = r'''    #[test]
    fn export33z_source_fixture() {
        let (store, root, raw, rule) = fixture();
        let nodes: Vec<_> = store.iter().map(|(d, t, b)| (*d, t, b.to_vec())).collect();
        let target = std::path::PathBuf::from(std::env::var("LEGACY_EXPORT_DIR").unwrap());
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("source.json"), serde_json::to_vec(&serde_json::json!({
            "head": "4df2d10f92d1075f5fb761a1b112c9c949da5408",
            "root": root.to_transport_json(), "raw": raw, "rule": rule, "nodes": nodes
        })).unwrap()).unwrap();
    }

'''
assert text.count(anchor) == 1
path.write_text(text.replace(anchor, export + anchor))
path = base / "driver.rs"
text = path.read_text()
start = text.index("    fn ordinary_proof_multicut_complete_outputs_match_frozen_oracle()")
end = text.index("\n    #[cfg(", start)
fn = text[start:end].replace("ordinary_proof_multicut_complete_outputs_match_frozen_oracle", "export33z_legacy_multicut")
anchor = '            assert_eq!(new_roots, old_roots, "{}", step.kind);'
addition = r'''
            let target = std::path::PathBuf::from(std::env::var("LEGACY_EXPORT_DIR").unwrap());
            std::fs::create_dir_all(&target).unwrap();
            let db = target.join(format!("cut-{}.db", index + 1));
            actual.conn.execute("VACUUM INTO ?1", [db.to_str().unwrap()]).unwrap();
            std::fs::write(target.join(format!("cut-{}.json", index + 1)),
                serde_json::to_vec(&serde_json::json!({
                    "head": "4df2d10f92d1075f5fb761a1b112c9c949da5408",
                    "reply": reply,
                    "roots_debug": format!("{:?}", prepared::tests::runtime_root_tables(&actual.conn, false)),
                    "arrays": new_roots
                })).unwrap()).unwrap();
'''
assert fn.count(anchor) == 1
fn = fn.replace(anchor, anchor + addition)
path.write_text(text[:start] + fn + text[end:])
