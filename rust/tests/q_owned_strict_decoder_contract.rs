//! #1394 独立旧 Strict oracle、实际额度拒绝和诊断持有验证。
#![cfg(feature = "s_allocation_json_testonly")]

use newchan_rust::{
    s_content::capture_allocation::{audit_json::strict, OwnedBudget, LIMIT},
    session_protocol::strict_json,
};

const FIXTURE_LIMIT: usize = 50 * 1024 * 1024;

fn compare(bytes: &[u8], budget: &OwnedBudget) {
    let original = strict_json(bytes);
    let actual = strict::parse(bytes, budget);
    match (original, actual) {
        (Ok(original), Ok(actual)) => assert_eq!(
            serde_json::to_vec(&original).unwrap(),
            serde_json::to_vec(&actual.view()).unwrap(),
            "完整规范值字节，输入={bytes:?}"
        ),
        (Err(original), Err(strict::ParseFailure::Syntax(actual))) => {
            assert_eq!(
                original,
                format!("SchemaUnsupported：{actual}"),
                "{bytes:?}"
            );
            let displayed = actual.to_string();
            for needle in [
                "",
                "duplicate key",
                "key ",
                " at line ",
                " column ",
                "CaptureBytes",
                "prototype_capture_owned_payload_limit",
                "中文",
                "0",
                "1",
                "🙂",
            ] {
                assert_eq!(actual.contains(needle), displayed.contains(needle));
            }
        }
        (Err(error), Ok(_)) => panic!("非法输入被接纳：{bytes:?}，旧首错={error}"),
        (Ok(_), Err(error)) => panic!("合法输入被拒绝：{bytes:?}，新首错={error:?}"),
        (_, Err(strict::ParseFailure::Allocation(error))) => {
            panic!("该50MiB具名输入应有足够真实额度：{bytes:?}，{error}")
        }
    }
    let state = budget.snapshot();
    assert_eq!(state.live_requested_layout_bytes, 0);
    assert_eq!(state.allocated_layouts, state.released_layouts);
    assert!(!budget.ledger_health().invariant_broken);
}

#[test]
fn old_strict_oracle_exhaustive_short_bytes_and_structured_mutations() {
    let budget = OwnedBudget::new(FIXTURE_LIMIT);
    let alphabet = b"{}[]\"\\:,ntf01-.e \n\t\0\xff";
    let mut count = 0;
    for len in 0..=3u32 {
        for mut code in 0..alphabet.len().pow(len) {
            let mut input = vec![0; len as usize];
            for byte in &mut input {
                *byte = alphabet[code % alphabet.len()];
                code /= alphabet.len();
            }
            compare(&input, &budget);
            count += 1;
        }
    }
    let bases = [
        r#"{"é🙂":[null,true,false,{"中文":"\n\u00e9\uD83D\uDE42"}]}"#,
        r#"{"a":1,"b":2,"c":[-0,1.25,1e-400,18446744073709551616]}"#,
        r#"{"x":1,"x":2}"#,
        r#"[null,{"x":1,"x":2},"tail"]"#,
        r#""\uD800\u0041""#,
        "-9223372036854775809",
        "1e400",
        "[1,2,{\"\":[]} ]\n ",
        "{\n\"é\":0,\n\"é\" : 1}",
    ];
    for base in bases {
        compare(base.as_bytes(), &budget);
        count += 1;
        for i in 0..base.len() {
            let mut removed = base.as_bytes().to_vec();
            removed.remove(i);
            compare(&removed, &budget);
            count += 1;
            for byte in [b'"', b'\\', b':', b',', b' ', b'\n', 0, 0x80, 0xff] {
                let mut changed = base.as_bytes().to_vec();
                changed[i] = byte;
                compare(&changed, &budget);
                count += 1;
                let mut inserted = base.as_bytes().to_vec();
                inserted.insert(i, byte);
                compare(&inserted, &budget);
                count += 1;
            }
            compare(&base.as_bytes()[..i], &budget);
            count += 1;
        }
    }
    let mut seed = 0x1394_0151_2026_1003u64;
    for i in 0..4000 {
        let mut bytes = Vec::new();
        for _ in 0..i % 97 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            bytes.push((seed >> 24) as u8);
        }
        compare(&bytes, &budget);
        count += 1;
    }
    println!("OWNED_STRICT_DIFFERENTIAL_CASES {count}");
}

