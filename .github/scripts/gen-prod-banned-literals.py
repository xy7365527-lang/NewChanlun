#!/usr/bin/env python3
"""#1640 §2.3 第 3 层：从测试门内的字符串字面量自动抽取产物禁串清单。

从 crate 根（`rust/src/lib.rs`、`rust/src/bin/s_structure_session.rs`）沿 `mod` 声明遍历全部
模块文件，逐个字符串字面量求其 cfg 链：生产配置（{s_session}、{s_session, s_incremental}，
test=false）均不可达、而测试配置（cfg(test) 或 testonly feature）可达的，记为测试字面量。
再去掉与任何生产可达字面量相同或为其子串者、含控制字符或 `{`/`}`（格式串在产物里被拆段）者，
并只保留本仓特有的字面（见 `distinctive`），剩余即禁串。输出入仓到 `.github/prod-banned-literals.txt`，
由 `check-prod-artifact-hooks.sh` 读取。

用法：python3 .github/scripts/gen-prod-banned-literals.py [--check] [仓根]
  无 --check：重写清单；--check：清单与当前源码不一致时 exit 1。
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

CRATE_ROOTS = ("rust/src/lib.rs", "rust/src/bin/s_structure_session.rs")
OUTPUT = ".github/prod-banned-literals.txt"
MIN_BYTES = 16
# 只收本仓特有、依赖与 std 不会自带的字面：含非 ASCII（中文诊断/断言）、全大写环境变量名、
# 或带测试钩子词。纯 ASCII 标识符样字面（serde/Debug 派生名、SQL 列名、依赖报错原文）不收，
# 否则依赖库自带的同名字面会让产物检查误红。
ENV_NAME = re.compile(r"[A-Z][A-Z0-9_]{7,}")
HOOK_WORD = re.compile(r"testonly|for_test|_probe|probe_|inject|fault", re.I)
STD_ENV_PREFIXES = ("RUST_", "CARGO_", "LD_", "DYLD_")
IDENTIFIER = re.compile(r"[A-Za-z0-9_]+")
PRODUCTION = (
    (frozenset({"s_session"}), False),
    (frozenset({"s_session", "s_incremental"}), False),
)
TEST_FEATURE_SETS = (
    frozenset({"s_session"}),
    frozenset({"s_session", "s_incremental"}),
    frozenset({"s_session", "s_incremental", "s_capture_native_testonly"}),
    frozenset({"s_session", "s_allocation_json_testonly"}),
)
TESTING = tuple(
    (features, test) for features in TEST_FEATURE_SETS for test in (False, True)
    if (features, test) not in PRODUCTION
)
HEADER_PREFIX = "#@ "
HEADER = (
    HEADER_PREFIX + "#1640 §2.3 第 3 层：生产增量产物禁串（测试门内字面量，自动生成，勿手改）。\n"
    + HEADER_PREFIX + "生成：python3 .github/scripts/gen-prod-banned-literals.py；每行一个字面，按字节匹配。\n"
)

_SPEC = importlib.util.spec_from_file_location(
    "check_prod_no_test_hooks", Path(__file__).with_name("check-prod-no-test-hooks.py")
)
_checker = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_checker)

ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\", '"': '"', "'": "'"}
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def outcomes(tree, features, test) -> frozenset:
    if tree[0] == "call":
        _, name, args = tree
        if name in ("any", "all"):
            parts = [outcomes(arg, features, test) for arg in args]
            if name == "any":
                can_true, can_false = any(True in o for o in parts), all(False in o for o in parts)
            else:
                can_true, can_false = all(True in o for o in parts), any(False in o for o in parts)
            return frozenset(v for v, ok in ((True, can_true), (False, can_false)) if ok)
        if name == "not" and len(args) == 1:
            return frozenset(not v for v in outcomes(args[0], features, test))
        return frozenset({True, False})
    if tree[0] == "pair" and tree[1] == "feature":
        return frozenset({tree[2] in features})
    if tree == ("word", "test"):
        return frozenset({test})
    return frozenset({True, False})


def parse_attr(text: str):
    body = text[text.index("(") + 1 : text.rindex(")")]
    try:
        return _checker.parse_cfg(body)
    except (IndexError, ValueError):
        return None


def reachable(chain, features, test) -> bool:
    return all(tree is None or True in outcomes(tree, features, test) for tree in chain)


def unescape(raw: str) -> str | None:
    out, index = [], 0
    while index < len(raw):
        char = raw[index]
        if char != "\\":
            out.append(char)
            index += 1
            continue
        code = raw[index + 1]
        if code in ESCAPES:
            out.append(ESCAPES[code])
            index += 2
        elif code == "x":
            out.append(chr(int(raw[index + 2 : index + 4], 16)))
            index += 4
        elif code == "u":
            end = raw.index("}", index)
            out.append(chr(int(raw[index + 3 : end], 16)))
            index = end + 1
        elif code == "\n":
            index += 2
            while index < len(raw) and raw[index] in " \t\r\n":
                index += 1
        else:
            return None
    return "".join(out)


def lex(text: str):
    """产出 ('attr', 原文) / ('str', 值或 None) / ('ident', 名) / ('punct', 字符)。"""
    index, size = 0, len(text)
    while index < size:
        char = text[index]
        if char.isspace():
            index += 1
        elif text.startswith("//", index):
            index = text.find("\n", index)
            index = size if index < 0 else index
        elif text.startswith("/*", index):
            depth, index = 1, index + 2
            while depth and index < size:
                if text.startswith("/*", index):
                    depth, index = depth + 1, index + 2
                elif text.startswith("*/", index):
                    depth, index = depth - 1, index + 2
                else:
                    index += 1
        elif char == "#" and (text.startswith("[", index + 1) or text.startswith("![", index + 1)):
            start = index
            index = text.index("[", index) + 1
            depth = 1
            while depth:
                if text[index] == '"':
                    index = skip_string(text, index)
                    continue
                depth += {"[": 1, "]": -1}.get(text[index], 0)
                index += 1
            yield ("attr", re.sub(r"\s+", " ", text[start:index]))
        else:
            raw = re.match(r'(?:b|c)?r(#*)"', text[index:])
            if raw and (index == 0 or not (text[index - 1].isalnum() or text[index - 1] == "_")):
                hashes = raw.group(1)
                start = index + raw.end()
                end = text.index('"' + hashes, start)
                byte = text[index] == "b"
                yield ("str", None if byte else text[start:end])
                index = end + 1 + len(hashes)
            elif char == '"' or (char in "bc" and text[index + 1 : index + 2] == '"'):
                start = index + (1 if char == '"' else 2)
                end = skip_string(text, start - 1)
                yield ("str", None if char == "b" else unescape(text[start : end - 1]))
                index = end
            elif char == "'":
                if text[index + 1] == "\\":
                    index = text.index("'", index + 3) + 1
                elif index + 2 < size and text[index + 2] == "'":
                    index += 3
                else:
                    index += 1
            elif IDENT.match(text, index):
                name = IDENT.match(text, index).group(0)
                yield ("ident", name)
                index += len(name)
            else:
                yield ("punct", char)
                index += 1


def skip_string(text: str, quote: int) -> int:
    index = quote + 1
    while text[index] != '"':
        index += 2 if text[index] == "\\" else 1
    return index + 1


def child_path(current: Path, name: str, explicit: str | None, is_dir_owner: bool) -> Path | None:
    base = current.parent if is_dir_owner or explicit else current.parent / current.stem
    if explicit:
        return base / explicit
    for candidate in (base / f"{name}.rs", base / name / "mod.rs"):
        if candidate.is_file():
            return candidate
    return None


ENV_READ = [("ident", "env"), ("punct", ":"), ("punct", ":")]


def walk(path: Path, chain, is_dir_owner: bool, literals: list, queue: list, env_reads: list | None = None) -> None:
    tokens = list(lex(path.read_text(encoding="utf-8")))
    consts = {}
    for position, (kind, value) in enumerate(tokens):
        if kind == "str" and tokens[position - 1] == ("punct", "="):
            head = tokens[max(0, position - 9) : position]
            names = [i for i, token in enumerate(head) if token == ("ident", "const")]
            if names and names[-1] + 1 < len(head) and head[names[-1] + 1][0] == "ident":
                consts[head[names[-1] + 1][1]] = value
    file_chain = list(chain)
    frames: list[tuple[list, int]] = []
    pending: list[tuple[object, int]] = []
    explicit_path = None
    depth = 0
    signature = False  # fn 签名内（到 `{`/`;` 为止）：返回类型泛型里的逗号不结束该 fn 的 cfg
    for position, (kind, value) in enumerate(tokens):
        scope = file_chain + [tree for gates, _ in frames for tree in gates]
        if kind == "attr":
            if value.startswith("#![cfg("):
                file_chain.append(parse_attr(value))
            elif value.startswith("#[cfg("):
                pending.append((parse_attr(value), depth))
            elif re.match(r'#\[path = "', value):
                explicit_path = value.split('"')[1]
        elif kind == "str":
            literals.append((value, scope + [tree for tree, _ in pending]))
        if kind in ("str", "ident") and env_reads is not None:
            call = tokens[position - 5 : position]
            if (call[:3] == ENV_READ and call[4] == ("punct", "(")
                    and call[3] in (("ident", "var"), ("ident", "var_os"))):
                # 参数是同文件 const 时取其字面；其余非字面参数原样记为不可解析，名单外即判违规。
                name = value if kind == "str" else consts.get(value, "<非字面量参数 " + value + ">")
                env_reads.append((name, scope + [tree for tree, _ in pending], path))
        if kind == "ident" and value == "fn":
            signature = True
        if kind == "punct":
            if value in "{;":
                signature = False
            if value in "([":
                depth += 1
            elif value in ")]":
                depth -= 1
                pending = [(tree, at) for tree, at in pending if at <= depth]
            elif value == "," and not signature:
                pending = [(tree, at) for tree, at in pending if at < depth]
            elif value == "{":
                frames.append(([tree for tree, _ in pending], depth))
                pending, explicit_path = [], None
            elif value == "}":
                gates, depth = frames.pop() if frames else ([], 0)
                following = tokens[position + 1] if position + 1 < len(tokens) else None
                pending = [(tree, depth) for tree in gates] if following == ("ident", "else") else []
            elif value == ";":
                previous = tokens[position - 2 : position]
                if len(previous) == 2 and previous[0] == ("ident", "mod") and previous[1][0] == "ident":
                    target = child_path(path, previous[1][1], explicit_path, is_dir_owner)
                    if target is not None:
                        gates = scope + [tree for tree, _ in pending]
                        queue.append((target, gates, target.name == "mod.rs"))
                pending, explicit_path = [], None


def in_scope(root: Path, path: Path) -> bool:
    """禁串只取 §2.3 管辖面（与第 2 层同一 ROOTS）；生产排除面取整个 crate 图。"""
    for base in _checker.ROOTS:
        target = (root / base).resolve()
        if path == target or target in path.parents:
            return True
    return False


def collect(root: Path, env_reads: list | None = None):
    literals: list = []
    queue = [(root / crate, [], True) for crate in CRATE_ROOTS]
    seen: set[Path] = set()
    while queue:
        path, chain, is_dir_owner = queue.pop(0)
        path = path.resolve()
        if path in seen or not path.is_file():
            continue
        seen.add(path)
        start = len(literals)
        walk(path, chain, is_dir_owner, literals, queue, env_reads)
        scoped = in_scope(root, path)
        for index in range(start, len(literals)):
            literals[index] = (*literals[index], scoped)
    return literals


def banned_literals(root: Path) -> list[str]:
    production, testing = set(), set()
    for value, chain, scoped in collect(root):
        if value is None:
            continue
        if any(reachable(chain, features, test) for features, test in PRODUCTION):
            production.add(value)
        elif scoped and any(reachable(chain, features, test) for features, test in TESTING):
            testing.add(value)
    production_blob = "\0".join(sorted(production))
    out = []
    for value in sorted(testing):
        if (not distinctive(value) or any(ord(c) < 0x20 or ord(c) == 0x7F for c in value)
                or "{" in value or "}" in value or value.startswith(HEADER_PREFIX.strip())
                or value in production_blob):
            continue
        out.append(value)
    return out


def distinctive(value: str) -> bool:
    if ENV_NAME.fullmatch(value):
        return not value.startswith(STD_ENV_PREFIXES)
    if len(value.encode("utf-8")) < MIN_BYTES:
        return False
    if any(ord(c) > 0x7F for c in value):
        return True
    return bool(HOOK_WORD.search(value)) and not IDENTIFIER.fullmatch(value)


def main() -> int:
    args = [arg for arg in sys.argv[1:] if arg != "--check"]
    root = Path(args[0] if args else ".").resolve()
    rendered = HEADER + "".join(value + "\n" for value in banned_literals(root))
    target = root / OUTPUT
    if "--check" in sys.argv[1:]:
        if not target.is_file() or target.read_text(encoding="utf-8") != rendered:
            print(f"::error::{OUTPUT} 与源码不一致，须重跑 gen-prod-banned-literals.py")
            return 1
        print(f"{OUTPUT} 与源码一致")
        return 0
    target.write_text(rendered, encoding="utf-8")
    print(f"gen-prod-banned-literals count={rendered.count(chr(10)) - HEADER.count(chr(10))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
