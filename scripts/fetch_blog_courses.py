#!/usr/bin/env python3
"""
抓取缠师原始博文108课（+ 序篇）并保存为 Markdown。
来源：https://www.fengmr.com/chanlun.html
"""

import os
import re
import time
import requests
from bs4 import BeautifulSoup

BASE_URL = "https://www.fengmr.com"
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "..", "docs", "chanlun", "text", "blog")

# 完整的课程URL列表（序篇 + 108课）
COURSES = [
    ("000", "序篇", "股市闲谈：G股是G点，大牛不用套！", "/cl108kxp.html"),
    ("001", "第1课", "不会赢钱的经济人，只是废人！", "/cld1kb.html"),
    ("002", "第2课", "没有庄家，有的只是赢家和输家！", "/cld2kc.html"),
    ("003", "第3课", "你的喜好，你的死亡陷阱！", "/cld3kb.html"),
    ("004", "第4课", "什么是理性？今早买N中工就是理性！", "/cld4kc.html"),
    ("005", "第5课", "市场无须分析，只要看和干！", "/cld5kb.html"),
    ("006", "第6课", "本ID如何在五粮液、包钢权证上提款的！", "/cld6kb.html"),
    ("007", "第7课", "给赚了指数亏了钱的一些忠告", "/cld7kb.html"),
    ("008", "第8课", "投资如选面首，G点为中心，拒绝ED男！", "/cld8kb.html"),
    ("009", "第9课", "甄别早泄男的数学原则！", "/cld9kb.html"),
    ("010", "第10课", "2005年6月，本ID为何时隔四年后重看股票", "/cld10k2.html"),
    ("011", "第11课", "不会吻，无以高潮！", "/cld11ka.html"),
    ("012", "第12课", "一吻何能消魂？", "/cld12ka.html"),
    ("013", "第13课", "不带套的操作不是好操作！", "/cld13ka.html"),
    ("014", "第14课", "喝茅台的高潮程序！", "/cld14ka.html"),
    ("015", "第15课", "没有趋势，没有背驰", "/cld15ka.html"),
    ("016", "第16课", "中小资金的高效买卖法", "/cld16ka.html"),
    ("017", "第17课", "走势终完美", "/cld17ka.html"),
    ("018", "第18课", "不被面首的雏男是不完美的", "/cld18ka.html"),
    ("019", "第19课", "学习缠中说禅技术分析理论的关键", "/cld19ka.html"),
    ("020", "第20课", "缠中说禅走势中枢级别扩张及第三类买卖点", "/cld20ka.html"),
    ("021", "第21课", "缠中说禅买卖点分析的完备性", "/cld21ka.html"),
    ("022", "第22课", "将8亿的大米装到5个庄家的肚里", "/cld22ka.html"),
    ("023", "第23课", "市场与人生", "/cld23ka.html"),
    ("024", "第24课", "MACD对背弛的辅助判断", "/cld24km.html"),
    ("025", "第25课", "吻，MACD、背弛、中枢", "/cld25ka.html"),
    ("026", "第26课", "市场风险如何回避", "/cld26ka.html"),
    ("027", "第27课", "盘整背驰与历史性底部", "/cld27k.html"),
    ("028", "第28课", "下一目标：摧毁基金", "/cld28ka.html"),
    ("029", "第29课", "转折的力度与级别", "/cld29ka.html"),
    ("030", "第30课", "缠中说禅理论的绝对性", "/cld30ka.html"),
    ("031", "第31课", "资金管理的最稳固基础", "/cld31ka.html"),
    ("032", "第32课", "走势的当下与投资者的思维方式", "/cld32ka.html"),
    ("033", "第33课", "走势的多义性", "/cld33ka.html"),
    ("034", "第34课", "宁当面首，莫成怨男", "/cld34ka.html"),
    ("035", "第35课", "给基础差的同学补补课", "/cld35ka.html"),
    ("036", "第36课", "走势类型连接结合性的简单运用", "/cld36ka.html"),
    ("037", "第37课", "背驰的再分辨", "/cld37ka.html"),
    ("038", "第38课", "走势类型连接的同级别分解", "/cld38ka.html"),
    ("039", "第39课", "同级别分解再研究", "/cld39ka.html"),
    ("040", "第40课", "同级别分解的多重赋格", "/cld40ka.html"),
    ("041", "第41课", "没有节奏，只有死", "/cld41ka.html"),
    ("042", "第42课", "有些人是不适合参与市场的", "/cld42ka.html"),
    ("043", "第43课", "有关背驰的补习课", "/cld43ka.html"),
    ("044", "第44课", "小级别背驰引发大级别转折", "/cld44k.html"),
    ("045", "第45课", "持股与持币，两种最基本的操作", "/cld45ka.html"),
    ("046", "第46课", "每日走势的分类", "/cld46ka.html"),
    ("047", "第47课", "一夜情行情分析", "/cld47ka.html"),
    ("048", "第48课", "暴跌，牛市行情的一夜情", "/cld48ka.html"),
    ("049", "第49课", "利润率最大的操作模式", "/cld49ka.html"),
    ("050", "第50课", "操作中的一些细节问题", "/cld50ka.html"),
    ("051", "第51课", "短线股评荐股者的传销把戏", "/cld51ka.html"),
    ("052", "第52课", "炒股票就是真正的学佛", "/cld52ka.html"),
    ("053", "第53课", "三类买卖点的再分辨", "/cld53ka.html"),
    ("054", "第54课", "一个具体走势的分析", "/cld54ka.html"),
    ("055", "第55课", "买之前戏，卖之高潮", "/cld55ka.html"),
    ("056", "第56课", "530印花税当日行情图解", "/cld56k5.html"),
    ("057", "第57课", "当下图解分析再示范", "/cld57ka.html"),
    ("058", "第58课", "图解分析示范三", "/cld58ka.html"),
    ("059", "第59课", "图解分析示范四", "/cld59ka.html"),
    ("060", "第60课", "图解分析示范五", "/cld60ka.html"),
    ("061", "第61课", "区间套定位标准图解（分析示范六）", "/cld61ka.html"),
    ("062", "第62课", "分型、笔与线段", "/cld62ka.html"),
    ("063", "第63课", "替各位理理基本概念", "/cld63ka.html"),
    ("064", "第64课", "去机场路上给各位补课", "/cld64ka.html"),
    ("065", "第65课", "再说说分型、笔、线段", "/cld65ka.html"),
    ("066", "第66课", "主力资金的食物链", "/cld66ka.html"),
    ("067", "第67课", "线段的划分标准", "/cld67ka.html"),
    ("068", "第68课", "走势预测的精确意义", "/cld68ka.html"),
    ("069", "第69课", "月线分段与上海大走势分析、预判", "/cld69ka.html"),
    ("070", "第70课", "一个教科书式走势的示范分析", "/cld70ka.html"),
    ("071", "第71课", "线段划分标准的再分辨", "/cld71ka.html"),
    ("072", "第72课", "本ID已有课程的再梳理", "/cld72ka.html"),
    ("073", "第73课", "市场获利机会的绝对分类", "/cld73ka.html"),
    ("074", "第74课", "如何躲避政策性风险", "/cld74ka.html"),
    ("075", "第75课", "逗庄家玩的一些杂史1", "/cld75ka.html"),
    ("076", "第76课", "逗庄家玩的一些杂史2", "/cld76ka.html"),
    ("077", "第77课", "一些概念的再分辨", "/cld77ka.html"),
    ("078", "第78课", "继续说线段的划分", "/cld78ka.html"),
    ("079", "第79课", "分型的辅助操作与一些问题的再解答", "/cld79ka.html"),
    ("080", "第80课", "市场没有同情、不信眼泪", "/cld80ka.html"),
    ("081", "第81课", "图例、更正及分型、走势类型的哲学本质", "/cld81ka.html"),
    ("082", "第82课", "分型结构的心理因素", "/cld82ka.html"),
    ("083", "第83课", "笔-线段与线段-最小中枢结构的不同心理意义1", "/cld83ka.html"),
    ("084", "第84课", "本ID理论一些必须注意的问题", "/cld84ka.html"),
    ("085", "第85课", "逗庄家玩的一些杂史3", "/cld85ka.html"),
    ("086", "第86课", "走势分析中必须杜绝一根筋思维", "/cld86ka.html"),
    ("087", "第87课", "逗庄家玩的一些杂史4", "/cld87ka.html"),
    ("088", "第88课", "图形生长的一个具体案例", "/cld88ka.html"),
    ("089", "第89课", "中阴阶段的具体分析", "/cld89ka.html"),
    ("090", "第90课", "中阴阶段结束时间的辅助判断", "/cld90ka.html"),
    ("091", "第91课", "走势结构的两重表里关系1", "/cld91ka.html"),
    ("092", "第92课", "中枢震荡的监视器", "/cld92ka.html"),
    ("093", "第93课", "走势结构的两重表里关系2", "/cld93ka.html"),
    ("094", "第94课", "当机立断", "/cld94ka.html"),
    ("095", "第95课", "修炼自己", "/cld95ka.html"),
    ("096", "第96课", "无处不在的赌徒心理", "/cld96ka.html"),
    ("097", "第97课", "中医、兵法、诗歌、操作1", "/cld97ka.html"),
    ("098", "第98课", "中医、兵法、诗歌、操作2", "/cld98ka.html"),
    ("099", "第99课", "走势结构的两重表里关系3", "/cld99ka.html"),
    ("100", "第100课", "中医、兵法、诗歌、操作3", "/cld100k.html"),
    ("101", "第101课", "答疑1", "/cld101k.html"),
    ("102", "第102课", "再说走势必完美", "/cld102k.html"),
    ("103", "第103课", "学屠龙术前先学好防狼术", "/cld103k.html"),
    ("104", "第104课", "几何结构与能量动力结构1", "/cld104k.html"),
    ("105", "第105课", "远离聪明、机械操作", "/cld105k.html"),
    ("106", "第106课", "均线、轮动与缠中说禅板块强弱指标", "/cld106k.html"),
    ("107", "第107课", "如何操作短线反弹", "/cld107k.html"),
    ("108", "第108课", "何谓底部？从月线看中期走势演化", "/cld108k.html"),
]

