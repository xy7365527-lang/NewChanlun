//! p123 字节护栏——仓内常驻回归门（GitHub issue #533，#421 三审 Standards 债）。
//!
//! ## 工位定位
//!
//! #429/#430 三审曾对 `p123_fast_replay` 做过 stdout / `P421_LIFECYCLE_DUMP` 的逐字节
//! pre/post 对拍（`chanlun/review-results/shadow-429-trireview-20260728.md` §5.1），但产物
//! 只留在评审报告与 `/tmp` 人工文件里，不是仓内可持续回归门——下一次改动是否破坏这份
//! 字节等价，只能靠人工重跑三审流程才能发现。本文件把同一套对拍收成 `cargo test`。
//!
//! ## 两档方案（票面 Acceptance 第 3 条：数据体积过大时哈希断言 + 小样本全文对拍）
//!
//! - **小样本全文对拍**（`P116_MAX_BARS=2000`）：dump ≈450KB，check 进仓库
//!   （`fixtures/issue533_p123_2000_{stdout,dump}.golden.txt`），失败时给出可读 diff。
//! - **大窗哈希断言**（`P116_MAX_BARS=20000`/`100000`，对齐三审原窗口）：dump 分别
//!   ≈5MB/≈25MB，check 进仓库不现实——只 check SHA-256（`fixtures/issue533_p123_{20000,100000}.sha256`），
//!   失败时只能报告"变了"，不能报告"哪里变了"（这是本方案对大窗的已知局限，票内登记）。
//!
//! ## 第三个产物面：`P116_DUMP`（票 #619 L10 补齐）
//!
//! `p123_fast_replay.rs:199` 的 `P116_DUMP` 是与 stdout / `P421_LIFECYCLE_DUMP` 并列的独立
//! 产物面（#98/#99 调研侧信道）。本门原先只覆盖前两面 ⟹ 它漏在护栏之外，只能靠评审逐次
//! 手工 `cmp`（#603 链的回退与小修包两侧交付都漏列了它，影子评审 #619 L10 登记）。现三窗
//! 全部纳入，且**三窗都用全文对拍**而非哈希——该面体积 438B / 8.8KB / 21.6KB，比 dump 小
//! 三个数量级，进仓库无成本，没有理由退回"只能报告『变了』"的哈希断言。
//!
//! ## 数据前提（环境 vs 真漂移的区分，仿 `theta_v0_fixture_drift.rs` 的 exit 码分域）
//!
//! `analysis/data_cache/btc_1m_full.json` 被 `.gitignore:82` 排除（314MB 真实行情，不入库）。
//! 本地无该文件 ⟹ 环境缺失，不是字节漂移；测试用 `panic!` 前缀 `P533 ENVIRONMENT` 明确区分，
//! 不得与真漂移的 `panic!` 前缀混淆。
//!
//! ## CI 覆盖边界（票内登记口径）
//!
//! `.github/workflows/ci.yml` 未挂载 `analysis/data_cache/`（同一 `.gitignore` 排除，CI checkout
//! 不含该目录），因此本门**不能**像 `fixture-drift` job 那样接入 CI push/PR 触发——这是与
//! `m8_e2e_all_systems_oos`/`wverify_full` 等既有真实行情 `#[ignore]` 测试相同的既有边界，非本票
//! 新增缺口。本门满足票面"门挂接 cargo test **或** CI workflow"的前一选项：开发者/评审在本地或
//! 持有数据的机器上用 `-- --ignored` 显式触发。
//!
//! ## 跑法
//!
//! ```sh
//! cargo test --release --manifest-path rust/Cargo.toml \
//!   --test issue533_p123_byte_guardrail -- --ignored --nocapture
//! ```
//!
//! ## 认识论（formalization-validity-domain 231号）
//!
//! L2：BTC 单标的、四个固定前缀窗口（2000/20000/100000 三档 + 20000/100000 双窗）的真实数据
//! 字节级断言；不外推其它标的/窗口（非 L3）。
//!
//! ## golden 变更纪律（票面 Acceptance 第 2 条）
//!
//! golden 文件只允许因**已审阅、故意的**行为变化而更新，且更新须在同一 PR 里说明原因
//! （引用相应 issue/report）；禁止为了让测试变绿而静默重新生成 golden。
//!
//! 变更登记（每次动 golden 都在此追加一行，便于 `git blame` 之外的正向可查）：
//!
//! | commit | 动了哪些 golden | 原因 |
//! |---|---|---|
//! | `fa912ba991` | 2000 dump + 20k/100k `dump` 行 | #603 档1+档2 新增诊断字段（impl 报告 §5） |
//! | `0e0d011da2` | 20k/100k `dump` 行（2000 回落至 `6ebe18eda1`） | #603 档2 回退（revert 报告「golden fixture」节） |
//! | `dc2b7dd48b` | 2000 dump + 20k/100k `dump` 行 | #618 小修包：诊断行补 `seg_a` 完整锚（**当时无仓内说明——影子评审 #619 M3；说明已补在 `chanlun/review-results/issue619-condition-closure-20260728.md` §M3 与 #533 issue 评论**） |
//! | 本次（#619） | **无重锚**：新增三份 `*_p116.golden.txt` | L10 补齐 P116 面；stdout/dump 三窗与 `dc2b7dd48b` 逐字节相同（零漂移，见报告 §验收） |

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/ 的父目录 = 项目根")
        .to_path_buf()
}

