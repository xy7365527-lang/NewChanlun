"""仅受影响的内存/fake测试。记录实际源字节，输出限本轮目录。"""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent
sys.path[:0]=[str(HERE),str(ROOT/'driver'),str(ROOT/'fixes-v3')]
tempfile.tempdir=str(HERE)
import test_v4
from test_v2 import V2Tests
from test_fixes import Fixes


def identities():
    files=[*sorted((ROOT/'driver').glob('*.py')),HERE/'test_v4.py',HERE/'run_checks.py',
           ROOT/'fixes-v3/domain_fields.py',ROOT/'fixes-v3/independent_fixture.py',ROOT/'fixes-v3/test_fixes.py']
    return {str(p.relative_to(ROOT)):dict(bytes=len(p.read_bytes()),sha256=hashlib.sha256(p.read_bytes()).hexdigest()) for p in files}


if __name__=='__main__':
    before=identities()
    names=['test_closure_validates_six_families_and_qcut','test_closure_rejects_missing_node_tamper_extent_and_mixed_anchor',
           'test_codec_large_bytes_numbers_and_json_text','test_domain_adapter_positive_and_zn_mutation_negative',
           'test_domain_withdrawal_rejects_stale_pair','test_driver_collects_fixed_pages_and_full_content',
           'test_live_is_unconditionally_denied']
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(test_v4.Tests)
    suite.addTests(V2Tests(name) for name in names)
    suite.addTest(Fixes('test_f5_original_snapshot_contentref_metadata'))
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    assert identities()==before,'测试期间源身份漂移'
    output=dict(schema='issue1419-v4-targeted-observations/1',qualification='实施工蜂内存/fake检查，非独立评审',
                tests=result.testsRun,failures=len(result.failures),errors=len(result.errors),
                observations=test_v4.OBS,source_identity=before,authorization=False,product_run='NOT_RUN')
    (HERE/'CHECK-OUTPUT.json').write_text(json.dumps(output,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(dict(tests=result.testsRun,failures=len(result.failures),errors=len(result.errors),
                         observations=len(test_v4.OBS)),ensure_ascii=False))
    sys.exit(not result.wasSuccessful())
