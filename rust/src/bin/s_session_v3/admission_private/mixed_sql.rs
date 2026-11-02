//! #1527 mixed M1: current expected tables intersect the fixed, reviewed column manifest.
use rusqlite::ffi;
use std::ffi::c_int;

const TABLES: &[(&[u8], &[&[u8]])] = &[
    (b"batches", &[b"batch_id", b"canonical_bytes", b"byte_len"]),
    (
        b"catalog",
        &[
            b"catalog_id",
            b"kind",
            b"title",
            b"domain",
            b"branches_json",
            b"impl_status",
            b"proof_status",
            b"run_status",
            b"evidence_json",
        ],
    ),
    (b"meta", &[b"key", b"value"]),
    (
        b"objects",
        &[
            b"object_id",
            b"object_revision",
            b"kind",
            b"batch_id",
            b"branch",
            b"dir_ab",
            b"dir_bc",
            b"window_start",
            b"window_mid",
            b"window_end",
            b"comparisons_json",
            b"input_refs_json",
            b"source_coords_json",
            b"first_known_generation",
            b"first_known_cut",
            b"published_generation",
            b"withdrawn_generation",
            b"withdrawal_reason",
            b"superseded_by",
        ],
    ),
    (
        b"observations",
        &[
            b"observation_id",
            b"batch_id",
            b"kind",
            b"window_start",
            b"window_mid",
            b"window_end",
            b"reason",
            b"detail_json",
            b"published_generation",
        ],
    ),
    (
        b"raw_events",
        &[
            b"identity_key",
            b"revision",
            b"input_revision",
            b"payload_hash",
            b"receipt_id",
            b"seq",
            b"source_namespace",
            b"source_epoch",
            b"instrument",
            b"event_id",
            b"received_at",
            b"raw_text",
            b"price",
            b"ts",
            b"volume",
            b"source_coord",
            b"supersedes_revision",
        ],
    ),
    (
        b"raw_ohlc",
        &[
            b"identity_key",
            b"revision",
            b"schema_revision",
            b"open",
            b"high",
            b"low",
            b"close",
        ],
    ),
    (
        b"relations",
        &[
            b"relation_id",
            b"subject",
            b"relation_type",
            b"object",
            b"published_generation",
        ],
    ),
    (
        b"s_clock_events",
        &[
            b"clock_event_id",
            b"phase",
            b"attempt_index",
            b"semantic_ns",
            b"message_key",
            b"attempt_ordinal",
            b"generation",
        ],
    ),
    (
        b"s_delivery_policy",
        &[
            b"singleton",
            b"session_id",
            b"session_generation",
            b"policy_revision",
            b"retain_generations",
            b"first_available_generation",
            b"head_generation",
        ],
    ),
    (b"s_delivery_refs", &[b"generation"]),
    (b"s_evidence_format", &[b"singleton", b"graph_format"]),
    (
        b"s_evidence_nodes",
        &[b"node_id", b"node_kind", b"payload", b"expanded_length"],
    ),
    (
        b"s_evidence_roots",
        &[
            b"root_key",
            b"node_id",
            b"slot_ids",
            b"expanded_length",
            b"content_sha256",
        ],
    ),
    (
        b"s_input_messages",
        &[
            b"source_namespace",
            b"source_epoch",
            b"message_id",
            b"payload_hash",
            b"canonical_envelope",
            b"receipt_id",
            b"accepted_seq",
            b"status",
            b"first_published_generation",
            b"clock_event_id",
            b"attempt_count",
        ],
    ),
    (
        b"s_protocol_meta",
        &[
            b"singleton",
            b"protocol_revision",
            b"session_generation",
            b"clock_plan_hash",
            b"clock_plan_json",
            b"next_attempt_ordinal",
            b"logical_phase_frontier",
        ],
    ),
    (b"s_storage_format", &[b"singleton", b"storage_format"]),
    (
        b"structure_deltas",
        &[
            b"generation",
            b"session_id",
            b"catalog_revision",
            b"base_cut",
            b"next_cut",
            b"seq_range_json",
            b"index_frontier",
            b"input_frontier",
            b"catalog_run_status",
            b"catalog_evidence_json",
            b"delta_json",
            b"policy_records",
        ],
    ),
    (
        b"structure_facts",
        &[
            b"object_id",
            b"object_revision",
            b"kind",
            b"batch_id",
            b"fact_key",
            b"payload_json",
            b"input_refs_json",
            b"source_coords_json",
            b"first_known_generation",
            b"first_known_cut",
            b"published_generation",
            b"withdrawn_generation",
            b"withdrawal_reason",
            b"superseded_by",
        ],
    ),
    (
        b"witnesses",
        &[
            b"witness_id",
            b"object_id",
            b"slot",
            b"merged_source_index",
            b"merged_high",
            b"merged_low",
            b"merged_open",
            b"merged_close",
            b"raw_json",
            b"published_generation",
        ],
    ),
    (
        b"writer_epoch_history",
        &[
            b"ordinal",
            b"from_epoch",
            b"to_epoch",
            b"generation_at_transition",
            b"advance_state_at_transition",
            b"transitioned_at",
        ],
    ),
];

