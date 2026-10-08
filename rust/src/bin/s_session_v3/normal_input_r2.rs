// #1394 r2：原样正常输入定位；仅由 prepared::tests 编译，保留生产错误和红测试。
use super::*;
use std::cell::RefCell;

thread_local! {
    static OBSERVATION: RefCell<Option<(&'static str, usize, &'static str)>> = const { RefCell::new(None) };
}

pub(crate) fn enabled() -> bool {
    OBSERVATION.with(|context| context.borrow().is_some())
}

pub(crate) fn observe(event: &str, details: Value) {
    OBSERVATION.with(|context| {
        if let Some((scenario, step, phase)) = *context.borrow() {
            eprintln!("R2_NORMAL {}", json!({"scenario":scenario,"step":step,"phase":phase,"event":event,"details":details}));
        }
    });
}

fn phase(scenario: &'static str, step: usize, phase: &'static str) {
    OBSERVATION.with(|context| *context.borrow_mut() = Some((scenario, step, phase)));
}

struct Observation;
impl Drop for Observation {
    fn drop(&mut self) {
        OBSERVATION.with(|context| *context.borrow_mut() = None);
    }
}

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
    phase(scenario, 0, "fixture");
    let _observation = Observation;
    let mut fixture = Fixture::new();
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("s_session");
    observe(
        "fixture",
        json!({
            "directory":fixture.path,"input":rows,
            "clock":serde_json::from_str::<Value>(&std::fs::read_to_string(fixture.path.join("clock.json")).unwrap()).unwrap(),
            "catalog_sha256":sha256_hex(&std::fs::read(base.join("catalog/signed-catalog.json")).unwrap()),
            "profile_sha256":sha256_hex(&std::fs::read(base.join("profiles/ohlc_integer_tb02a_v1.json")).unwrap())
        }),
    );
    for (offset, (id, coord, open, high, low, close, revision)) in rows.into_iter().enumerate() {
        let step = offset + 1;
        phase(scenario, step, "accept_begin");
        let (envelope, token) =
            fixture.accept_begin(step as i64, (id, coord, open, high, low, close), revision);
        // 原 Fixture 返回意味着两个事务都成功提交；不复制或替换夹具接纳/Begin。
        observe(
            "accept_ok",
            json!({"raw":read_raw_events(&fixture.conn).unwrap()}),
        );
        observe(
            "begin_ok",
            json!({"generation":meta_i64(&fixture.conn,"generation").unwrap(),"next_generation":step,"frontier":read_raw_events(&fixture.conn).unwrap().len() as i64-1}),
        );
        phase(scenario, step, "publish");
        observe("publish_enter", json!({}));
        let result = fixture.publish(&envelope, &token);
        match &result {
            Ok(receipt) => observe(
                "publish_ok",
                json!({"generation":receipt.generation,"snapshot":receipt.snapshot.root,"delta":receipt.delta.root}),
            ),
            Err(error) => observe(
                "publish_error",
                json!({"error":error,"persisted_generation":meta_i64(&fixture.conn,"generation").unwrap()}),
            ),
        }
        result.expect("原样正常输入必须 publish 成功；错误保留为红测试");
        phase(scenario, step, "ordinary_audit_after_publish");
        let tx = fixture.conn.transaction().unwrap();
        let result = state::audit(&tx);
        match &result {
            Ok(audited) => observe("ordinary_audit_ok", audit_summary(audited)),
            Err(error) => observe("ordinary_audit_error", json!({"error":error})),
        }
        result.expect("publish 后普通审计必须成功");
    }
    phase(scenario, 4, "final_ordinary_native_parity");
    let tx = fixture.conn.transaction().unwrap();
    let ordinary = state::audit(&tx).expect("最终普通审计必须成功");
    let native = state::audit_native_current(&tx).expect("最终Native审计必须成功");
    assert_eq!(audit_summary(&ordinary), audit_summary(&native));
    observe("final_parity_ok", audit_summary(&ordinary));
}

#[cfg(feature = "s_capture_native_testonly")]
#[test]
fn r2_normal_unordered_same_ohlc() {
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

#[cfg(feature = "s_capture_native_testonly")]
#[test]
fn r2_normal_ordered_same_ohlc() {
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

#[cfg(feature = "s_capture_native_testonly")]
#[test]
fn r2_normal_earlier_revision() {
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
