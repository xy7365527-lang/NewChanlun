#!/usr/bin/env python3
# 来源 tb03b_oracle.py；保留独算公式，assert 转拒绝异常，实际原库只读由 driver 提供。
"""#1394：从冻结现金原件独立核账，不导入任何生产 reducer/查询投影。"""
import argparse
import hashlib
import json
from pathlib import Path
import re
from driver import require, read_domain, strict, ExecutionAdmission

def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode()

def sha(value):
    return hashlib.sha256(canonical(value)).hexdigest()

def amount(value):
    if not isinstance(value, str) or not re.fullmatch('0|[1-9][0-9]*', value):
        raise ValueError('oracle 数值不是规范非负整数文本')
    result = int(value)
    if result > 2 ** 63 - 1:
        raise ValueError('oracle 数值溢出')
    return result

def check(manifest_path, expect='committed', *, admission=None, budget=None,
          audit_contract=None):
    """真实入口必须另具冻结核账合同；不再用第一条 slot/owner 覆盖其余 decision。"""
    from driver import ReadBudget, read_original
    from financial_oracle import audit
    require(type(admission) is ExecutionAdmission, '独立核账实际读库尚未获根精确准入')
    require(type(budget) is ReadBudget and audit_contract is not None,
            '缺共享 Job 预算或冻结完整核账原件；不从 actual 生成 expected')
    frozen = strict(audit_contract.read(budget))
    manifest = strict(read_original(manifest_path, budget))
    require(manifest['schema_revision'] == 'economic-manifest/1' and
            manifest.get('protocol_revision') == 'economic-session/2', '经济协议不符')
    require(frozen['manifest_sha256'] == sha(manifest), '核账合同/manifest 不同')
    data_raw = read_original(manifest['finance']['input_path'], budget)
    domains = {'E': read_domain(manifest['e']['db'], budget=budget)}
    domains.update({'B:' + b['chong_id']: read_domain(b['db'], budget=budget)
                    for b in manifest['chongs']})
    return audit(data_raw, manifest['finance']['input_sha256'],
                 session_id=manifest['session_id'], session_generation=manifest['session_generation'],
                 initial_books=frozen['initial_books'], initial_sequences=frozen['initial_sequences'],
                 operations=frozen['operations'], histories={k: v['commits'] for k,v in domains.items()}, budget=budget)

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--manifest', type=Path, required=True)
    p.add_argument('--expect', choices=('committed', 'awaiting_policy', 'unavailable_alpha', 'reserved'), default='committed')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    result = check(args.manifest, args.expect)
    with args.output.open('x') as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2)
    print(json.dumps({k: v for k, v in result.items() if k != 'semantic'}, ensure_ascii=False))
if __name__ == '__main__':
    raise SystemExit('execution_authorized=false: 请由根冻结的调度合同调用独立核账适配层')
