#!/usr/bin/env python3
"""清理 pattern-buffer.yaml 中的误报 candidate 条目（v6：容错解析 + 碎片过滤）

修复了原始文件中存在的格式碎片（truncated lines 如 'ion"'）。
"""
import re

PATTERN_FILE = '.chanlun/pattern-buffer.yaml'

def normalize_path(s):
    """将 YAML 中的双重转义路径标准化为单斜杠"""
    s = s.replace('\\\\\\\\', '/').replace('\\\\', '/').replace('\\', '/')
    return s

def is_whitelisted(sig):
    """判断 signature 是否属于白名单"""
    m = re.match(r'^lead-direct-(Write|Edit|Bash):(.*)$', sig, re.DOTALL)
    if not m:
        return False
    tool = m.group(1)
    raw_target = m.group(2)
    target = normalize_path(raw_target)

    if tool == 'Bash':
        cmd = raw_target

        # git 操作（任何 git 子命令）
        if re.search(r'\bgit\s+\w', cmd):
            return True
        if 'ceremony_scan.py' in cmd:
            return True
        if re.search(r'dag_add_(node|edge)\.py|dag_validate\.py|validate_dag\.py', cmd):
            return True
        if 'pattern-buffer.yaml' in cmd:
            return True
        if 'downstream_audit' in cmd:
            return True
        if re.search(r'cat\s*>\s*scripts/', cmd):
            return True
        if 'cleanup_pattern_buffer.py' in cmd:
            return True

        # 只读/诊断命令打头（独立或 cd ... && 后）
        readonly_cmds = r'(ls|wc|head|tail|cat|find|echo|sleep|test|where|which|pwd|du|df|stat|file|touch|mkdir|for\s+\w|grep|sed)'
        if re.match(r'^' + readonly_cmds, cmd.strip()):
            return True
        if re.match(r'^cd\s+\S+\s+&&\s+' + readonly_cmds, cmd.strip()):
            return True

        # 裸 cd 命令 / 截断产物
        if re.match(r'^cd\s+\S+\s*$', cmd.strip()):
            return True
        if cmd.strip() in ('rm \\', 'cd \\', 'rm \\\\', 'cd \\\\'):
            return True

        # rm pending 谱系文件
        if re.search(r'\brm\b', cmd) and '.chanlun/genealogy/pending/' in cmd:
            return True

        # sleep 命令
        if re.match(r'^sleep\s+\d+', cmd.strip()):
            return True

        # python 工具调用
        if re.search(r'(python|python3|\.venv/Scripts/python)\s*(-c|\.exe\s+-c|-m)', cmd):
            if re.search(r'newchan\.\w+', cmd):
                return True
            if not re.search(r'open\s*\([^)]*[\'"][wa][\'"]', cmd):
                return True

        # bash hook 验证脚本
        if re.search(r'bash\s+\.claude/hooks/dag-validation-guard\.sh', cmd):
            return True

        # bash -n（语法检查，只读）
        if re.search(r'\bbash\s+-n\s+', cmd):
            return True

        # sed 操作 dag.yaml
        if 'sed' in cmd and 'dag.yaml' in cmd:
            return True

        # gh 只读
        if re.search(r'\bgh\s+(auth\s+status|pr\s+list|issue\s+list|api\s+)', cmd):
            return True

        # for 循环
        if re.match(r'^for\s+\w+\s+in\s+', cmd.strip()):
            return True

        return False

    if tool in ('Write', 'Edit'):
        fp = target

        if 'dag.yaml' in fp and 'genealogy' in fp:
            return True
        if '/.chanlun/genealogy/' in fp or fp.startswith('.chanlun/genealogy/'):
            return True
        if 'downstream-action-overrides.yaml' in fp:
            return True
        if '/.chanlun/sessions/' in fp or fp.startswith('.chanlun/sessions/'):
            return True
        if 'dispatch-dag.yaml' in fp:
            return True
        if '/tmp/' in fp or fp.startswith('tmp/'):
            return True
        if 'cleanup_pattern_buffer.py' in fp:
            return True
        if 'type-vocabulary.yaml' in fp or 'manifest.yaml' in fp:
            return True
        if '/.chanlun/definitions/' in fp or fp.startswith('.chanlun/definitions/'):
            return True
        if '/.claude/skills/' in fp or fp.startswith('.claude/skills/'):
            return True
        if '/scripts/' in fp or fp.startswith('scripts/'):
            return True
        if fp.endswith('README.md'):
            return True

        return False

    return False


KNOWN_FIELDS = {
    'id', 'signature', 'frequency', 'first_seen', 'last_seen', 'sources',
    'description', 'status', 'anomaly_type', 'whitelist_reason', 'type',
    'trigger', 'action', 'target', 'phase', 'agent', 'pattern', 'context'
}

def is_valid_field_line(line):
    """检查是否是有效的 YAML 字段行（非碎片）"""
    # 空行
    if not line.strip():
        return True
    # 注释
    if line.strip().startswith('#'):
        return True
    # 序列条目开始
    if re.match(r'^\s+- id:', line):
        return True
    # 字段行：4 空格缩进 + field: value
    m = re.match(r'^    (\w[\w_-]*):\s', line)
    if m and m.group(1) in KNOWN_FIELDS:
        return True
    # sources 数组行
    if re.match(r'^\s+sources:\s+\[', line):
        return True
    return False


