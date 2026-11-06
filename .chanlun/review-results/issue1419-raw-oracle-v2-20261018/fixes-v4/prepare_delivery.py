"""封包辅助：只复制指定历史证据并记录固定Git对象的精确摘录；不运行产品。"""
import hashlib
import json
from pathlib import Path
import subprocess

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent
REPO=ROOT.parents[2]
BASE='4161ebfe6d1a11b93cccf1de1a8f134c6c7d2c6a'
MANAGER=Path('/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018')


def digest(b):return hashlib.sha256(b).hexdigest()
def write(path,v):path.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def git(*args):return subprocess.check_output(['git',*args],cwd=REPO)


def main():
    refs=[]
    for source,name in [
        (MANAGER/'RAW-V3-INDEPENDENT-REVIEW.json','V3-INDEPENDENT-REVIEW.reference.json'),
        (MANAGER/'run01-artifacts/XERJ-ATTEMPTS.json','XERJ-ATTEMPTS.reference.json'),
        (MANAGER/'run01-artifacts/REFERENCE-HITS.json','REFERENCE-HITS.reference.json'),
        (MANAGER/'run01-artifacts/REFERENCE-SEARCH.md','REFERENCE-SEARCH.reference.md')]:
        b=source.read_bytes();(HERE/name).write_bytes(b)
        refs.append(dict(source=str(source),copy=name,bytes=len(b),sha256=digest(b)))
    for name in ('MANIFEST.json','SHA256SUMS'):
        b=git('show',BASE+':'+str((ROOT/name).relative_to(REPO)))
        target='V3-'+name+'.reference';(HERE/target).write_bytes(b)
        refs.append(dict(source=BASE+':'+str((ROOT/name).relative_to(REPO)),copy=target,bytes=len(b),sha256=digest(b)))
    write(HERE/'REUSED-EVIDENCE.json',dict(status='复用既有失败原件和参考响应；本轮未联网/重装/索引',files=refs))
    sources=[]
    for commit,path,ranges in [
        ('2694afcf147f3ba4114fde386470886c2428efc0','rust/src/bin/s_session_v3/object_records.rs',[(70,119),(769,829)]),
        (BASE,'s_session/browser/s-content-v3.js',[(97,103),(648,681),(746,766)]),
        (BASE,'s_session/browser/q3-consumer.js',[(42,60),(1628,1656)])]:
        b=git('show',commit+':'+path);lines=b.splitlines(keepends=True);excerpts=[]
        for first,last in ranges:
            start=sum(map(len,lines[:first-1]));end=sum(map(len,lines[:last]))
            excerpts.append(dict(first_line=first,last_line=last,byte_start_inclusive=start,byte_end_exclusive=end,
                                 sha256=digest(b[start:end]),text=b[start:end].decode()))
        sources.append(dict(commit=commit,path=path,bytes=len(b),sha256=digest(b),
                            blob=git('rev-parse',commit+':'+path).decode().strip(),excerpts=excerpts))
    write(HERE/'STATIC-WIRE-SOURCES.json',dict(qualification='静态协议参考；不是冻结产品源或运行资格',sources=sources))
    prior=json.loads((ROOT/'fixes-v3/EXACT-RUN-PROPOSAL.json').read_text())
    prior.update(schema='issue1419-raw-driver-contract/4',baseline=BASE,
                 review_status='PENDING_INDEPENDENT_REVIEW_OF_V4_DIFF')
    prior['blockers']=[s.replace('v3离线','v4离线').replace('PENDING_V3_REVIEW','PENDING_V4_REVIEW') for s in prior['blockers']]
    prior['driver_revision']=dict(report='fixes-v4/REPORT.md',matrix='fixes-v4/FIX-MATRIX.json',
        schema='fixes-v4/DRIVER-CONTRACT.schema.json',
        public_payload='仅公共payload.data；受测kind出现payload_json一律拒绝，缺字段不回退',
        work_budget='Graph输入/重复节点/hex/hash/头形状/Collector投影与队列/decode/plain/records共享预留；同collector不重置不加帽',
        watch_requires='bind_named_submissions保留完整accepted_cut，逐代逐字段核frontier与切面身份；不以seq_range.to代input_frontier',
        first_visibility_interface='原接口保留；真实139前缀产品轨迹未接入')
    write(HERE/'EXACT-RUN-PROPOSAL.json',prior)
    schema={'$schema':'https://json-schema.org/draft/2020-12/schema','title':'#1419 v4候选与离线绑定合同',
            'type':'object','required':['schema','authorization','execution_authorized','source_commit','source_tree','binary_sha256','product_run'],
            'properties':{'schema':{'const':'issue1419-raw-driver-contract/4'},'authorization':{'const':False},
                          'execution_authorized':{'const':False},'source_commit':{'const':'PENDING_SOURCE'},
                          'source_tree':{'const':'PENDING_SOURCE'},'binary_sha256':{'const':'PENDING_SOURCE'},'product_run':{'const':'NOT_RUN'}},
            '$defs':{'public_tested_object':{'type':'object','required':['kind','payload'],
                         'not':{'required':['payload_json']},'properties':{'kind':{'type':'string'},'payload':{'type':'object','required':['data'],'properties':{'data':{'type':'object'}}}}},
                     'accepted_binding':{'type':'object','additionalProperties':False,
                         'required':['session_id','session_generation','message_id','source_epoch','accepted_events','accepted_cut'],
                         'properties':{k:{'type':'string'} for k in ('session_id','session_generation','message_id','source_epoch','accepted_events')}}}}
    fixed='session_id session_generation structure_cut cut_generation catalog_revision index_frontier profile_id profile_hash input_frontier seq_range catalog_run_status catalog_evidence scope rule_revision history_mode as_of_generation'.split()
    schema['$defs']['accepted_binding']['properties']['accepted_cut']={'type':'object','required':fixed,'additionalProperties':False,
        'properties':{k:({'type':'object'} if k in ('seq_range','catalog_evidence','scope') else {'type':'string'}) for k in fixed}}
    write(HERE/'DRIVER-CONTRACT.schema.json',schema)


if __name__=='__main__':main()
