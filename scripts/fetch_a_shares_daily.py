"""批量获取 A 股日线数据（baostock），用于 T(S) L2 验证样本增强。

数据源：baostock（免费，无需 API key，使用自有协议）。
输出：data/scanner_l2/{code}_daily.json，格式与现有文件一致。

认识论等级：数据准备（为 L2 验证提供真实数据）。
谱系引用：350号、366号。
"""

from __future__ import annotations

import json
import logging
import sys
import time
from pathlib import Path

import baostock as bs
import pandas as pd

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
logger = logging.getLogger("fetch_a_shares_daily")

OUTPUT_DIR = Path(__file__).resolve().parent.parent / "data" / "scanner_l2"

# A 股各行业龙头/大盘股，覆盖 10+ 行业，100+ 标的
# 格式：{六位代码: {"name": 名称, "sector": 行业}}
SYMBOLS: dict[str, dict] = {
    # ── 黄金/有色 ──
    "600547": {"name": "山东黄金", "sector": "au"},
    "002155": {"name": "湖南黄金", "sector": "au"},
    "600489": {"name": "中金黄金", "sector": "au"},
    "601899": {"name": "紫金矿业", "sector": "au"},
    "600362": {"name": "江西铜业", "sector": "metal"},
    "601600": {"name": "中国铝业", "sector": "metal"},
    "603993": {"name": "洛阳钼业", "sector": "metal"},
    "000630": {"name": "铜陵有色", "sector": "metal"},
    # ── 石油/能源 ──
    "600028": {"name": "中国石化", "sector": "oil"},
    "601857": {"name": "中国石油", "sector": "oil"},
    "600688": {"name": "上海石化", "sector": "oil"},
    "601985": {"name": "中国核电", "sector": "energy"},
    "600900": {"name": "长江电力", "sector": "energy"},
    "600886": {"name": "国投电力", "sector": "energy"},
    "600025": {"name": "华能水电", "sector": "energy"},
    "003816": {"name": "中国广核", "sector": "energy"},
    # ── 银行 ──
    "601398": {"name": "工商银行", "sector": "bank"},
    "601288": {"name": "农业银行", "sector": "bank"},
    "601988": {"name": "中国银行", "sector": "bank"},
    "601939": {"name": "建设银行", "sector": "bank"},
    "600036": {"name": "招商银行", "sector": "bank"},
    "601166": {"name": "兴业银行", "sector": "bank"},
    "600016": {"name": "民生银行", "sector": "bank"},
    "601328": {"name": "交通银行", "sector": "bank"},
    "000001": {"name": "平安银行", "sector": "bank"},
    "600000": {"name": "浦发银行", "sector": "bank"},
    "002142": {"name": "宁波银行", "sector": "bank"},
    # ── 保险/券商 ──
    "601318": {"name": "中国平安", "sector": "insurance"},
    "601628": {"name": "中国人寿", "sector": "insurance"},
    "601601": {"name": "中国太保", "sector": "insurance"},
    "600030": {"name": "中信证券", "sector": "broker"},
    "601211": {"name": "国泰君安", "sector": "broker"},
    "600837": {"name": "海通证券", "sector": "broker"},
    "000776": {"name": "广发证券", "sector": "broker"},
    "601688": {"name": "华泰证券", "sector": "broker"},
    # ── 房地产 ──
    "600048": {"name": "保利发展", "sector": "re"},
    "000002": {"name": "万科A", "sector": "re"},
    "001979": {"name": "招商蛇口", "sector": "re"},
    "600606": {"name": "绿地控股", "sector": "re"},
    "002146": {"name": "荣盛发展", "sector": "re"},
    # ── 科技/电子 ──
    "002230": {"name": "科大讯飞", "sector": "tech"},
    "300059": {"name": "东方财富", "sector": "tech"},
    "000725": {"name": "京东方A", "sector": "tech"},
    "600588": {"name": "用友网络", "sector": "tech"},
    "002415": {"name": "海康威视", "sector": "tech"},
    "000063": {"name": "中兴通讯", "sector": "tech"},
    "603986": {"name": "兆易创新", "sector": "tech"},
    "002049": {"name": "紫光国微", "sector": "tech"},
    "688981": {"name": "中芯国际", "sector": "tech"},
    "300750": {"name": "宁德时代", "sector": "tech"},
    "002475": {"name": "立讯精密", "sector": "tech"},
    "300015": {"name": "爱尔眼科", "sector": "tech"},
    # ── 消费/白酒 ──
    "600519": {"name": "贵州茅台", "sector": "consumer"},
    "000858": {"name": "五粮液", "sector": "consumer"},
    "002304": {"name": "洋河股份", "sector": "consumer"},
    "000568": {"name": "泸州老窖", "sector": "consumer"},
    "600809": {"name": "山西汾酒", "sector": "consumer"},
    "603369": {"name": "今世缘", "sector": "consumer"},
    "000596": {"name": "古井贡酒", "sector": "consumer"},
    # ── 食品饮料 ──
    "600887": {"name": "伊利股份", "sector": "food"},
    "000895": {"name": "双汇发展", "sector": "food"},
    "603288": {"name": "海天味业", "sector": "food"},
    "002714": {"name": "牧原股份", "sector": "food"},
    "300498": {"name": "温氏股份", "sector": "food"},
    # ── 家电 ──
    "000651": {"name": "格力电器", "sector": "appliance"},
    "000333": {"name": "美的集团", "sector": "appliance"},
    "600690": {"name": "海尔智家", "sector": "appliance"},
    # ── 汽车 ──
    "600104": {"name": "上汽集团", "sector": "auto"},
    "002594": {"name": "比亚迪", "sector": "auto"},
    "601238": {"name": "广汽集团", "sector": "auto"},
    "000625": {"name": "长安汽车", "sector": "auto"},
    "601127": {"name": "赛力斯", "sector": "auto"},
    # ── 医药 ──
    "300760": {"name": "迈瑞医疗", "sector": "pharma"},
    "600276": {"name": "恒瑞医药", "sector": "pharma"},
    "000538": {"name": "云南白药", "sector": "pharma"},
    "600196": {"name": "复星医药", "sector": "pharma"},
    "002007": {"name": "华兰生物", "sector": "pharma"},
    "603259": {"name": "药明康德", "sector": "pharma"},
    # ── 建材/建筑 ──
    "600585": {"name": "海螺水泥", "sector": "material"},
    "002271": {"name": "东方雨虹", "sector": "material"},
    "601668": {"name": "中国建筑", "sector": "infra"},
    "601390": {"name": "中国中铁", "sector": "infra"},
    "601186": {"name": "中国铁建", "sector": "infra"},
    "601800": {"name": "中国交建", "sector": "infra"},
    # ── 钢铁 ──
    "600019": {"name": "宝钢股份", "sector": "steel"},
    "000709": {"name": "河钢股份", "sector": "steel"},
    "600010": {"name": "包钢股份", "sector": "steel"},
    # ── 化工 ──
    "600309": {"name": "万华化学", "sector": "chemical"},
    "002601": {"name": "龙蟒佰利", "sector": "chemical"},
    "600352": {"name": "浙江龙盛", "sector": "chemical"},
    # ── 交通运输 ──
    "601006": {"name": "大秦铁路", "sector": "transport"},
    "600029": {"name": "南方航空", "sector": "transport"},
    "601111": {"name": "中国国航", "sector": "transport"},
    "600115": {"name": "中国东航", "sector": "transport"},
    "601021": {"name": "春秋航空", "sector": "transport"},
    # ── 通信 ──
    "600941": {"name": "中国移动", "sector": "telecom"},
    "601728": {"name": "中国电信", "sector": "telecom"},
    # ── 军工 ──
    "600893": {"name": "航发动力", "sector": "military"},
    "600760": {"name": "中航沈飞", "sector": "military"},
    "601989": {"name": "中国重工", "sector": "military"},
    "000768": {"name": "中航西飞", "sector": "military"},
    # ── 新能源 ──
    "601012": {"name": "隆基绿能", "sector": "solar"},
    "600438": {"name": "通威股份", "sector": "solar"},
    "002459": {"name": "晶澳科技", "sector": "solar"},
    # ── 煤炭 ──
    "601088": {"name": "中国神华", "sector": "coal"},
    "600188": {"name": "兖矿能源", "sector": "coal"},
    "601898": {"name": "中煤能源", "sector": "coal"},
    # ── 地产服务/物业 ──
    "002244": {"name": "滨江集团", "sector": "re"},
    # ── 传媒 ──
    "300413": {"name": "芒果超媒", "sector": "media"},
    "002027": {"name": "分众传媒", "sector": "media"},
    # ── 纺织服装 ──
    "600398": {"name": "海澜之家", "sector": "apparel"},
}


