"""SIP consolidated tick 去重/条件码过滤（issue #1041，ADR 0023 §四-1 草案实现）。

本模块是 #1041 的「修正规则」可执行草案——把 Massive 逐笔成交（字段 schema 与 Polygon.io
一致）的原样数据收敛成 consolidated 落盘数据。三条判据各自单一、依次流水（#799 不同判断，
非宽严两档），任何一条都不存在「先 A 后 B 兜底」：

1. **条件码/修正过滤**（filter）：丢修正/撤单/非标准成交（条件码黑名单 + correction
   字段黑名单）；
2. **主所判定**（venue）：丢 TRF/暗池打印（`trf_id` 非空）；
3. **同笔多报去重**（dedup）：按消歧键分组，每组留一行（键是唯一参数，**不是**候选键
   逐个兜底——候选键的选择由 `scripts/probe_sip_dedup.py` 实测，本模块只接受一个键）。

## 权威源（草案阶段逐字核到，见 analysis/sip_consolidated_dedup_rule_draft.md）

- schema 13+1 字段：docs.massive.com `/docs/flat-files/stocks/trades.md` +
  `/docs/rest/stocks/trades-quotes/trades.md`；
- 主所/TRF 语义：`/docs/rest/stocks/market-operations/exchanges.md`（`type ∈
  {exchange, TRF, SIP}`）+ Massive 知识库 FAQ「exchange:4 且带 trf_id = 暗池」；
- 条件码与 correction 编码：massive.com `/glossary/us/stocks/conditions-indicators`
  （Trade Conditions 表 + Trade Corrections 表）。

## 诚实声明（090）

- **条件码黑名单与 correction 黑名单是文档语义草案**，未经实测数据对拍；数据落地
  （#1040）后须用 `probe_sip_dedup.py` 统计各条件码占比 + `correction` 取值分布，
  再据 `/v3/reference/conditions` 的 `update_rules`/`type` 机械复核名单；
- **消歧键（item 3）未定稿**：本模块把键当参数，不内置选择。`(sip_timestamp,
  sequence_number)` 与 `participant_timestamp` 谁对由实测定（#1041 三件之一）。
"""

from __future__ import annotations

import math
from collections.abc import Iterable, Sequence
from numbers import Integral, Real

import pandas as pd

# ---------------------------------------------------------------------------
# 口径常量（单一判据的黑名单/白名单）
# ---------------------------------------------------------------------------

# correction 字段编码（Massive glossary「Trade Corrections」表，逐字核到）：
#   0/00 原始成交（未修正/撤单/错误）       → 保留
#   1/01 原始成交（迟修正，含修正后数据）   → 保留
#   7/07 原始成交（后被标为错误）           → 丢
#   8/08 原始成交（后被撤单）               → 丢
#   10   撤单记录（跟在 08 之后）           → 丢
#   11   错误记录（跟在 07 之后）           → 丢
#   12   修正记录（跟在 01 之后，最终修正） → 保留
#
# 黑名单只列「撤单/错误」四档；原始与最终修正都保留（修正去重由消歧键层处理，见模块头）。
DROP_CORRECTIONS: frozenset[int] = frozenset({7, 8, 10, 11})

# 条件码黑名单（Massive glossary「Trade Conditions」表逐字核到的「非标准成交」——
# 修正/撤单/错误/合成价/均价/衍生定价/非合格带，不构成「单一真实成交价」的那类）。
# 逐条出处见 analysis/sip_consolidated_dedup_rule_draft.md 附录。
DROP_CONDITIONS: frozenset[int] = frozenset(
    {
        2,    # Average Price Trade（均价，非单笔成交价）
        10,   # Derivatively Priced（衍生定价，非报价驱动）
        15,   # Market Center Official Close（官方收盘合成值）
        16,   # Market Center Official Open（官方开盘合成值）
        22,   # Prior Reference Price（>90s 前的参考价，执行时间≠报单时间）
        38,   # Corrected Consolidated Close（收盘修正，非成交）
        39,   # Unknown（未知）
        42,   # NonEligible（非合格，不入 consolidated 带）
        43,   # NonEligible Extended（延时段非合格）
        44,   # Cancelled（撤单）
        45,   # Recovery（恢复）
        46,   # Correction（修正）
        48,   # As of Correction（截止修正）
        49,   # As of Cancel（截止撤单）
        50,   # OOB（越界）
        51,   # Summary（汇总）
        54,   # Errored（错误）
        56,   # Placeholder（占位，TBD）
        59,   # Placeholder for 611 exempt（占位，TBD）
    }
)

# 候选消歧键（item 3，实测定稿前仅作参数候选，不参与判据）：
#   ("sip_timestamp", "sequence_number") —— SIP 收报时间(ns) + 序列号(逐票唯一/日)
#   ("participant_timestamp",)          —— 交易所生成时间(ns)
KEY_SIP_SEQ: tuple[str, ...] = ("sip_timestamp", "sequence_number")
KEY_PARTICIPANT: tuple[str, ...] = ("participant_timestamp",)

# ---------------------------------------------------------------------------
# 纯函数
# ---------------------------------------------------------------------------


