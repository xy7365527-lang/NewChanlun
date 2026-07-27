#!/usr/bin/env python3
"""费率科目分解（treasury 重验 T2，issue #388）——臂D（venue 标定）vs 臂R（未标定常率）。

**性质：离线独立重算，不是账本实扣分项。**

为什么必须离线重算：生产侧的 R 分解（`strategy::risk::RDecomposition`）只有一个**合并**科目
`commission_slippage`，不逐科目拆分；`trades.jsonl` 是**费前**口径（`opsem_dump.rs:233-234` 明文）
且不含任何费用字段。因此本脚本从 `trades.jsonl` 的 `units` / `entry_px` / `exit_px` 出发，用两臂
各自的费率口径**逐笔重算**费用并按科目累计。

**有效域（231号）**：重算基 = **已平仓声部的双腿名义额**。账本的 `Comm+Slip` 列口径更宽——它含
全部订单流（减仓腿、强平腿、窗末未平仓腿、被拒单前的试算不含），故本表的 Σ 与四层报告的
`Comm+Slip` 列**不应相等**（两者的差随验收报告一并登记）。本表回答的是
「同一批已平仓声部，两臂的费用差落在哪个科目」，不是「账本一共扣了多少」。

**费率口径来源（路径即契约）**：

- 臂D：`analysis/data_cache/venue_fee_binance_spot_20260726.json` 的 (BTC, VIP0) 条目
  （taker_bps；sha256 sidecar 已由 Rust 侧 `load_datum` 校验）+ 未标定滑点 addon
  `ExecConfig::slippage_bps`（datum 不覆盖滑点，`venue_fee.rs` FeeQuoter 明文相加）。
  Binance 现货 datum **无监管/清算科目** ⟹ 该两列恒 0（不是「没算」，是该 venue 无此科目）。
- 臂R：`ExecConfig::default()` 三常数（`rust/src/theta_v0/config.rs`，`impl Default for ExecConfig`
  内，当前 :318-320：
  commission 1bp / slippage 2bp / tax 0bp），无监管/清算科目。
  **这三个数在本脚本内是常量（手抄），不是从文件读的**——三常数档按定义**没有 datum**
  （未标定就是它的性质），无文件可读。Rust 侧改默认值而未同步本脚本 ⟹ 本表会静默漂移；
  防线 = 表头随每次运行打印这三个数与其来源行号，复核时人工对一眼。
  臂D 侧无此问题（费率从 datum 文件读，不手抄）。

**监管/清算两列恒 0 的适用范围**：仅对本表覆盖的两个档位（Binance 现货 datum、三常数档）成立，
两者都无这两个科目。换成含该科目的 datum（如 IBKR per-share）本脚本**不适用**——它只实现
per-notional 形态的分解，per-share 形态的逐笔分解见 `fill.rs::oklo_real_window_per_share_readings`。

用法：

    python3 scripts/fee_account_decomposition.py                      # 默认目录，打 markdown 表
    python3 scripts/fee_account_decomposition.py --out /tmp/tbl.md

退出码：0=成功；4=产物缺失（跑批未做）。
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import pathlib
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
DATUM_PATH = REPO_ROOT / "analysis" / "data_cache" / "venue_fee_binance_spot_20260726.json"
WINDOWS = ("p3fold", "wf7", "wf8")

# 臂R 三常数（rust/src/theta_v0/config.rs 的 impl Default for ExecConfig，当前 :318-320）。
ARM_R_COMMISSION_BPS = 1.0
ARM_R_SLIPPAGE_BPS = 2.0
ARM_R_TAX_BPS = 0.0
# 未标定滑点 addon（datum 不覆盖，两臂同值；venue_fee.rs FeeQuoter::new 的 uncalibrated_addon）。
SLIPPAGE_BPS = ARM_R_SLIPPAGE_BPS
# 标定档（臂D、臂C 共用——两者都吃 datum `fee_schedule`）的 tax：`config.rs:271`（`fee_schedule`
# 字段文档）声明「`fee_schedule = Some(...)` 时要求 `tax_bps == 0`」，机器强制在
# `treasury.rs` 的 `fn fee_quoter`（当前 :124 起）内的 `assert!`（当前 :128；禁与 datum 内的监管税费重复计）。
# 这个 0 的依据是「标定档」这个性质本身，不是「臂D」这个臂——臂C 同样标定，故共用同一常量。
ARM_CALIBRATED_TAX_BPS = 0.0


def binance_taker_bps() -> float:
    """从仓内 datum 读 (BTC, VIP0) 的 taker bps——不手抄费率数字。"""
    book = json.loads(DATUM_PATH.read_text())
    for e in book["entries"]:
        if e["symbol"] == "BTC" and e["tier"] == "VIP0":
            return float(e["taker_bps"])
    raise SystemExit(f"datum {DATUM_PATH} 无 (BTC, VIP0) 条目")


def leg_notionals(trades_path: pathlib.Path) -> list[float]:
    """每笔 trade 的两条腿名义额（开仓腿 + 平仓腿）。"""
    out: list[float] = []
    with trades_path.open() as fh:
        for line in fh:
            if not line.strip():
                continue
            t = json.loads(line)
            units = abs(float(t["units"]))
            out.append(units * float(t["entry_px"]))
            out.append(units * float(t["exit_px"]))
    return out


@dataclasses.dataclass(frozen=True)
class Decomposition:
    """一个 (窗, 臂) 的费率科目分解。金额单位与 `trades.jsonl` 的价格单位一致。"""

    n_legs: int
    notional: float
    commission: float
    regulatory: float
    clearing: float
    slippage: float
    tax: float

    @property
    def total(self) -> float:
        return (
            self.commission
            + self.regulatory
            + self.clearing
            + self.slippage
            + self.tax
        )


def decompose(
    notionals: list[float], commission_bps: float, tax_bps: float
) -> Decomposition:
    """科目分解：commission / 监管 / 清算 / slippage / tax（本表覆盖的两个档位均无监管/清算科目；
    tax 在两个档位下均恒 0——臂R 是 `ExecConfig::default()` 常量，臂D/C 是 datum 标定档下
    `config.rs:271` 声明、`treasury.rs` 的 `fn fee_quoter`（当前 :124 起）内 `assert!`（当前 :128）
    机器强制的约束的推论，非「没算」）。"""
    total = sum(notionals)
    return Decomposition(
        n_legs=len(notionals),
        notional=total,
        commission=total * commission_bps / 10_000.0,
        regulatory=0.0,
        clearing=0.0,
        slippage=total * SLIPPAGE_BPS / 10_000.0,
        tax=total * tax_bps / 10_000.0,
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--armD-dir", default="/tmp/m8_win_armD")
    ap.add_argument("--armR-dir", default="/tmp/m8_win_gate")
    # ★#389 T3（帽臂 C）：与臂D **同一 datum 同一档位**（唯一差异 = `enforce_level_cap`），
    #   故 commission_bps 取同一个 `taker_bps`——本表的臂C 行只反映**名义额（规模/形态）差**，
    #   费率口径与臂D 逐位相同。产物缺失时本行**跳过而非报错**（帽臂是可选跑批，T2 时不存在）。
    ap.add_argument("--armC-dir", default="/tmp/m8_win_armC")
    ap.add_argument("--out", default=None)
    args = ap.parse_args()

    taker_bps = binance_taker_bps()
    rows: list[str] = []
    missing: list[str] = []
    optional_absent: list[str] = []
    for tag in WINDOWS:
        for arm, root, comm_bps, tax_bps, required in (
            ("D(标定)", pathlib.Path(args.armD_dir), taker_bps, ARM_CALIBRATED_TAX_BPS, True),
            ("C(标定+帽)", pathlib.Path(args.armC_dir), taker_bps, ARM_CALIBRATED_TAX_BPS, False),
            ("R(未标定)", pathlib.Path(args.armR_dir), ARM_R_COMMISSION_BPS, ARM_R_TAX_BPS, True),
        ):
            path = root / tag / "trades.jsonl"
            if not path.exists():
                if required:
                    missing.append(str(path))
                else:
                    # 可选臂缺产物：**不静默**。表里少一行与「跑了但为空」在下游读起来一样，
                    # 必须在 stderr 说出来（同票 `M8_LEVEL_CAP` 非法值 fail-loud 的同一理由）。
                    optional_absent.append(str(path))
                continue
            d = decompose(leg_notionals(path), comm_bps, tax_bps)
            rows.append(
                f"| {tag} | {arm} | {d.n_legs} | {d.notional:.2f} | "
                f"{d.commission:.2f} | {d.regulatory:.2f} | {d.clearing:.2f} | "
                f"{d.slippage:.2f} | {d.tax:.2f} | {d.total:.2f} |"
            )
    if missing:
        print("产物缺失（跑批未做）：\n  " + "\n  ".join(missing), file=sys.stderr)
        return 4
    if optional_absent:
        print(
            "可选臂产物缺失，对应行**未出现在表中**（不是「跑了但为空」）：\n  "
            + "\n  ".join(optional_absent),
            file=sys.stderr,
        )

    md = "\n".join(
        [
            f"费率口径：臂D commission={taker_bps:.1f}bp（datum {DATUM_PATH.name} / BTC / VIP0 taker）；"
            f"臂R commission={ARM_R_COMMISSION_BPS:.1f}bp、tax={ARM_R_TAX_BPS:.1f}bp"
            f"（config.rs impl Default for ExecConfig，当前:318-320）；"
            f"臂C commission 与臂D 同源（同 datum 同档位，唯一差异 = `enforce_level_cap`）；"
            f"三臂 slippage={SLIPPAGE_BPS:.1f}bp（datum 不覆盖，未标定 addon）；"
            f"臂D/C tax={ARM_CALIBRATED_TAX_BPS:.1f}bp（config.rs:271 声明、treasury.rs fn fee_quoter"
            f"[当前:124起]内 assert![当前:128] 强制的约束下的推论，非手抄）、"
            f"臂R tax={ARM_R_TAX_BPS:.1f}bp（config.rs impl Default for ExecConfig，当前:318-320）。",
            "",
            "| 窗 | 臂 | 成交腿数 | Σ名义额 | commission | 监管 | 清算 | slippage | tax | Σ费用 |",
            "|---|---|---|---|---|---|---|---|---|---|",
            *rows,
            "",
            "**口径**：基于 `trades.jsonl` 的**独立重算**（已平仓声部双腿），"
            "**非账本实扣分项**——账本 `Comm+Slip` 列含全部订单流（减仓/强平/窗末未平腿），口径更宽。",
            "**监管/清算恒 0**：Binance 现货与三常数档均无此科目（不是漏算）。",
            "**tax 恒 0**：臂R 是 `ExecConfig::default()` 常量；臂D/C 是标定档下 "
            "`config.rs:271` 声明、`treasury.rs` 的 `fn fee_quoter`（当前 :124 起）内 `assert!`"
            "（当前 :128；`fee_schedule = Some(...)` 时禁 `tax_bps != 0`）机器强制的约束"
            "的推论——若上游改变该约束或该常量非零，本表逐笔重算会自动纳入（一等科目，非特判）。",
        ]
    )
    if args.out:
        pathlib.Path(args.out).write_text(md + "\n")
    print(md)
    return 0


if __name__ == "__main__":
    sys.exit(main())