def _code_to_baostock(code: str) -> str:
    """六位代码转 baostock 格式（sh./sz.前缀）。"""
    if code.startswith(("6", "9")):
        return f"sh.{code}"
    return f"sz.{code}"


def fetch_single(code: str, name: str, start: str = "2018-01-01", end: str = "2026-03-05") -> int:
    """获取单个标的日线数据，返回 bar 数量。"""
    out_path = OUTPUT_DIR / f"{code}_daily.json"
    if out_path.exists():
        existing = json.loads(out_path.read_text(encoding="utf-8"))
        if len(existing) >= 1000:
            logger.info("  跳过 %s (%s): 已有 %d bars", code, name, len(existing))
            return len(existing)

    bs_code = _code_to_baostock(code)
    rs = bs.query_history_k_data_plus(
        bs_code,
        "date,open,high,low,close,volume",
        start_date=start,
        end_date=end,
        frequency="d",
        adjustflag="2",  # 前复权
    )

    if rs.error_code != "0":
        logger.warning("  查询失败 %s (%s): %s", code, name, rs.error_msg)
        return 0

    rows = []
    while rs.next():
        row = rs.get_row_data()
        # baostock 有时返回空值
        if not row[1] or not row[4]:
            continue
        rows.append({
            "ts": f"{row[0]}T00:00:00",
            "open": float(row[1]),
            "high": float(row[2]),
            "low": float(row[3]),
            "close": float(row[4]),
            "volume": float(row[5]) if row[5] else 0.0,
        })

    if not rows:
        logger.warning("  无数据 %s (%s)", code, name)
        return 0

    out_path.write_text(json.dumps(rows, ensure_ascii=False), encoding="utf-8")
    return len(rows)


def main() -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    lg = bs.login()
    if lg.error_code != "0":
        logger.error("baostock 登录失败: %s", lg.error_msg)
        sys.exit(1)
    logger.info("baostock 登录成功")

    total = len(SYMBOLS)
    success = 0
    total_bars = 0

    for i, (code, info) in enumerate(SYMBOLS.items(), 1):
        logger.info("[%d/%d] 获取 %s (%s, %s)", i, total, code, info["name"], info["sector"])
        n = fetch_single(code, info["name"])
        if n > 0:
            success += 1
            total_bars += n
            logger.info("  → %d bars", n)
        # 避免请求过快
        time.sleep(0.3)

    bs.logout()
    logger.info("完成: %d/%d 标的成功, 总计 %d bars", success, total, total_bars)


if __name__ == "__main__":
    main()
