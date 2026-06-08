"""COT 持仓数据层：CFTC Legacy 期货非商业净持仓（Socrata 6dca-aqww）。

口径（与 gamma_delta_full_verification._fetch_cot 一致，单一真相源）：

    net = noncomm_positions_long_all − noncomm_positions_short_all
        = 非商业（投机）净持仓 = 用户要求的 "speculative net position"。

CFTC Legacy "Non-Commercial" 分类是覆盖全品种的统一口径。本体论文档
`docs/architecture/capital_flow_ontology.md` §2.1 给出更精细的映射（GC/CL 用
Disaggregated 的 Managed Money，6E/ES 用 TFF 的 Leveraged Funds）。本模块主用
Legacy Non-Commercial（用户明确指定 + 四标的口径统一便于跨品种比较 + 与既有
_cot_{gc,cl,es}.json 缓存口径一致）。精细口径作为稳健性备选（报告中标注为边界条件）。

会计恒等式（L0，必然真）：期货零和 → 所有类别净头寸之和 ≡ 0。故 net 是相对量
（投机方向），不是绝对资金注入量。

数据频率：周度，report_date 为周二 as-of 快照，周五发布（→ 回测须 ≥1 周滞后）。
缓存：analysis/data_cache/_cot_{sym}.json，list[[date, net]] 升序。
"""

from __future__ import annotations

import json
import urllib.parse
import urllib.request
from pathlib import Path

_DATA = Path(__file__).resolve().parent / "data_cache"
_ENDPOINT = "https://publicreporting.cftc.gov/resource/6dca-aqww.json"

# Legacy futures-only market_and_exchange_names 精确全名（= 匹配，非 like 通配）。
# 用精确全名避免通配混入交叉盘/多交易所同标的（否则同周多条 → 序列污染）。
#
# CL 选 ICE 而非 NYMEX 的理由（严格诚实标注）：CFTC 在 2022-02-01 后停更
# NYMEX 'CRUDE OIL, LIGHT SWEET - NEW YORK MERCANTILE EXCHANGE'（仅 322 周，
# 止于 2022-02），WTI Legacy 报告迁移到 'ICE FUTURES EUROPE' 名下（548 周，全覆盖
# 2015-12 → 至今）。两个 regime 检验点（2022-09、2024-09）都在 NYMEX 停更之后，
# 故 CL 必须用 ICE WTI。ICE WTI 与 NYMEX WTI 同为 WTI 标的，投机方向高度一致，
# 但非同一池——作为边界条件标注（见报告）。
_MARKET_NAMES: dict[str, str] = {
    "6e": "EURO FX - CHICAGO MERCANTILE EXCHANGE",
    "gc": "GOLD - COMMODITY EXCHANGE INC.",
    "cl": "CRUDE OIL, LIGHT SWEET-WTI - ICE FUTURES EUROPE",
    "es": "E-MINI S&P 500 - CHICAGO MERCANTILE EXCHANGE",
}

_SINCE = "2015-12-01"


def fetch_cot(sym: str, force: bool = False) -> list[tuple[str, float]]:
    """拉取/读取单标的 COT 非商业净持仓 → [(date, net)] 升序。

    sym ∈ {6e, gc, cl, es}。优先读缓存（除非 force）。
    """
    if sym not in _MARKET_NAMES:
        raise ValueError(f"未知 COT 标的 {sym!r}，支持 {list(_MARKET_NAMES)}")
    cache = _DATA / f"_cot_{sym}.json"
    if cache.exists() and not force:
        return [(d, v) for d, v in json.load(open(cache))]

    where = (f"market_and_exchange_names = '{_MARKET_NAMES[sym]}' "
             f"AND report_date_as_yyyy_mm_dd > '{_SINCE}'")
    params = urllib.parse.urlencode({
        "$select": "report_date_as_yyyy_mm_dd,noncomm_positions_long_all,"
                   "noncomm_positions_short_all",
        "$where": where,
        "$order": "report_date_as_yyyy_mm_dd",
        "$limit": "5000",
    })
    url = f"{_ENDPOINT}?{params}"
    req = urllib.request.Request(url, headers={"User-Agent": "research"})
    rows = json.load(urllib.request.urlopen(req, timeout=90))
    out: list[tuple[str, float]] = []
    for r in rows:
        d = r["report_date_as_yyyy_mm_dd"][:10]
        lo = float(r.get("noncomm_positions_long_all", 0) or 0)
        sh = float(r.get("noncomm_positions_short_all", 0) or 0)
        out.append((d, lo - sh))
    if not out:
        raise RuntimeError(f"COT 拉取为空：{sym} ({_MARKET_NAMES[sym]})")
    json.dump(out, open(cache, "w"))
    return out


def fetch_all(force: bool = False) -> dict[str, list[tuple[str, float]]]:
    """四标的 COT 全量。"""
    return {sym: fetch_cot(sym, force=force) for sym in _MARKET_NAMES}


if __name__ == "__main__":
    for s, series in fetch_all().items():
        print(f"_cot_{s}: n={len(series):4d}  "
              f"{series[0][0]} → {series[-1][0]}  last_net={series[-1][1]:,.0f}")
