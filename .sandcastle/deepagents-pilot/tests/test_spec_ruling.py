
import json
import subprocess

from pilot1742.approval import spec_ruling, spec_ruling_digest


def test_ts_python_ruling_protocol(wayfinder_engine_path):
    module = wayfinder_engine_path
    bodies = ['中文\r\n正文 \r\n', '中文\n正文', '正文\ufeff', '正文\u0085', '正文\u001c', '正文\u2029', '正文\r']
    cases = []
    for body in bodies:
        ruling = (f'**SPEC 独审裁定：PASS**\nSPEC：1340\n作者：author\n独审：reviewer\n'
                  f'正文 sha256：{spec_ruling_digest(body)}\n证据：/review_file.json\n')
        for variant in [ruling, 'nac\u212a', ruling.replace('reviewer', 'AUTHOR'), ruling.replace('/review_file.json', ''),
                        ruling.replace('PASS', 'BLOCK'), ruling.replace('1340', '1341'), '批准 通过',
                        ruling.replace('PASS**', 'BLOCK中文**'),
                        ruling.replace('PASS**', 'BLOCK\u0085**'),
                        ruling.replace('SPEC：1340', 'SPEC：0x53c'),
                        ruling.replace('作者：author', '作者：\u0085author'),
                        ruling.replace(spec_ruling_digest(body), '0' * 64)]:
            for trusted in [True, False]:
                comments = [{'id': 1, 'created_at': '2026-11-07T01:00:00Z',
                             'user': {'login': 'owner'}, 'body': ruling},
                            {'id': 2, 'created_at': '2026-11-07T02:00:00Z',
                             'user': {'login': 'owner' if trusted else 'other'}, 'body': variant}]
                cases.append({'body': body, 'comments': comments})
    runner = """import {readFileSync} from 'node:fs';
      const m = await import(process.argv[1]);
      const cases = JSON.parse(readFileSync(0).toString('utf8'));
      console.log(JSON.stringify(cases.map(c => ({digest: m.specBodyDigest(c.body),
        approved: m.isSpecApproved({number:1340, body:c.body,
          comments:c.comments.map(x => ({body:x.body, author:x.user.login, createdAt:x.created_at}))}, 'owner')}))));"""
    result = subprocess.run(['node', '--experimental-strip-types', '--input-type=module', '-e', runner,
                             module.as_uri()], input=json.dumps(cases), text=True, capture_output=True, check=True)
    for case, actual in zip(cases, json.loads(result.stdout), strict=True):
        assert actual['digest'] == spec_ruling_digest(case['body'])
        assert actual['approved'] == (spec_ruling(1340, case['body'], case['comments'], 'owner')[0] is not None)
