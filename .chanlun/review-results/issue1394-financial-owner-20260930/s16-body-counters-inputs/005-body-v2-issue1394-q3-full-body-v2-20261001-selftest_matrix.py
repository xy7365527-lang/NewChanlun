#!/usr/bin/env python3
"""No-Q cross-language Q3R2 archive and rejection matrix."""
import hashlib, json, shutil, struct, subprocess, sys, zlib
from pathlib import Path
if not __debug__: raise SystemExit('optimized Python disables selftest asserts')
import archive_v2

ROOT=Path(__file__).resolve().parent
def unpack_one(folder):
    folder=Path(folder)
    assert archive_v2.verify(folder)['records']==1
    raw=subprocess.check_output(['zstd','-q','-dc'],input=(folder/'archive-chunk-000001.zst').read_bytes())
    records=list(archive_v2.unpack(raw));assert len(records)==1
    return records[0]
def rejected(folder):
    try:archive_v2.verify(folder)
    except (AssertionError,FileNotFoundError,subprocess.CalledProcessError):return True
    return False
def main(out):
    out=Path(out);out.mkdir(exist_ok=False)
    normal=ROOT/'selftest-normal-v4';term=ROOT/'selftest-parent-v4/term'
    m,q,r=unpack_one(normal)
    assert m=={'serial':1,'complete':True,'error':None,'op':'test','code':200}
    assert q==b'GET /test HTTP/1.0\r\n\r\n' and r==b'HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nOK'
    tm,tq,tr=unpack_one(term)
    assert tm['serial']==1 and tm['complete'] is False and tm['code'] is None and 'SIGTERM' in tm['error']
    assert tq.startswith(b'POST /api/v3/content HTTP/1.0\r\n')
    assert tr==b'HTTP/1.0 200 OK\r\nContent-Length: 100\r\n\r\nabc'
    parent=json.loads((ROOT/'selftest-parent-v4/SELFTEST.json').read_bytes())
    assert parent['status']=='PASS_NO_Q_SELFTEST' and parent['graceful_collector']['same_popen_reaped'] and parent['stubborn_child_escalation']['kill_sent']
    for mode in ('shortwrite','rowerror','http503'):
        folder=ROOT/('selftest-'+mode+'-v4')
        receipt=json.loads((folder/'selftest-result.json').read_bytes())
        assert receipt['archive_closed'] is True and receipt['business_success'] is False
        mm,qq,rr=unpack_one(folder)
        if mode=='shortwrite':assert receipt['shortWriteCalls']>2 and mm['complete'] is True
        if mode=='rowerror':assert receipt['expectedRowError'] is True and mm['complete'] is True
        if mode=='http503':assert mm['complete'] is True and mm['code']==503 and rr.startswith(b'HTTP/1.0 503 ')
    budget=json.loads((ROOT/'selftest-budget-v5/selftest-result.json').read_bytes())
    assert budget['archive_closed'] and budget['budget']['unsafe_refused_before_http'] and budget['budget']['safe_pending_flush_first']
    cases={}
    manifest=(normal/'archive-manifest.jsonl').read_bytes();chunk=(normal/'archive-chunk-000001.zst').read_bytes()
    for name in ('corrupt','truncated','missing','orphan_part','missing_final_lf','bool_chunk','bool_records','bool_first_serial','oversize_chunk'):
        d=out/name;d.mkdir();(d/'archive-manifest.jsonl').write_bytes(manifest)
        item=json.loads(manifest)
        if name=='missing_final_lf':(d/'archive-manifest.jsonl').write_bytes(manifest[:-1])
        if name in ('bool_chunk','bool_records','bool_first_serial'):
            field={'bool_chunk':'chunk','bool_records':'records','bool_first_serial':'first_serial'}[name]
            item[field]=True;(d/'archive-manifest.jsonl').write_bytes(archive_v2.canonical(item)+b'\n')
        if name=='oversize_chunk':
            item['uncompressed_bytes']=archive_v2.MAX_CHUNK+1
            (d/'archive-manifest.jsonl').write_bytes(archive_v2.canonical(item)+b'\n')
        if name!='missing':
            payload=bytearray(chunk)
            if name=='corrupt':payload[len(payload)//2]^=1
            if name=='truncated':payload=payload[:-1]
            (d/'archive-chunk-000001.zst').write_bytes(payload)
        if name=='orphan_part':(d/'archive-chunk-000002.part').write_bytes(b'incomplete')
        cases[name]=rejected(d)
        assert cases[name],name
    # Bypass pack() only in the negative test so the decoder itself must reject
    # malformed complete/partial metadata while compressed-byte hashes remain valid.
    def bad_meta(name,meta):
        d=out/name;d.mkdir()
        q=b'GET /test HTTP/1.0\r\n\r\n';r=b'HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nOK';m=archive_v2.canonical(meta)
        raw=archive_v2.HEADER.pack(archive_v2.MAGIC,len(m),len(q),len(r),zlib.crc32(q+r),hashlib.sha256(q).digest(),hashlib.sha256(r).digest())+m+q+r
        enc=subprocess.run(['zstd','-q','-c','-3','--long=27','--check'],input=raw,capture_output=True,check=True).stdout
        (d/'archive-chunk-000001.zst').write_bytes(enc)
        receipt=dict(json.loads(manifest));receipt.update(uncompressed_bytes=len(raw),uncompressed_sha256=archive_v2.sha(raw),
            compressed_bytes=len(enc),compressed_sha256=archive_v2.sha(enc))
        (d/'archive-manifest.jsonl').write_bytes(archive_v2.canonical(receipt)+b'\n')
        cases[name]=rejected(d);assert cases[name],name
    bad_meta('complete_null_code',{'serial':1,'complete':True,'error':None,'op':'test','code':None})
    bad_meta('complete_bool_code',{'serial':1,'complete':True,'error':None,'op':'test','code':True})
    bad_meta('partial_empty_error',{'serial':1,'complete':False,'error':'','op':'test','code':None})
    bad_meta('partial_bool_code',{'serial':1,'complete':False,'error':'SIGTERM','op':'test','code':True})
    prefix=archive_v2.verify(ROOT/'prefix-test')
    assert prefix['records']==1835 and prefix['chunks']==8 and prefix['partial']==0
    result={'status':'PASS_NO_Q_CARRIER_MATRIX','normal_node_writer_python_reader_exact':True,
            'sigterm_active_socket_partial_wire_exact':True,'sigterm_last_frame_flushed':True,
            'same_popen_graceful_and_KILL_reaped':True,'negative_rejections':cases,
            'shortwrite_loop_and_archive_fsync':True,'row_write_EIO_detected_archive_closed':True,
            'complete_503_original_wire_preserved_business_failure':True,
            'worst_pending_frame_before_HTTP_reservation':budget['budget'],
            'old_prefix_readable_records':prefix['records'],'old_prefix_business_closure':'NOT_CLAIMED',
            'Q_started':False,'client_atomic_install':'NOT_ATTEMPTED'}
    (out/'RESULT.json').write_text(json.dumps(result,sort_keys=True)+'\n')
    return result
if __name__=='__main__':
    if len(sys.argv)!=2:raise SystemExit('usage: python3 selftest_matrix.py NEW_OUTPUT_DIR')
    print(json.dumps(main(sys.argv[1]),sort_keys=True))