/// Fixed-size set; never learns columns from the database under inspection.
#[derive(Clone, Copy, Debug)]
pub(super) struct ExpectedTables(u32);
impl ExpectedTables {
    pub(super) fn empty() -> Self {
        Self(0)
    }
    pub(super) fn selected(v2: bool, ohlc: bool, delta: bool, graph: bool) -> Self {
        let mut result = Self::empty();
        for (i, (name, _)) in TABLES.iter().enumerate() {
            let include = match *name {
                b"s_protocol_meta" | b"s_input_messages" | b"s_clock_events"
                | b"s_delivery_policy" | b"s_delivery_refs" => v2,
                b"raw_ohlc" | b"structure_facts" => ohlc,
                b"s_storage_format" => delta,
                b"s_evidence_format" | b"s_evidence_nodes" | b"s_evidence_roots" => graph,
                _ => true,
            };
            if include {
                result.0 |= 1 << i;
            }
        }
        result
    }
    #[cfg(test)]
    pub(super) fn include(&mut self, name: &str) -> Result<(), &'static str> {
        let i = TABLES
            .iter()
            .position(|(n, _)| *n == name.as_bytes())
            .ok_or("MixedSqlUnknownExpectedTable")?;
        self.0 |= 1 << i;
        Ok(())
    }
    fn columns(&self, table: &[u8]) -> Option<&'static [&'static [u8]]> {
        TABLES.iter().enumerate().find_map(|(i, (name, columns))| {
            (*name == table && self.0 & (1 << i) != 0).then_some(*columns)
        })
    }
    fn contains(&self, table: &[u8]) -> bool {
        self.columns(table).is_some()
    }
    pub(super) fn allows(
        &self,
        action: c_int,
        a: Option<&[u8]>,
        b: Option<&[u8]>,
        db: Option<&[u8]>,
        origin: Option<&[u8]>,
        exclusive_armed: bool,
    ) -> bool {
        if origin.is_some() || !matches!(db, None | Some(b"main")) {
            return false;
        }
        match action {
            ffi::SQLITE_READ if b == Some(b"") => {
                db.is_none()
                    && a.is_some_and(|t| {
                        self.contains(t)
                            && (super::fd_wal::COUNT_SCAN_TABLES.contains(&t)
                                || t == b"s_evidence_nodes")
                    })
            }
            ffi::SQLITE_READ => match (a, b, db) {
                (Some(b"sqlite_master" | b"sqlite_schema"), Some(c), Some(b"main")) => {
                    matches!(c, b"type" | b"name" | b"sql")
                }
                (Some(b"pragma_table_info"), Some(c), None | Some(b"main")) => {
                    matches!(c, b"type" | b"notnull" | b"pk" | b"cid")
                }
                (Some(t), Some(c), Some(b"main")) => {
                    self.columns(t).is_some_and(|cs| cs.contains(&c))
                }
                _ => false,
            },
            ffi::SQLITE_PRAGMA if a == Some(b"table_info") => b.is_some_and(|t| self.contains(t)),
            _ => super::fd_wal::audit_sql_allowed(action, a, b, db, origin, exclusive_armed),
        }
    }
}