HEADERS = {
    "User-Agent": "Mozilla/5.0 (compatible; NewChanlun/1.0; research)",
    "Accept": "text/html,application/xhtml+xml",
    "Accept-Language": "zh-CN,zh;q=0.9",
}


def fetch_page(url: str) -> str:
    """Fetch a page with retries."""
    for attempt in range(3):
        try:
            resp = requests.get(url, headers=HEADERS, timeout=30)
            resp.encoding = "utf-8"
            return resp.text
        except Exception as e:
            if attempt < 2:
                time.sleep(2 ** attempt)
            else:
                raise


def extract_article(html: str) -> dict:
    """Extract article content from fengmr.com page."""
    soup = BeautifulSoup(html, "html.parser")

    result = {
        "title": "",
        "date": "",
        "main_text": "",
        "qa_text": "",
        "images": [],
    }

    # Try to find the article title
    title_tag = soup.find("h1")
    if title_tag:
        result["title"] = title_tag.get_text(strip=True)

    # Try to find publish date
    date_tag = soup.find("time") or soup.find(class_=re.compile(r"date|time|publish"))
    if date_tag:
        result["date"] = date_tag.get_text(strip=True)

    # Find the main article content
    # fengmr.com typically uses <article> or a main content div
    article = soup.find("article")
    if not article:
        # Try common content containers
        for selector in ["div.article-content", "div.post-content", "div.entry-content",
                         "div.content", "div.post-body", "main"]:
            article = soup.select_one(selector)
            if article:
                break

    if not article:
        # Fallback: find the largest text block
        article = soup.find("body")

    if article:
        # Convert to markdown-like text
        text_parts = []
        for elem in article.descendants:
            if elem.name == "img":
                src = elem.get("src", "")
                alt = elem.get("alt", "")
                if src:
                    if not src.startswith("http"):
                        src = BASE_URL + "/" + src.lstrip("/")
                    text_parts.append(f"\n![{alt}]({src})\n")
                    result["images"].append(src)
            elif elem.name in ("h1", "h2", "h3", "h4"):
                level = int(elem.name[1])
                text = elem.get_text(strip=True)
                if text:
                    text_parts.append(f"\n{'#' * level} {text}\n")
            elif elem.name == "p":
                text = elem.get_text(strip=True)
                if text:
                    text_parts.append(f"\n{text}\n")
            elif elem.name == "br":
                text_parts.append("\n")
            elif elem.name == "blockquote":
                text = elem.get_text(strip=True)
                if text:
                    text_parts.append(f"\n> {text}\n")
            elif elem.name in ("strong", "b"):
                text = elem.get_text(strip=True)
                if text and not any(text in p for p in text_parts[-3:] if isinstance(p, str)):
                    text_parts.append(f"**{text}**")

        raw_text = "".join(text_parts)
        # Clean up excessive newlines
        raw_text = re.sub(r"\n{3,}", "\n\n", raw_text)
        result["main_text"] = raw_text.strip()

    return result