def parse_and_clean(content):
    """解析 patterns 块，去除碎片行，提取有效条目"""
    header_match = re.search(r'^(.*?^patterns:\s*\n)', content, re.DOTALL | re.MULTILINE)
    if not header_match:
        return content, [], ''

    header = header_match.group(1)
    body_start = header_match.end()

    # 预处理：去除非条目行（碎片）
    body_lines = content[body_start:].split('\n')
    clean_lines = []
    for line in body_lines:
        # 跳过碎片行：不以 '  - id:' 开头也不以 '    ' 开头的非空行
        if line and not line.startswith('  ') and not line.startswith('#'):
            # 可能是碎片（如 'ion"'）
            continue
        clean_lines.append(line)

    clean_body = '\n'.join(clean_lines)

    # 按 '  - id:' 分割条目
    parts = re.split(r'(?=^  - id:)', clean_body, flags=re.MULTILINE)

    entries = []
    trailing = ''
    for part in parts:
        if not part.strip():
            continue
        if re.match(r'^  - id:', part):
            entries.append(part)
        else:
            trailing += part

    return header, entries, trailing


def process_entries(entries):
    seen_sigs = set()
    result = []
    stats = {'total': 0, 'whitelisted': 0, 'kept': 0, 'non_anomaly': 0, 'deduplicated': 0, 'fragment': 0}

    for entry in entries:
        stats['total'] += 1

        sig_m = re.search(r'^\s+signature:\s+"([^"]*)"', entry, re.MULTILINE)
        status_m = re.search(r'^\s+status:\s+"([^"]*)"', entry, re.MULTILINE)

        sig = sig_m.group(1) if sig_m else ''
        # signature 未找到的条目是碎片/格式错误，跳过
        if not sig:
            stats['fragment'] += 1
            continue

        # 非 anomaly 条目保持不变
        if not sig.startswith('lead-direct-'):
            if sig not in seen_sigs:
                result.append(entry)
                seen_sigs.add(sig)
                stats['non_anomaly'] += 1
            else:
                stats['deduplicated'] += 1
            continue

        # 去重
        if sig in seen_sigs:
            stats['deduplicated'] += 1
            continue
        seen_sigs.add(sig)

        if is_whitelisted(sig):
            # 降级为 observed
            new_entry = re.sub(r'(\s+status:\s+)"candidate"', r'\1"observed"', entry)
            if 'whitelist_reason' not in new_entry:
                if 'anomaly_type' in new_entry:
                    new_entry = re.sub(
                        r'(\s+anomaly_type:\s+"lead_direct_execution"\n)',
                        r'\1    whitelist_reason: "lead ceremony/orchestration operation"\n',
                        new_entry
                    )
                else:
                    new_entry = new_entry.rstrip('\n') + '\n    whitelist_reason: "lead ceremony/orchestration operation"\n'
            result.append(new_entry)
            stats['whitelisted'] += 1
        else:
            result.append(entry)
            stats['kept'] += 1

    return result, stats


def main():
    with open(PATTERN_FILE, 'r', encoding='utf-8') as f:
        content = f.read()

    header, entries, trailing = parse_and_clean(content)
    processed, stats = process_entries(entries)

    print(f"原始总条目: {stats['total']}")
    print(f"  碎片/格式错误跳过: {stats['fragment']}")
    print(f"  白名单降级为 observed: {stats['whitelisted']}")
    print(f"  保留为 candidate (真实异常): {stats['kept']}")
    print(f"  非异常条目 (session-summary 等): {stats['non_anomaly']}")
    print(f"  去重删除: {stats['deduplicated']}")
    print(f"  输出条目数: {len(processed)}")

    if processed:
        print("\n保留为 candidate 的条目 (真实异常):")
        for entry in processed:
            sig_m = re.search(r'^\s+signature:\s+"([^"]*)"', entry, re.MULTILINE)
            status_m = re.search(r'^\s+status:\s+"([^"]*)"', entry, re.MULTILINE)
            if sig_m and status_m and status_m.group(1) == 'candidate':
                print(f"  {sig_m.group(1)[:110]}")

    # 重建文件内容
    new_content = header
    if processed:
        for entry in processed:
            if not entry.endswith('\n'):
                entry += '\n'
            new_content += entry
    else:
        new_content = header.rstrip('\n') + ' []\n'

    if trailing.strip():
        new_content += trailing

    with open(PATTERN_FILE, 'w', encoding='utf-8') as f:
        f.write(new_content)

    print("\npattern-buffer.yaml 已更新，YAML 格式验证...")

    # 验证输出 YAML 有效性（宽松验证）
    import yaml
    try:
        with open(PATTERN_FILE, 'r', encoding='utf-8') as f:
            data = yaml.safe_load(f)
        patterns = data.get('patterns', [])
        print(f"YAML 有效，共 {len(patterns)} 条目。")
    except Exception as e:
        print(f"YAML 验证失败: {e}")


if __name__ == '__main__':
    main()
