#!/usr/bin/env bash
# topology-guard.sh — PostToolUse hook for Bash (git commit)
# 孤岛检测守卫：从根节点 BFS 扫描引用图，警告不可达文件
#
# 触发条件：Bash 工具调用完成后（git commit 成功时）
# 动作：构建 .chanlun/ 和 .claude/ 的引用图，BFS 标记可达节点，警告孤岛
# 级别：警告（不阻断），因为系统仍在建设中

set -euo pipefail

INPUT=$(cat)

# 只处理 Bash 工具
TOOL_NAME=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))" 2>/dev/null || echo "")
if [ "$TOOL_NAME" != "Bash" ]; then
  exit 0
fi

# 获取命令内容和退出码
COMMAND=$(echo "$INPUT" | python -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_input',{}).get('command',''))" 2>/dev/null || echo "")
EXIT_CODE=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_result',{}).get('exit_code',1))" 2>/dev/null || echo "1")

# 只处理 git commit 成功的情况
if ! echo "$COMMAND" | grep -q "git commit"; then
  exit 0
fi
if [ "$EXIT_CODE" != "0" ]; then
  exit 0
fi

# 提取 cwd
CWD=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('cwd','.'))" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

# BFS 孤岛检测
python -c "
import os, re, json
from collections import deque

# 根节点集合
ROOT_NODES = [
    '.chanlun/dispatch-dag.yaml',
    'CLAUDE.md',
    'src/newchan/skills/manifest.yaml',
]

# 白名单模式（跳过这些文件/目录）
WHITELIST = {
    'README.md', '__pycache__', 'tmp', '.git',
    '.chanlun/sessions/archive',
}

def is_whitelisted(path):
    basename = os.path.basename(path)
    if basename in WHITELIST:
        return True
    for w in WHITELIST:
        if path.startswith(w + '/') or path.startswith(w + os.sep):
            return True
        if '/' + w + '/' in path or '/' + w in path:
            return True
    return False

def collect_files():
    \"\"\"收集 .chanlun/ 和 .claude/ 下的 yaml/md/sh 文件\"\"\"
    files = set()
    for root_dir in ['.chanlun', '.claude']:
        if not os.path.isdir(root_dir):
            continue
        for dirpath, dirnames, filenames in os.walk(root_dir):
            # 跳过白名单目录
            dirnames[:] = [d for d in dirnames if not is_whitelisted(os.path.join(dirpath, d))]
            for fname in filenames:
                if not fname.endswith(('.yaml', '.yml', '.md', '.sh', '.json')):
                    continue
                fpath = os.path.join(dirpath, fname).replace(os.sep, '/')
                if not is_whitelisted(fpath):
                    files.add(fpath)
    # 也加入根节点中非目录内的文件
    for r in ROOT_NODES:
        if os.path.isfile(r):
            files.add(r)
    return files