def save_course(num: str, label: str, title: str, content: dict, url: str) -> str:
    """Save a course as a markdown file."""
    filename = f"{num}-{label}.md"
    filepath = os.path.join(OUTPUT_DIR, filename)

    md_lines = [
        f"# {label}：{title}",
        "",
        f"> 来源：{BASE_URL}{url.replace(BASE_URL, '')}",
    ]
    if content["date"]:
        md_lines.append(f"> 发表日期：{content['date']}")
    md_lines.extend([
        "",
        "---",
        "",
        content["main_text"],
        "",
    ])

    with open(filepath, "w", encoding="utf-8") as f:
        f.write("\n".join(md_lines))

    return filepath


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    total = len(COURSES)
    success = 0
    failed = []

    for i, (num, label, title, path) in enumerate(COURSES):
        url = BASE_URL + path
        print(f"[{i + 1}/{total}] 抓取 {label}: {title} ...", flush=True)

        try:
            html = fetch_page(url)
            content = extract_article(html)
            filepath = save_course(num, label, title, content, path)
            size = os.path.getsize(filepath)
            print(f"  ✓ 保存 {os.path.basename(filepath)} ({size:,} bytes)", flush=True)
            success += 1
        except Exception as e:
            print(f"  ✗ 失败: {e}", flush=True)
            failed.append((num, label, title, str(e)))

        # Rate limiting: 0.5s between requests
        if i < total - 1:
            time.sleep(0.5)

    print(f"\n完成: {success}/{total} 成功", flush=True)
    if failed:
        print(f"失败 ({len(failed)}):", flush=True)
        for num, label, title, err in failed:
            print(f"  - {num} {label}: {err}", flush=True)


if __name__ == "__main__":
    main()