fn btc_data_path() -> PathBuf {
    project_root().join("analysis/data_cache/btc_1m_full.json")
}

fn require_btc_data() -> PathBuf {
    let path = btc_data_path();
    assert!(
        path.is_file(),
        "P533 ENVIRONMENT（非漂移）：缺 {}——该文件被 .gitignore:82 排除，\
         不随 checkout 到位；本地/CI 机器需先备好真实 BTC 1min 数据才能跑本门",
        path.display()
    );
    path
}

fn p123_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_p123_fast_replay"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn read_golden(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("读 golden {} 失败：{e}", path.display()))
}

fn read_golden_sha_map(name: &str) -> std::collections::BTreeMap<String, String> {
    let text = String::from_utf8(read_golden(name)).expect("golden sha256 文件必须是 UTF-8");
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.split_whitespace();
            let key = parts.next().expect("每行须 `<label> <hex>`").to_string();
            let hex = parts.next().expect("每行须 `<label> <hex>`").to_string();
            (key, hex)
        })
        .collect()
}

struct P123Run {
    stdout: Vec<u8>,
    dump: Vec<u8>,
    /// `P116_DUMP` 侧信道产物（票 #619 L10 纳入）。
    p116: Vec<u8>,
}

fn run_p123(max_bars: usize, dump_path: &Path, p116_path: &Path) -> P123Run {
    let data = require_btc_data();
    let output = Command::new(p123_binary())
        .arg(&data)
        .env("P116_MAX_BARS", max_bars.to_string())
        .env("P421_LIFECYCLE_DUMP", dump_path)
        .env("P116_DUMP", p116_path)
        .output()
        .unwrap_or_else(|e| panic!("P533 ENVIRONMENT（非漂移）：启动 p123_fast_replay 失败：{e}"));
    assert!(
        output.status.success(),
        "p123_fast_replay 退出非零（max_bars={max_bars}）：stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dump = std::fs::read(dump_path).unwrap_or_else(|e| {
        panic!(
            "读 P421_LIFECYCLE_DUMP 产物 {} 失败：{e}",
            dump_path.display()
        )
    });
    let p116 = std::fs::read(p116_path).unwrap_or_else(|e| {
        panic!("读 P116_DUMP 产物 {} 失败：{e}", p116_path.display())
    });
    P123Run {
        stdout: output.stdout,
        dump,
        p116,
    }
}