#[test]
fn duplicate_suffix_unicode_depth_and_numeric_boundaries_match_original() {
    let budget = OwnedBudget::new(FIXTURE_LIMIT);
    for key in [
        "",
        "中文🙂",
        "at line 2 column 3",
        " at line 0 column 9",
        "x at line 0 column 0",
        "x at line 7 column 0",
        "x at line 3 column 4",
        "x at line 18446744073709551616 column 1",
        "x at line 1 column 999999999999999999999",
        "x at line 2 column ",
        "x at line 2 column 3 suffix",
        "x at line 1 column 2 at line 0 column 0",
        "CaptureBytes",
        "prototype_capture_owned_payload_limit",
        "prototype_audit_allocation_incomplete",
    ] {
        let quoted = serde_json::to_string(key).unwrap();
        for tail in [":1}", " \n:1}", "}", "", " \n\n "] {
            let bytes = format!("{{{quoted}:0,{quoted}{tail}");
            compare(bytes.as_bytes(), &budget);
            let nested = format!("[{{\"parent\":{bytes}}}]");
            compare(nested.as_bytes(), &budget);
        }
    }
    for depth in [1, 2, 63, 126, 127, 128, 129] {
        for middle in ["null", "\"é🙂\"", "{\"x\":0,\"x\":1}"] {
            compare(
                format!("{}{middle}{}", "[".repeat(depth), "]".repeat(depth)).as_bytes(),
                &budget,
            );
        }
    }
    for input in [
        "0",
        "-0",
        "1e-400",
        "-1e-400",
        "1e309",
        "0e999999999999999999999999",
        "1e-999999999999999999999",
        "12345678901234567890123456789012345678901234567890.012345678901e-40",
        "9223372036854775807",
        "-9223372036854775808",
        "18446744073709551615",
        "18446744073709551616",
        "-9223372036854775809",
        "2.2250738585072014e-308",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
    ] {
        compare(input.as_bytes(), &budget);
        compare(format!("[ {input} ]").as_bytes(), &budget);
    }
}

#[test]
fn real_limits_refuse_growth_and_duplicate_key_stays_owned_until_drop() {
    let escaped = format!("\"{}\"", "\\u00e9".repeat(1024));
    let object = format!("{{\"key\":{escaped},\"rows\":[0,1,2,3,4,5,6,7]}}");
    for input in [escaped.as_bytes(), object.as_bytes()] {
        for limit in [0, 1, 3, 7, 16, 31, 64, 128, 256, 512, 1024, 4096] {
            let budget = OwnedBudget::new(limit);
            let result = strict::parse(input, &budget);
            if let Err(error) = &result {
                match error {
                    strict::ParseFailure::Allocation(cause) => {
                        assert_eq!(cause.code(), LIMIT);
                        assert!(budget.snapshot().rejected_layouts > 0);
                    }
                    other => panic!("合法输入只能因真实额度拒绝：{other:?}"),
                }
            }
            drop(result);
            let state = budget.snapshot();
            assert_eq!(state.live_requested_layout_bytes, 0);
            assert_eq!(state.allocated_layouts, state.released_layouts);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }
    let key = "é🙂\\\"".repeat(8192);
    let quoted = serde_json::to_string(&key).unwrap();
    let bytes = format!("{{{quoted}:0,{quoted}:1}}");
    let budget = OwnedBudget::new(FIXTURE_LIMIT);
    let result = strict::parse(bytes.as_bytes(), &budget);
    let diagnostic = match result {
        Err(strict::ParseFailure::Syntax(diagnostic)) => diagnostic,
        _ => panic!("具名重复键必须保持原syntax错误"),
    };
    assert_eq!(
        format!("SchemaUnsupported：{diagnostic}"),
        strict_json(bytes.as_bytes()).unwrap_err()
    );
    assert!(budget.snapshot().live_requested_layout_bytes >= key.len());
    drop(diagnostic);
    assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    assert_eq!(
        budget.snapshot().allocated_layouts,
        budget.snapshot().released_layouts
    );
}