def normalize_conditions(value) -> frozenset[int]:
    """把 `conditions` 列的单值归一为 int frozenset。

    兼容 Massive REST（array[int]）、flat-files CSV（"12,37" 引号字符串）、单整数、
    缺失（None/NaN/空）四种形态，以及 parquet 读回后 numpy 标量（np.int64/np.float64，
    后者 NaN 视为空）。非空输入里的非整数元素照实抛 ValueError（fail-loud，不静默吞）。
    """
    if value is None:
        return frozenset()
    # bool 必须先于 Integral 判（bool 是 Integral 的子类）。
    if isinstance(value, bool):
        return frozenset({int(value)})
    if isinstance(value, Integral):
        return frozenset({int(value)})
    if isinstance(value, Real):
        if math.isnan(float(value)):
            return frozenset()
        return frozenset({int(value)})
    if isinstance(value, str):
        s = value.strip()
        if s == "":
            return frozenset()
        parts = [p.strip() for p in s.split(",")]
        return frozenset(int(p) for p in parts if p != "")
    if isinstance(value, Iterable):
        out = set()
        for v in value:
            out.update(normalize_conditions(v))
        return frozenset(out)
    raise ValueError(f"无法归一化的 conditions 值: {value!r}")


def conditions_series(df: pd.DataFrame) -> pd.Series:
    """返回 `conditions` 列的归一化 frozenset Series（列缺失视为全空，不炸）。"""
    if "conditions" not in df.columns:
        return pd.Series([frozenset()] * len(df), index=df.index, dtype=object)
    return df["conditions"].map(normalize_conditions)


def _is_nonnull(col: pd.Series) -> pd.Series:
    """trf_id 这类可空整数字段的非空判定：NaN/None 视为空，>0 视为非空。"""
    return col.notna() & (col != 0)


def filter_trades(df: pd.DataFrame) -> tuple[pd.DataFrame, dict]:
    """判据一：条件码/修正过滤（单一黑名单，无兜底）。

    丢两类行：
    - `correction` 取值在黑名单 DROP_CORRECTIONS 内（撤单/错误）；
    - `conditions` 与黑名单 DROP_CONDITIONS 有交集（修正/撤单/合成价/非合格）。

    返回 (过滤后 df, stats)。不删不改字段，纯行过滤。
    """
    n_in = len(df)
    mask = pd.Series(True, index=df.index)

    if "correction" in df.columns:
        corr = pd.to_numeric(df["correction"], errors="coerce")
        mask &= ~corr.isin(DROP_CORRECTIONS)

    conds = conditions_series(df)
    bad_cond = conds.map(lambda s: bool(s & DROP_CONDITIONS))
    mask &= ~bad_cond

    out = df.loc[mask].copy()
    stats = {
        "filter_in": n_in,
        "filter_out": len(out),
        "dropped_correction": int(corr[~mask].isin(DROP_CORRECTIONS).sum())
        if "correction" in df.columns
        else 0,
        "dropped_condition": int(bad_cond.sum()),
    }
    return out, stats


def drop_trf_prints(df: pd.DataFrame) -> tuple[pd.DataFrame, dict]:
    """判据二：主所判定——丢 TRF/暗池打印（单一判据：`trf_id` 非空）。

    Massive KB FAQ：exchange:4 且带 `trf_id` 的成交来自暗池；TRF 打印是场外成交经
    贸易报告设施（TRF）上报的那份，与主所打印构成「同笔多报」。consolidated 保留主所
    打印（trf_id 为空），丢弃 TRF 打印。列缺失时按「无 TRF 打印」处理（零丢）。
    """
    n_in = len(df)
    if "trf_id" not in df.columns:
        return df.copy(), {"venue_in": n_in, "venue_out": n_in, "dropped_trf": 0}
    trf = _is_nonnull(pd.to_numeric(df["trf_id"], errors="coerce"))
    out = df.loc[~trf].copy()
    return out, {
        "venue_in": n_in,
        "venue_out": len(out),
        "dropped_trf": int(trf.sum()),
    }


def dedup_by_key(df: pd.DataFrame, key: Sequence[str]) -> tuple[pd.DataFrame, dict]:
    """判据三：同笔多报去重（单一键分组，每组保留一行，无候选键兜底）。

    `key` 是列名序列（例如 KEY_SIP_SEQ 或 KEY_PARTICIPANT）。分组后按 `(key..., 原始
    顺序)` 稳定排序取每组首行。键列缺失时报错（fail-loud：键是判据的承重件，缺失不可
    静默跳过）。
    """
    for col in key:
        if col not in df.columns:
            raise ValueError(f"消歧键列缺失: {col!r}")
    n_in = len(df)
    # 稳定排序保证「保留哪一行」确定：先按键升序，同键内按出现顺序。
    tmp = df.assign(__order__=range(n_in))
    tmp = tmp.sort_values(list(key) + ["__order__"], kind="stable")
    out = tmp.drop_duplicates(subset=list(key), keep="first")
    out = out.drop(columns=["__order__"]).sort_index()
    return out, {
        "dedup_in": n_in,
        "dedup_out": len(out),
        "dedup_dropped": n_in - len(out),
    }


def consolidate(
    df: pd.DataFrame,
    key: Sequence[str] = KEY_SIP_SEQ,
    *,
    drop_trf: bool = True,
) -> tuple[pd.DataFrame, dict]:
    """三判据流水：条件/修正过滤 → 主所判定 → 同笔多报去重。

    返回 (consolidated df, stats)。stats 含每层进出计数与去重率读数：
    `duplicate_rate = 1 - len(out) / len(in)`（与 #1041 交付「consolidated vs 原样，
    行数差 = 重复率读数」同口径）。
    """
    n_in = len(df)
    f, s1 = filter_trades(df)
    if drop_trf:
        v, s2 = drop_trf_prints(f)
    else:
        v, s2 = f, {"venue_in": len(f), "venue_out": len(f), "dropped_trf": 0}
    out, s3 = dedup_by_key(v, key)
    stats = {**s1, **s2, **s3, "consolidate_in": n_in, "consolidate_out": len(out)}
    stats["duplicate_rate"] = 1.0 - (len(out) / n_in) if n_in else 0.0
    return out, stats