/// 小样本全文对拍：2000 bar 前缀，stdout + lifecycle dump 与仓内 golden 逐字节相等。
/// 失败时 `assert_eq!` 打印可读 diff（Rust 测试框架对 `String` 的默认行为）。
#[test]
#[ignore = "字节护栏：需本地 analysis/data_cache/btc_1m_full.json；-- --ignored 单独触发"]
fn p123_replay_2000bars_byte_exact() {
    let dump_path = std::env::temp_dir().join("issue533_p123_2000_dump.actual.txt");
    let p116_path = std::env::temp_dir().join("issue533_p123_2000_p116.actual.txt");
    let run = run_p123(2000, &dump_path, &p116_path);

    let golden_stdout = read_golden("issue533_p123_2000_stdout.golden.txt");
    let golden_dump = read_golden("issue533_p123_2000_dump.golden.txt");
    let golden_p116 = read_golden("issue533_p123_2000_p116.golden.txt");

    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&golden_stdout),
        "P533 DRIFT：2000-bar stdout 与仓内 golden 不再逐字节相等"
    );
    assert_eq!(
        String::from_utf8_lossy(&run.dump),
        String::from_utf8_lossy(&golden_dump),
        "P533 DRIFT：2000-bar lifecycle dump 与仓内 golden 不再逐字节相等"
    );
    assert_eq!(
        String::from_utf8_lossy(&run.p116),
        String::from_utf8_lossy(&golden_p116),
        "P533 DRIFT：2000-bar P116_DUMP 与仓内 golden 不再逐字节相等（票 #619 L10）"
    );
}

/// 大窗断言：20k/100k 前缀（对齐 #429/#430 三审原窗口）。
///
/// - stdout / lifecycle dump：SHA-256 与仓内 golden 哈希相等。哈希不等 ⟹ 字节流已变——但不
///   给出字段级 diff（票内登记的已知局限，数据量过大不适合全文 check 进仓库）。
/// - `P116_DUMP`（票 #619 L10）：**全文对拍**——该面 8.8KB / 21.6KB，不受上述体积约束。
#[test]
#[ignore = "字节护栏：需本地 analysis/data_cache/btc_1m_full.json；-- --ignored 单独触发"]
fn p123_replay_large_windows_hash_guardrail() {
    for (max_bars, golden_name, p116_golden_name) in [
        (
            20_000usize,
            "issue533_p123_20000.sha256",
            "issue533_p123_20000_p116.golden.txt",
        ),
        (
            100_000usize,
            "issue533_p123_100000.sha256",
            "issue533_p123_100000_p116.golden.txt",
        ),
    ] {
        let dump_path =
            std::env::temp_dir().join(format!("issue533_p123_{max_bars}_dump.actual.txt"));
        let p116_path =
            std::env::temp_dir().join(format!("issue533_p123_{max_bars}_p116.actual.txt"));
        let run = run_p123(max_bars, &dump_path, &p116_path);
        let golden = read_golden_sha_map(golden_name);

        // P116 面走全文对拍（体积比 dump 小三个数量级，失败时给出字段级 diff；票 #619 L10）。
        assert_eq!(
            String::from_utf8_lossy(&run.p116),
            String::from_utf8_lossy(&read_golden(p116_golden_name)),
            "P533 DRIFT：max_bars={max_bars} P116_DUMP 与仓内 golden 不再逐字节相等"
        );

        let stdout_sha = sha256_hex(&run.stdout);
        let dump_sha = sha256_hex(&run.dump);
        assert_eq!(
            &stdout_sha,
            golden.get("stdout").expect("golden 缺 stdout 行"),
            "P533 DRIFT：max_bars={max_bars} stdout SHA-256 与仓内 golden 不等"
        );
        assert_eq!(
            &dump_sha,
            golden.get("dump").expect("golden 缺 dump 行"),
            "P533 DRIFT：max_bars={max_bars} lifecycle dump SHA-256 与仓内 golden 不等"
        );
    }
}