/// 原序捕获先于 schema 判别：此时只开放这些真实前缀查询的固定列。
/// 不从待审库登记学表/列；schema 完成判别后整个前缀权限由 Selected 替换。
#[derive(Clone, Copy, Debug)]
pub(super) enum MixedPolicy {
    Bootstrap,
    Capture { graph: bool, records_in_graph: bool },
    Selected(ExpectedTables),
}
impl MixedPolicy {
    pub(super) fn allows(
        &self,
        action: c_int,
        a: Option<&[u8]>,
        b: Option<&[u8]>,
        db: Option<&[u8]>,
        origin: Option<&[u8]>,
        armed: bool,
    ) -> bool {
        if let Self::Selected(selected) = self {
            return selected.allows(action, a, b, db, origin, armed);
        }
        if origin.is_some() || !matches!(db, None | Some(b"main")) {
            return false;
        }
        let (graph, records) = match self {
            Self::Capture {
                graph,
                records_in_graph,
            } => (*graph, *records_in_graph),
            _ => (false, false),
        };
        if action == ffi::SQLITE_READ {
            match (a, b, db) {
                (Some(b"meta"), Some(b"key" | b"value"), Some(b"main")) => return true,
                (Some(b"structure_deltas"), Some(b"index_frontier"), Some(b"main")) => return true,
                // read_format_typed 的原判别；格式成功后，LegacyBlob 不再开放此表。
                (
                    Some(b"s_evidence_format"),
                    Some(b"singleton" | b"graph_format"),
                    Some(b"main"),
                ) if matches!(self, Self::Bootstrap) || graph => return true,
                (
                    Some(b"batches"),
                    Some(b"batch_id" | b"canonical_bytes" | b"byte_len"),
                    Some(b"main"),
                ) if matches!(self, Self::Capture { .. }) => return true,
                (Some(b"structure_deltas"), Some(b"generation"), Some(b"main")) if records => {
                    return true
                }
                (Some(b"structure_facts"), Some(b"object_id"), Some(b"main")) if records => {
                    return true
                }
                (Some(t @ (b"s_evidence_nodes" | b"s_evidence_roots")), Some(c), Some(b"main"))
                    if graph =>
                {
                    return TABLES
                        .iter()
                        .find(|(name, _)| *name == t)
                        .is_some_and(|(_, columns)| columns.contains(&c));
                }
                (Some(b"s_evidence_nodes"), Some(b""), None) if graph => return true,
                (Some(b"structure_deltas"), Some(b""), None) if records => return true,
                _ => {}
            }
        }
        ExpectedTables::empty().allows(action, a, b, db, origin, armed)
    }
}

