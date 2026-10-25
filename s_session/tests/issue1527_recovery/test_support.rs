//! #1527 TestOnly：既存测试库发布明确的小域锚，再走真实 loader；无生产默认入口。
use super::*;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

pub(super) fn prepare(db: &Path) -> PreparedAdmission {
    let db = std::fs::canonicalize(db).unwrap();
    let owner_path = {
        let mut p = db.as_os_str().to_os_string();
        p.push(".control-owner.json");
        PathBuf::from(p)
    };
    if owner_path.exists() {
        let owner: Value = serde_json::from_slice(&std::fs::read(&owner_path).unwrap()).unwrap();
        return load_for_db(
            &db,
            Path::new(owner["admission"]["path"].as_str().unwrap()),
            ControlEnv::from_values(None, None),
        )
        .unwrap();
    }
    let state = db.parent().unwrap().join(format!(
        "i1527-state-{}",
        db.file_name().unwrap().to_string_lossy()
    ));
    std::fs::create_dir(&state).unwrap();
    let published = state.join(PUBLISHED);
    std::fs::create_dir(&published).unwrap();
    std::fs::set_permissions(&published, std::fs::Permissions::from_mode(0o700)).unwrap();
    let limits = published.join(LIMITS_NAME);
    let binding_path = published.join(BINDING_NAME);
    let raw=serde_json::to_vec(&json!({
        "max_total_wire_bytes":"1073741824", "max_total_node_bytes":"1073741824", "max_total_nodes":"10000000",
        "max_total_records":"10000000", "max_total_requests":"1000000", "max_elapsed_ms":"600000",
        "max_staged_bytes":"67108864", "max_store_bytes":"1073741824"
    })).unwrap();
    for (p, bytes) in [(&limits, raw.as_slice()), (&binding_path, &b""[..])] {
        std::fs::write(p, bytes).unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn identity(p: &Path) -> Value {
        let m = std::fs::metadata(p).unwrap();
        json!({"device":m.dev(),"inode":m.ino(),"uid":m.uid(),"mode":m.mode()&0o7777})
    }
    let binding = json!({"db":db,"path":limits,"sha256":sha256_hex(&raw),"file_identity":identity(&limits),
        "binding_identity":identity(&binding_path),"directory_identity":identity(&published)});
    std::fs::write(&binding_path, serde_json::to_vec(&binding).unwrap()).unwrap();
    std::fs::write(
        &owner_path,
        serde_json::to_vec(&json!({"db":db,"state_dir":state,"admission":binding})).unwrap(),
    )
    .unwrap();
    std::fs::set_permissions(&owner_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    load_for_db(&db, &limits, ControlEnv::from_values(None, None)).unwrap()
}