def extract_references(filepath):
    \"\"\"从文件中提取引用的其他文件路径\"\"\"
    refs = set()
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except (IOError, UnicodeDecodeError):
        return refs

    # YAML path/file/buffer_path 等字段值
    for m in re.finditer(r'(?:path|file|buffer_path|command|source):\s*[\"\\']?([^\"\\'\\n#]+)', content):
        candidate = m.group(1).strip().strip('\"').strip(\"'\")
        if candidate and not candidate.startswith('http') and ('/' in candidate or candidate.endswith(('.yaml', '.yml', '.md', '.sh', '.json'))):
            refs.add(candidate)

    # Markdown 链接 [text](path)
    for m in re.finditer(r'\\[([^\\]]*?)\\]\\(([^)]+)\\)', content):
        candidate = m.group(2).strip()
        if candidate and not candidate.startswith('http') and ('/' in candidate or candidate.endswith(('.yaml', '.yml', '.md', '.sh', '.json'))):
            refs.add(candidate)

    # Shell/code 中的文件路径引用（引号内或赋值）
    for m in re.finditer(r'[\"\\'](\\.(?:chanlun|claude)/[^\"\\'\\n]+)[\"\\']', content):
        refs.add(m.group(1))

    # 反引号内的路径
    for m in re.finditer(r'\x60([^\\x60]*\\.(?:chanlun|claude)/[^\\x60\\n]+)\x60', content):
        candidate = m.group(1).strip()
        if candidate:
            refs.add(candidate)

    # 直接文件名引用（无路径前缀但在已知目录下）
    for m in re.finditer(r'(?:dispatch-dag|pattern-buffer|definitions|adjacency-index|manifest)\\.yaml', content):
        refs.add(m.group(0))

    return refs

def normalize_path(ref, source_dir):
    \"\"\"尝试将引用解析为实际文件路径\"\"\"
    ref = ref.strip().strip('\"').strip(\"'\")
    # 绝对项目路径
    if os.path.isfile(ref):
        return ref.replace(os.sep, '/')
    # 相对于源文件目录
    candidate = os.path.join(source_dir, ref)
    if os.path.isfile(candidate):
        return candidate.replace(os.sep, '/')
    # 相对于项目根
    if os.path.isfile(ref):
        return ref.replace(os.sep, '/')
    return None

# 收集所有文件
all_files = collect_files()

# 构建引用图
graph = {f: set() for f in all_files}
for f in all_files:
    source_dir = os.path.dirname(f)
    for ref in extract_references(f):
        target = normalize_path(ref, source_dir)
        if target and target in all_files:
            graph.setdefault(f, set()).add(target)

# BFS 从根节点出发
reachable = set()
queue = deque()
for root in ROOT_NODES:
    root_norm = root.replace(os.sep, '/')
    if root_norm in all_files:
        queue.append(root_norm)
        reachable.add(root_norm)

while queue:
    node = queue.popleft()
    for neighbor in graph.get(node, set()):
        if neighbor not in reachable:
            reachable.add(neighbor)
            queue.append(neighbor)

# 检测孤岛
islands = sorted(all_files - reachable)

# 过滤 session 文件（session 文件通过 CLAUDE.md line 183 的协议间接引用）
islands = [i for i in islands if '/sessions/' not in i]

if not islands:
    # 无孤岛，静默通过
    pass
else:
    lines = ['  - ' + i for i in islands[:20]]
    if len(islands) > 20:
        lines.append(f'  ... 及其他 {len(islands) - 20} 个文件')
    island_list = '\\n'.join(lines)
    msg = (
        f'[topology-guard] 警告：检测到 {len(islands)} 个孤岛文件（无根节点可达路径）。\\n'
        f'孤岛文件：\\n{island_list}\\n'
        '处理方式：\\n'
        '  1. 在某个已连接文件中添加对孤岛文件的引用\\n'
        '  2. 如果文件确实不再需要，删除它\\n'
        '  3. 如果是新建文件，确保至少一个已有文件引用它'
    )
    print(json.dumps({
        'decision': 'block',
        'reason': msg
    }, ensure_ascii=False))
" 2>/dev/null || true

# ── manifest 注册检查 ──────────────────────────────────────────────
# 检查 staged 中新创建的文件是否已注册到 manifest.yaml
python -c "
import subprocess, os, json, sys

MANIFEST_PATH = '.chanlun/manifest.yaml'

# 需要检查注册的路径模式
MONITORED = {
    'src/newchan/': '.py',
    '.claude/commands/': '.md',
    '.claude/agents/': '.md',
    '.claude/hooks/': '.sh',
}

# 排除的目录和文件
EXCLUDE_DIRS = {'tests/', 'tmp/', '__pycache__/'}
EXCLUDE_FILES = {'__init__.py'}

def get_new_staged_files():
    \"\"\"获取 staged 中新增的文件（git status A）\"\"\"
    try:
        result = subprocess.run(
            ['git', 'diff', '--cached', '--name-status', '--diff-filter=A'],
            capture_output=True, text=True, timeout=10
        )
        if result.returncode != 0:
            return []
        files = []
        for line in result.stdout.strip().split('\n'):
            if not line.strip():
                continue
            parts = line.split('\t', 1)
            if len(parts) == 2:
                files.append(parts[1].strip())
        return files
    except Exception:
        return []

def should_check(filepath):
    \"\"\"判断文件是否在监控范围内\"\"\"
    basename = os.path.basename(filepath)
    if basename in EXCLUDE_FILES:
        return False
    for exc in EXCLUDE_DIRS:
        if exc in filepath:
            return False
    for prefix, ext in MONITORED.items():
        if filepath.startswith(prefix) and filepath.endswith(ext):
            return True
    return False

def load_manifest():
    \"\"\"加载 manifest.yaml 内容为文本（简单字符串匹配）\"\"\"
    try:
        with open(MANIFEST_PATH, 'r', encoding='utf-8') as f:
            return f.read()
    except (IOError, FileNotFoundError):
        return ''

def check_registration(filepath, manifest_content):
    \"\"\"检查文件是否在 manifest 中有对应条目\"\"\"
    return ('file: ' + filepath) in manifest_content

new_files = get_new_staged_files()
manifest_content = load_manifest()
unregistered = []

for f in new_files:
    if should_check(f) and not check_registration(f, manifest_content):
        unregistered.append(f)

if unregistered:
    file_list = '\n'.join(['  - ' + f for f in unregistered])
    msg = (
        f'[topology-guard/manifest] 警告：{len(unregistered)} 个新文件未注册到 manifest.yaml\n'
        f'{file_list}\n'
        '请使用 DynamicRegistry 注册，或确认该文件不需要注册（测试/临时文件）。'
    )
    # 只警告，不阻断
    print(json.dumps({
        'decision': 'warn',
        'reason': msg
    }, ensure_ascii=False))
" 2>/dev/null || true