#[cfg(test)]
pub(super) fn test_progressive_policy<T>(
    conn: &rusqlite::Connection,
    observer: &dyn crate::v3::audit_budget::LegacyObserver,
    run: impl FnOnce(&dyn crate::v3::audit_budget::LegacyObserver) -> T,
) -> T {
    use crate::v3::audit_budget::{LegacyObserver, LegacyRecordRef};
    use newchan_rust::s_content::{self as sc, capture_allocation::scratch::WorkScope};
    use std::cell::{Cell, RefCell};
    use std::ffi::{c_char, c_void, CStr};
    struct PolicyObserver<'a> {
        policy: Cell<MixedPolicy>,
        inner: &'a dyn LegacyObserver,
        denied: RefCell<Vec<String>>,
    }
    impl LegacyObserver for PolicyObserver<'_> {
        fn select_legacy_capture(&self, graph: bool, records_in_graph: bool) {
            self.policy.set(MixedPolicy::Capture {
                graph,
                records_in_graph,
            });
        }
        fn select_legacy_tables(&self, v2: bool, ohlc: bool, delta: bool, graph: bool) {
            self.policy
                .set(MixedPolicy::Selected(ExpectedTables::selected(
                    v2, ohlc, delta, graph,
                )));
        }
        fn visit_record(
            &self,
            f: &'static str,
            p: &'static str,
            m: LegacyRecordRef<'_>,
        ) -> Result<(), sc::CodecError> {
            self.inner.visit_record(f, p, m)
        }
        fn precheck(&self) -> Result<(), sc::CodecError> {
            self.inner.precheck()
        }
        fn check_work(&self, w: &WorkScope) -> Result<(), sc::CodecError> {
            self.inner.check_work(w)
        }
        fn accept_checked(
            &self,
            r: &sc::legacy_evidence::CheckedLegacyEvidenceRow,
        ) -> Result<(), sc::CodecError> {
            self.inner.accept_checked(r)
        }
    }
    unsafe extern "C" fn authorize(
        data: *mut c_void,
        action: c_int,
        a: *const c_char,
        b: *const c_char,
        db: *const c_char,
        origin: *const c_char,
    ) -> c_int {
        unsafe fn arg<'a>(p: *const c_char) -> Option<&'a [u8]> {
            if p.is_null() {
                None
            } else {
                Some(unsafe { CStr::from_ptr(p) }.to_bytes())
            }
        }
        let state = unsafe { &*data.cast::<PolicyObserver<'_>>() };
        let (a, b, db, origin) = unsafe { (arg(a), arg(b), arg(db), arg(origin)) };
        if state.policy.get().allows(action, a, b, db, origin, true) {
            ffi::SQLITE_OK
        } else {
            state
                .denied
                .borrow_mut()
                .push(format!("{action} {a:?} {b:?} {db:?}"));
            ffi::SQLITE_DENY
        }
    }
    let tx = if conn.is_autocommit() {
        Some(conn.unchecked_transaction().unwrap())
    } else {
        None
    };
    let state = PolicyObserver {
        policy: Cell::new(MixedPolicy::Bootstrap),
        inner: observer,
        denied: RefCell::new(Vec::new()),
    };
    unsafe {
        assert_eq!(
            ffi::sqlite3_set_authorizer(
                conn.handle(),
                Some(authorize),
                (&state as *const PolicyObserver<'_>).cast_mut().cast()
            ),
            ffi::SQLITE_OK
        );
    }
    let result = run(&state);
    unsafe {
        ffi::sqlite3_set_authorizer(conn.handle(), None, std::ptr::null_mut());
    }
    if let Some(tx) = tx {
        tx.rollback().unwrap();
    }
    assert!(
        state.denied.borrow().is_empty(),
        "progressive callback denied: {:?}",
        state.denied.borrow()
    );
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::collections::BTreeMap;
    use std::ffi::{c_char, c_void, CStr};
    struct Capture {
        policy: ExpectedTables,
        calls: Vec<String>,
    }
    unsafe fn arg<'a>(p: *const c_char) -> Option<&'a [u8]> {
        if p.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(p) }.to_bytes())
        }
    }
    unsafe extern "C" fn authorize(
        data: *mut c_void,
        action: c_int,
        a: *const c_char,
        b: *const c_char,
        db: *const c_char,
        origin: *const c_char,
    ) -> c_int {
        let c = unsafe { &mut *data.cast::<Capture>() };
        let (a, b, db, origin) = unsafe { (arg(a), arg(b), arg(db), arg(origin)) };
        let allowed = c.policy.allows(action, a, b, db, origin, true);
        c.calls.push(format!(
            "{action} {a:?} {b:?} {db:?} {origin:?} allowed={allowed}"
        ));
        if allowed {
            ffi::SQLITE_OK
        } else {
            ffi::SQLITE_DENY
        }
    }
    pub(super) fn with_policy<T>(conn: &Connection, run: impl FnOnce() -> T) -> T {
        let tx = if conn.is_autocommit() {
            Some(conn.unchecked_transaction().unwrap())
        } else {
            None
        };
        let mut capture = Capture {
            policy: ExpectedTables::selected(true, true, true, true),
            calls: Vec::new(),
        };
        unsafe {
            assert_eq!(
                ffi::sqlite3_set_authorizer(
                    conn.handle(),
                    Some(authorize),
                    (&mut capture as *mut Capture).cast()
                ),
                ffi::SQLITE_OK
            );
        }
        let result = run();
        unsafe {
            ffi::sqlite3_set_authorizer(conn.handle(), None, std::ptr::null_mut());
        }
        if let Some(tx) = tx {
            tx.rollback().unwrap();
        }
        assert!(
            !capture.calls.iter().any(|c| c.ends_with("allowed=false")),
            "callbacks: {:?}",
            capture.calls
        );
        result
    }

    #[test]
    fn mixed_sql_bootstrap_and_typed_tightening_reject_extra_columns() {
        let read = |p: MixedPolicy, t: &[u8], c: &[u8]| {
            p.allows(
                ffi::SQLITE_READ,
                Some(t),
                Some(c),
                Some(b"main"),
                None,
                true,
            )
        };
        assert!(read(
            MixedPolicy::Bootstrap,
            b"structure_deltas",
            b"index_frontier"
        ));
        assert!(read(
            MixedPolicy::Bootstrap,
            b"s_evidence_format",
            b"graph_format"
        ));
        assert!(!read(
            MixedPolicy::Bootstrap,
            b"structure_deltas",
            b"delta_json"
        ));
        assert!(!read(
            MixedPolicy::Bootstrap,
            b"s_evidence_nodes",
            b"payload"
        ));
        let legacy = MixedPolicy::Capture {
            graph: false,
            records_in_graph: false,
        };
        assert!(read(legacy, b"batches", b"canonical_bytes"));
        assert!(!read(legacy, b"s_evidence_format", b"graph_format"));
        assert!(!read(legacy, b"structure_facts", b"object_id"));
        let graph = MixedPolicy::Capture {
            graph: true,
            records_in_graph: true,
        };
        assert!(read(graph, b"structure_facts", b"object_id"));
        assert!(!read(graph, b"structure_facts", b"payload_json"));
        let selected = MixedPolicy::Selected(ExpectedTables::selected(false, false, false, false));
        assert!(!read(selected, b"s_evidence_format", b"graph_format"));
        assert!(!read(selected, b"structure_facts", b"object_id"));
    }
    #[test]
    fn mixed_sql_selected_schema_is_exact_for_each_domain() {
        for v2 in [false, true] {
            for ohlc in [false, true] {
                for delta in [false, true] {
                    for graph in [false, true] {
                        let mut expected = BTreeMap::new();
                        for ddl in [
                            crate::SCHEMA,
                            if v2 { crate::v2::SCHEMA_V2 } else { "" },
                            if ohlc { crate::tb02a::SCHEMA } else { "" },
                            if delta { crate::delta_storage::DDL } else { "" },
                            if graph {
                                crate::evidence_store::DDL
                            } else {
                                ""
                            },
                        ] {
                            crate::v2::expect_ddl_tables("selected constants", ddl, &mut expected)
                                .unwrap();
                        }
                        let policy = ExpectedTables::selected(v2, ohlc, delta, graph);
                        for (name, _) in TABLES {
                            let name = std::str::from_utf8(name).unwrap();
                            assert_eq!(
                                policy.contains(name.as_bytes()),
                                expected.contains_key(name),
                                "{name} v2={v2} ohlc={ohlc} delta={delta} graph={graph}"
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn mixed_sql_manifest_columns_equal_compile_time_ddl() {
        let conn = Connection::open_in_memory().unwrap();
        for ddl in [
            crate::SCHEMA,
            crate::v2::SCHEMA_V2,
            crate::tb02a::SCHEMA,
            crate::delta_storage::DDL,
            crate::evidence_store::DDL,
        ] {
            conn.execute_batch(ddl).unwrap();
        }
        let column_names = |table: &str| {
            let mut stmt = conn
                .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
                .unwrap();
            stmt.query_map([table], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .unwrap();
        let ddl: Vec<(String, Vec<String>)> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|name| {
                let name = name.unwrap();
                let columns = column_names(&name);
                (name, columns)
            })
            .collect();
        let manifest: Vec<(String, Vec<String>)> = TABLES
            .iter()
            .map(|(name, columns)| {
                let text = |b: &[u8]| String::from_utf8(b.to_vec()).unwrap();
                (text(name), columns.iter().map(|c| text(c)).collect())
            })
            .collect();
        assert_eq!(
            manifest, ddl,
            "mixed manifest must equal the five compile-time DDL constants"
        );
    }
    #[test]
    fn mixed_sql_real_root_callbacks_close_under_selected_policy() {
        crate::v2::with_mixed_control_fixture(|conn| {
            let result = with_policy(conn, || crate::verified_root_raw_events(conn));
            assert!(result.is_ok(), "root: {result:?}");
        });
    }
    #[test]
    fn mixed_sql_manifest_actual_schema_callbacks_and_negative_controls() {
        for graph in [false, true] {
            let conn = Connection::open_in_memory().unwrap();
            let mut expected = BTreeMap::new();
            for ddl in [
                crate::SCHEMA,
                crate::v2::SCHEMA_V2,
                crate::tb02a::SCHEMA,
                crate::delta_storage::DDL,
                if graph {
                    crate::evidence_store::DDL
                } else {
                    ""
                },
            ] {
                conn.execute_batch(ddl).unwrap();
                crate::v2::expect_ddl_tables("mixed test", ddl, &mut expected).unwrap();
            }
            // Unknown columns exist physically but never enter the compile-time manifest.
            conn.execute_batch("CREATE TABLE forbidden(x TEXT); CREATE VIEW hidden_view AS SELECT value FROM meta;").unwrap();
            let mut policy = ExpectedTables::empty();
            for name in expected.keys() {
                policy.include(name).unwrap();
            }
            assert!(policy.include("forbidden").is_err());
            let mut capture = Capture {
                policy,
                calls: Vec::new(),
            };
            unsafe {
                assert_eq!(
                    ffi::sqlite3_set_authorizer(
                        conn.handle(),
                        Some(authorize),
                        (&mut capture as *mut Capture).cast()
                    ),
                    ffi::SQLITE_OK
                );
            }
            for sql in [
                "SELECT x FROM forbidden",
                "SELECT value FROM hidden_view",
                "DELETE FROM meta",
                "ATTACH ':memory:' AS other",
                "PRAGMA table_info(forbidden)",
                "SELECT random()",
                "SELECT rootpage FROM sqlite_schema",
            ] {
                assert!(
                    conn.prepare(sql)
                        .and_then(|mut s| s.query([]).map(|_| ()))
                        .is_err(),
                    "{sql}"
                );
            }
            unsafe {
                ffi::sqlite3_set_authorizer(conn.handle(), None, std::ptr::null_mut());
            }
            conn.execute_batch("DROP VIEW hidden_view; DROP TABLE forbidden;")
                .unwrap();
            unsafe {
                ffi::sqlite3_set_authorizer(
                    conn.handle(),
                    Some(authorize),
                    (&mut capture as *mut Capture).cast(),
                );
            }
            let result = crate::v2::verify_expected_schema(&conn, &expected);
            if result.is_err() {
                for call in &capture.calls {
                    eprintln!("{call}");
                }
            }
            assert!(result.is_ok(), "schema graph={graph}: {result:?}");
            for table in ["catalog", "raw_events", "raw_ohlc", "structure_deltas"] {
                let n: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                    .unwrap();
                assert_eq!(n, 0);
            }
            if graph {
                let n: i64 = conn
                    .query_row("SELECT COUNT(*) FROM s_evidence_nodes", [], |r| r.get(0))
                    .unwrap();
                assert_eq!(n, 0);
            } else {
                assert!(!policy.allows(
                    ffi::SQLITE_READ,
                    Some(b"s_evidence_nodes"),
                    Some(b"payload"),
                    Some(b"main"),
                    None,
                    true
                ));
            }
            assert!(!policy.allows(
                ffi::SQLITE_READ,
                Some(b"meta"),
                Some(b"undeclared"),
                Some(b"main"),
                None,
                true
            ));
            let deltas = |column: &[u8]| {
                policy.allows(
                    ffi::SQLITE_READ,
                    Some(b"structure_deltas"),
                    Some(column),
                    Some(b"main"),
                    None,
                    true,
                )
            };
            assert!(deltas(b"policy_records"));
            assert!(!deltas(b"policy_records_extra"));
            assert!(!policy.allows(
                ffi::SQLITE_READ,
                Some(b"meta"),
                Some(b"value"),
                Some(b"other"),
                None,
                true
            ));
            assert!(!policy.allows(
                ffi::SQLITE_READ,
                Some(b"raw_events"),
                Some(b""),
                Some(b"main"),
                None,
                true
            ));
            unsafe {
                ffi::sqlite3_set_authorizer(conn.handle(), None, std::ptr::null_mut());
            }
            eprintln!(
                "mixed schema graph={graph}: {} actual SQLite authorizer callbacks",
                capture.calls.len()
            );
        }
    }
}
