#!/usr/bin/env python3
"""Independent S16 s-content/1 and Q3 body/source oracle for Q3R2 carrier.

Reads the frozen fullDB JSON, never opens its SQLite source. With --actual it
also checks the owned collector's raw HTTP proof and independently renders
every native record. No JS renderer or production content codec is imported.
"""
import argparse
import base64
import gzip
import codecs
import hashlib
import json
import re
import struct
import subprocess
import sys
import zlib
from functools import lru_cache
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path('/Users/silencehan/Projects/NewChanlun-1394-content-v3')
HERE = ROOT / '.scratch/issue1394-q3-full-body-v2-20261001'
OLD_ORACLE_SHA = 'bb3a6655adab81f97c66491fde964b1e4b117b6f532842e8f82c95efda3e9c4f'
ARCHIVE_FORMAT = 'Q3R2/zstd-independent-256-or-16MiB/1'
RECORD_HEADER = struct.Struct('>4sIIII32s32s')
MAX_RECORD = 10*1024*1024
MAX_CHUNK = 16*1024*1024
FROZEN = ROOT / '.scratch/issue1394-formal-fullclock-prefix16-20261001/release1/A/final.fullDB.json'
FROZEN_SHA = '8a7f8f248d1ffad4081555d2900bfd9bc73fb32971167edb544cb15292ea8174'
D = b'NewChanlun/SContent/1\0'
NUMBER = re.compile(rb'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?\Z')
HEX = re.compile(r'[0-9a-f]{64}\Z')
VALUE_TAGS = frozenset(range(9))


class Number(str):
    """Exact JSON number token, deliberately unequal in type to strings."""


class JsonText:
    def __init__(self, inner): self.inner = inner


class Template:
    pass


class Ref:
    def __init__(self, root): self.root = root


def require(ok, message):
    if not ok: raise ValueError(message)


def digest(data): return hashlib.sha256(data).digest()


def u64(raw):
    require(len(raw) == 8, 'U64 width')
    n = struct.unpack('>Q', raw)[0]
    require(n <= 2**63-1, 'U64 domain')
    return n


def f(tag, body):
    require(type(tag) is int and 0 <= tag <= 255 and len(body) <= 2**63-1, 'F arguments')
    return digest(D + bytes((tag,)) + struct.pack('>Q', len(body)) + body)


def seq(hashes):
    n = len(hashes)
    if n == 0: return f(0x11, b'')
    if n == 1: return f(0x12, hashes[0])
    k = 1 << ((n-1).bit_length()-1)
    return f(0x13, struct.pack('>Q', n) + seq(hashes[:k]) + seq(hashes[k:]))


def txt(value):
    require(type(value) is str, 'string expected')
    raw = value.encode('utf-8', 'strict')
    return f(3, struct.pack('>Q', len(raw)) + seq([f(0x10, raw[i:i+4096]) for i in range(0,len(raw),4096)]))


def number_token(raw):
    require(NUMBER.fullmatch(raw) is not None, 'illegal JSON number token')
    return Number(raw.decode('ascii'))


def pairs_unique(pairs):
    result = {}
    for key, value in pairs:
        require(type(key) is str and key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def parse_json(raw):
    if type(raw) is str: raw = raw.encode('utf-8', 'strict')
    require(type(raw) is bytes, 'JSON input bytes')
    value = json.loads(raw.decode('utf-8', 'strict'), parse_int=lambda s:number_token(s.encode()),
                       parse_float=lambda s:number_token(s.encode()), object_pairs_hook=pairs_unique,
                       parse_constant=lambda s: (_ for _ in ()).throw(ValueError('non-JSON constant '+s)))
    return value


def escaped(value):
    """§5.1 exact string renderer; Python json.dumps differs for several cases."""
    require(type(value) is str, 'render string type')
    out = bytearray(b'"')
    short = {8:b'\\b', 9:b'\\t', 10:b'\\n', 12:b'\\f', 13:b'\\r'}
    for ch in value:
        cp = ord(ch)
        require(not 0xD800 <= cp <= 0xDFFF, 'isolated surrogate')
        if cp == 34: out += b'\\"'
        elif cp == 92: out += b'\\\\'
        elif cp in short: out += short[cp]
        elif cp < 32: out += ('\\u%04x' % cp).encode()
        else: out += ch.encode('utf-8')
    out += b'"'
    return bytes(out)


def render(value):
    if value is None: yield b'null'
    elif type(value) is bool: yield b'true' if value else b'false'
    elif type(value) is Number: yield value.encode('ascii')
    elif type(value) is str: yield escaped(value)
    elif type(value) is JsonText:
        # A JSON_TEXT value's logical value is the canonical inner text string.
        inner = b''.join(render(value.inner)).decode('utf-8', 'strict')
        yield escaped(inner)
    elif type(value) is list:
        yield b'['
        for i, item in enumerate(value):
            if i: yield b','
            yield from render(item)
        yield b']'
    elif type(value) is dict:
        yield b'{'
        for i, key in enumerate(sorted(value, key=lambda x:x.encode('utf-8', 'strict'))):
            if i: yield b','
            yield escaped(key); yield b':'; yield from render(value[key])
        yield b'}'
    else: raise ValueError('unrenderable value '+type(value).__name__)


def body_hash(value):
    h = hashlib.sha256(); n = 0
    for part in render(value): h.update(part); n += len(part)
    return h.hexdigest(), n


def same(a,b):
    """No Python bool/int equality, null/missing or number spelling collapse."""
    return v_digest(a)==v_digest(b)


def descriptor_root(desc):
    require(type(desc) is dict and set(desc)=={'format','root','kind','extent'},'ContentRef exact fields')
    require(desc['format']=='s-content/1' and type(desc['root']) is str and desc['root'].startswith('sc1-') and HEX.fullmatch(desc['root'][4:]),'ContentRef root syntax')
    return bytes.fromhex(desc['root'][4:])


def v_digest(value):
    if type(value) is Ref: return value.root
    if value is None: return f(0,b'')
    if type(value) is bool: return f(2 if value else 1,b'')
    if type(value) is str:
        raw=value.encode('utf-8','strict')
        return f(3,struct.pack('>Q',len(raw))+seq([f(0x10,raw[i:i+4096]) for i in range(0,len(raw),4096)]))
    if type(value) is Number:
        raw=value.encode('ascii');number_token(raw)
        return f(4,struct.pack('>Q',len(raw))+seq([f(0x10,raw[i:i+4096]) for i in range(0,len(raw),4096)]))
    if type(value) is JsonText: return f(7,v_digest(value.inner))
    if type(value) is list: return f(5,struct.pack('>Q',len(value))+seq([v_digest(x) for x in value]))
    if type(value) is dict:
        keys=sorted(value,key=lambda x:x.encode('utf-8','strict'))
        return f(6,struct.pack('>Q',len(keys))+seq([f(0x14,v_digest(k)+v_digest(value[k])) for k in keys]))
    raise ValueError('cannot hash value '+type(value).__name__)


class Store:
    def __init__(self, rows):
        self.nodes = {}
        for row in rows:
            d = bytes.fromhex(row[0]['blob_hex']); tag = row[1]; body = bytes.fromhex(row[2]['blob_hex'])
            require(len(d) == 32 and d == f(tag, body), 'native F digest')
            require(d not in self.nodes, 'duplicate native digest')
            self.nodes[d] = (tag,body)
        self.count = len(self.nodes)

    def validate_all_nodes(self):
        """Validate all stored local tags, child links and unique SEQ shapes."""
        @lru_cache(maxsize=None)
        def seq_count(d):
            tag,body=self.node(d)
            if tag==0x11:require(body==b'','stored empty sequence');return 0
            if tag==0x12:
                require(len(body)==32 and body in self.nodes,'stored sequence leaf');return 1
            require(tag==0x13 and len(body)==72,'stored sequence branch')
            n=u64(body[:8]);require(n>1,'stored sequence branch n')
            left=seq_count(body[8:40]);right=seq_count(body[40:])
            k=1<<((n-1).bit_length()-1)
            require(left==k and right==n-k,'stored sequence unique split')
            return n
        for d,(tag,body) in self.nodes.items():
            require(0<=tag<=0x14,'stored unknown tag')
            if tag in (0,1,2,8,0x11):require(body==b'','stored empty body')
            elif tag in (3,4,5,6):
                require(len(body)==40,'stored value header')
                size=u64(body[:8]);count=seq_count(body[8:])
                require(count==((size+4095)//4096 if tag in (3,4) else size),'stored value sequence extent')
            elif tag==7:require(len(body)==32 and body in self.nodes,'stored JSON_TEXT body')
            elif tag==0x10:require(0<len(body)<=4096,'stored byte chunk size')
            elif tag in (0x12,0x13):seq_count(d)
            elif tag==0x14:require(len(body)==64 and body[:32] in self.nodes and body[32:] in self.nodes,'stored field children')
            else:raise ValueError('stored reserved tag')
        for d,(tag,_) in self.nodes.items():
            if tag in VALUE_TAGS:self.value(d)
        return len(self.nodes)

    def node(self, d):
        require(type(d) is bytes and len(d) == 32 and d in self.nodes, 'missing content node '+getattr(d,'hex',lambda:repr(d))())
        return self.nodes[d]

    def sequence(self, root, n):
        """Return logical child digests after checking the unique M shape."""
        tag, body = self.node(root)
        if n == 0:
            require(tag == 0x11 and body == b'', 'M empty shape')
            return []
        if n == 1:
            require(tag == 0x12 and len(body) == 32, 'M leaf shape')
            return [body]
        require(tag == 0x13 and len(body) == 72 and u64(body[:8]) == n, 'M branch shape/count')
        k = 1 << ((n-1).bit_length()-1)
        return self.sequence(body[8:40],k) + self.sequence(body[40:],n-k)

    def byte_string(self, body, number=False):
        require(len(body) == 40, 'byte value header length')
        size = u64(body[:8]); count = (size+4095)//4096
        chunks = []
        for i, ref in enumerate(self.sequence(body[8:],count)):
            tag, chunk = self.node(ref)
            require(tag == 0x10 and len(chunk) == (size-4096*i if i == count-1 else 4096)
                    and len(chunk)>0, 'B chunk type/size')
            chunks.append(chunk)
        raw = b''.join(chunks)
        require(len(raw) == size, 'B byte length')
        if number: return number_token(raw)
        return raw.decode('utf-8','strict')

    @lru_cache(maxsize=4096)
    def value(self, root, internal=False):
        tag, body = self.node(root)
        require(tag in VALUE_TAGS and (not internal or tag <= 6), 'value tag/internal JSON type')
        if tag in (0,1,2,8):
            require(body == b'', 'scalar body')
            return (None,False,True,Template())[tag if tag < 3 else 3]
        if tag in (3,4): return self.byte_string(body, tag == 4)
        if tag == 7:
            require(len(body)==32, 'JSON_TEXT child digest')
            return JsonText(self.value(body,True))
        require(len(body)==40, 'collection header')
        n = u64(body[:8]); children = self.sequence(body[8:],n)
        if tag == 5: return [self.value(x,internal) for x in children]
        require(tag==6, 'object type')
        result={}; prev=None
        for ref in children:
            field_tag, pair = self.node(ref)
            require(field_tag==0x14 and len(pair)==64, 'object field body')
            key=self.value(pair[:32],internal)
            require(type(key) is str, 'object key must be string')
            key_bytes=key.encode('utf-8','strict')
            require(prev is None or prev < key_bytes, 'object key ordering/duplicate')
            prev=key_bytes
            result[key]=self.value(pair[32:],internal)
        return result

    def ref(self, desc):
        require(type(desc) is dict and set(desc)=={'format','root','kind','extent'}, 'ContentRef exact fields')
        require(desc['format']=='s-content/1' and type(desc['root']) is str and desc['root'].startswith('sc1-') and HEX.fullmatch(desc['root'][4:]), 'ContentRef format/root')
        root=bytes.fromhex(desc['root'][4:]); tag,body=self.node(root)
        kinds={0:'null',1:'bool',2:'bool',3:'string',4:'number',5:'array',6:'object',7:'json_text',8:'template_batch'}
        require(tag in kinds and kinds[tag]==desc['kind'], 'ContentRef kind')
        extent = None if tag in (0,1,2,7,8) else str(u64(body[:8]))
        require(desc['extent']==extent, 'ContentRef extent')
        return root

    def reachable(self, root):
        """Strictly parse every node in the logical value DAG from one root."""
        seen=set();seq_seen=set()
        def visit_seq(d,n,visit_child):
            tag,body=self.node(d)
            require((n==0 and tag==0x11 and body==b'') or
                    (n==1 and tag==0x12 and len(body)==32) or
                    (n>1 and tag==0x13 and len(body)==72 and u64(body[:8])==n),'reachable M header')
            if (d,n) in seq_seen:return
            seq_seen.add((d,n));seen.add(d)
            if n==0:require(tag==0x11 and body==b'','reachable empty M');return
            if n==1:
                require(tag==0x12 and len(body)==32,'reachable leaf M')
                visit_child(body);return
            require(tag==0x13 and len(body)==72 and u64(body[:8])==n,'reachable branch M')
            k=1<<((n-1).bit_length()-1)
            visit_seq(body[8:40],k,visit_child);visit_seq(body[40:],n-k,visit_child)
        def visit_value(d,internal=False):
            tag,body=self.node(d)
            require(tag in VALUE_TAGS and (not internal or tag<=6) and tag!=8,'reachable value type')
            if d in seen:return
            seen.add(d)
            if tag in (0,1,2):require(body==b'','reachable scalar body');return
            if tag in (3,4):
                text=self.byte_string(body,tag==4)
                size=u64(body[:8]);count=(size+4095)//4096
                def chunk(d):
                    tt,bb=self.node(d);require(tt==0x10 and 0<len(bb)<=4096,'reachable byte chunk');seen.add(d)
                visit_seq(body[8:],count,chunk)
                require((type(text) is Number)==(tag==4),'reachable number token')
                return
            if tag==7:
                require(len(body)==32,'reachable JSON_TEXT width');visit_value(body,True);return
            require(len(body)==40,'reachable collection header')
            n=u64(body[:8])
            if tag==5:visit_seq(body[8:],n,lambda d:visit_value(d,internal))
            else:
                require(tag==6,'reachable object')
                keys=[]
                def field(d):
                    tt,bb=self.node(d);require(tt==0x14 and len(bb)==64,'reachable object field')
                    seen.add(d);key=self.value(bb[:32],internal)
                    require(type(key) is str,'reachable key string')
                    keys.append(key.encode('utf-8','strict'))
                    visit_value(bb[:32],internal);visit_value(bb[32:],internal)
                visit_seq(body[8:],n,field)
                require(keys==sorted(set(keys)),'reachable key order/duplicate')
        visit_value(root)
        return seen


def table(db,name):
    t=db['tables'][name]; cols=[x[1] for x in t['columns']]
    return [dict(zip(cols,row)) for row in t['rows']]


def frozen():
    h=hashlib.sha256()
    with FROZEN.open('rb') as inp:
        while chunk:=inp.read(1024*1024): h.update(chunk)
    require(h.hexdigest()==FROZEN_SHA, 'frozen fullDB SHA')
    # The export's SQL INTEGER cells are control metadata. Exact JSON number
    # tokens in the content graph are stored separately as tagged byte nodes.
    return json.loads(FROZEN.read_bytes())


def native():
    db=frozen(); rows=table(db,'s_content_nodes')
    native_rows=[[r['digest'],r['tag'],r['body']] for r in rows]
    store=Store(native_rows)
    require(store.validate_all_nodes()==47671,'all native node schemas')
    pubs={r['generation']:r for r in table(db,'s_content_publications')}
    require(len(pubs)==16 and set(pubs)==set(range(1,17)), 'publication generation set')
    p=pubs[16]; snapshot_root=bytes.fromhex(p['snapshot_root']['blob_hex'])
    snap=store.value(snapshot_root)
    return db,store,pubs,snap


def source_check(db,store,snap):
    """Cross all six families with independent SQL column values and raw source."""
    counts={k:len(snap[k]) for k in ('objects','withdrawn_objects','witnesses','relations','observations','raw_history')}
    require(counts=={'objects':65,'withdrawn_objects':82,'witnesses':81,'relations':199,'observations':29,'raw_history':16},'native six-family counts')
    records=snap['objects']+snap['withdrawn_objects']
    by_object={(r['object_id'],r['object_revision']):r for r in records}
    require(len(by_object)==len(records),'object ID/revision duplicate')
    pubs={r['generation']:r['batch_id'] for r in table(db,'s_content_publications')}
    raw_by_id={(r['identity_key'],r['revision']):r for r in table(db,'raw_events')}
    raw_by_coord={r['source_coord']:r for r in snap['raw_history']}
    require(len(raw_by_coord)==16 and len(raw_by_id)==16,'raw source uniqueness')
    raw_ohlc={(r['identity_key'],r['revision']):r for r in table(db,'raw_ohlc')}
    ref_checks=0;coord_checks=0;bars_checked=0;json_text_checks=0;cc006_numeric_ref_fields=0
    def public_cc006_refs(v):
        nonlocal cc006_numeric_ref_fields
        if type(v) is list:return [public_cc006_refs(q) for q in v]
        if type(v) is dict:
            out={}
            for k,q in v.items():
                if k in ('revision','seq','merged_index') and type(q) is Number:
                    require(re.fullmatch(r'0|[1-9][0-9]*',q) is not None,'CC006 source integer token')
                    out[k]=str(q);cc006_numeric_ref_fields+=1
                else:out[k]=public_cc006_refs(q)
            return out
        return v
    for r in snap['raw_history']:
        source=raw_by_id[(r['identity_key'],int(r['revision']))]
        shape=raw_ohlc[(r['identity_key'],int(r['revision']))]
        for k,v in source.items():
            if k=='source_coord': require(same(r[k],v),'raw source coord')
            elif k in r: require(same(r[k],str(v) if k in ('revision','seq') else v), 'raw SQL field '+k)
        require(r['schema_revision']==shape['schema_revision'],'raw OHLC schema')
        for k in ('open','high','low','close'): require(same(r[k],shape[k]),'raw OHLC '+k)
        raw_text=parse_json(r['raw_text'])
        require(b''.join(render(raw_text)).decode()==r['raw_text'],'raw_text original canonical token/escape')
        # raw_text is opaque in graph; we only verify its existing source bytes.
    def source_ref_walk(v):
        nonlocal ref_checks,coord_checks
        if type(v) is list:
            for q in v:source_ref_walk(q)
        elif type(v) is dict:
            if {'identity_key','revision','source_coord'}<=set(v):
                key=(v['identity_key'],int(v['revision']));require(key in raw_by_id,'missing raw revision')
                source=raw_by_id[key];raw=raw_by_coord[v['source_coord']]
                require(raw['identity_key']==v['identity_key'] and raw['revision']==v['revision'],'source coordinate/revision mapping')
                for k in ('event_id','identity_key','input_revision','payload_hash','receipt_id','revision','seq','source_coord'):
                    if k in v:require(same(v[k],str(source[k]) if k in ('revision','seq') else source[k]),'source ref '+k)
                ref_checks+=1
            if 'source_coords' in v and type(v['source_coords']) is list:
                for coord in v['source_coords']:
                    require(type(coord) is str and coord in raw_by_coord,'source_coords reachability')
                    coord_checks+=1
            for q in v.values():source_ref_walk(q)
    for row in table(db,'s_content_objects'):
        key=(row['object_id'],str(row['object_revision']));require(key in by_object,'SQL object absent snapshot')
        actual=by_object[key]
        birth=store.value(bytes.fromhex(row['birth_wire_root']['blob_hex']))
        generated=store.value(bytes.fromhex(row['generated_root']['blob_hex']))
        for k in birth:
            if k in ('lifecycle','withdrawn_generation','withdrawal_reason','superseded_by'):continue
            require(k in actual and same(actual[k],birth[k]),'object birth field '+k)
        for k in ('object_id','kind','object_revision','batch_id','first_known_generation','first_known_cut','published_generation','withdrawn_generation','withdrawal_reason','superseded_by'):
            value=row[k]
            if k in ('object_revision','first_known_generation','published_generation','withdrawn_generation') and value is not None:value=str(value)
            require(k in actual and same(actual[k],value),'object SQL metadata '+k)
        for k in ('window_start','window_mid','window_end'):
            if row[k] is not None:require(same(actual[k],str(row[k])),'object SQL window '+k)
        if row['fact_key'] is not None:require(same(actual['fact_key'],row['fact_key']),'SQL fact key')
        require(actual['batch_id']==pubs[int(actual['published_generation'])],'object published batch')
        if 'payload_json' in generated:
            for old,new in (('payload_json','payload'),('input_refs_json','input_refs'),('source_coords_json','source_coords')):
                require(type(generated[old]) is JsonText,'declared JSON_TEXT tag '+old)
                require(new in actual and same(generated[old].inner,actual[new]),'JSON_TEXT inner '+old)
                json_text_checks+=1
        elif 'comparisons_json' in generated:
            require(same(parse_json(generated['comparisons_json']),actual['comparisons']),'CC-006 comparisons JSON')
            for old,new in (('input_refs_json','input_refs'),('source_coords_json','source_coords')):
                require(type(generated[old]) is JsonText,'CC-006 JSON_TEXT tag '+old)
                normalized=public_cc006_refs(generated[old].inner) if old=='input_refs_json' else generated[old].inner
                require(same(normalized,actual[new]),'CC-006 JSON_TEXT '+old)
                json_text_checks+=1
        else:raise ValueError('object generated schema unknown')
        source_ref_walk(actual)
    require(len(by_object)==len(table(db,'s_content_objects')),'object table count')
    for fam,tab,col,key in (
        ('witnesses','s_content_witnesses','wire_root','witness_id'),
        ('relations','s_content_relations','generated_root','relation_id'),
        ('observations','s_content_observations','birth_wire_root','observation_id')):
        records=snap[fam]
        if fam=='relations':
            by_key={(r['subject'],r['relation_type'],r['object']):r for r in records}
        else:by_key={r[key]:r for r in records}
        require(len(by_key)==len(records)==len(table(db,tab)),fam+' table cardinality')
        for row in table(db,tab):
            value=store.value(bytes.fromhex(row[col]['blob_hex']))
            lookup=(row['subject'],row['relation_type'],row['object']) if fam=='relations' else row[key]
            require(lookup in by_key and same(by_key[lookup],value),fam+' native row body')
            for name in row:
                if name in ('generated_root','wire_root','birth_wire_root'):continue
                if name=='merged_index': require(same(value['merged_source_index'],str(row[name])),'witness merged_index')
                elif name in value:
                    v=str(row[name]) if name in ('slot','merged_source_index') and row[name] is not None else row[name]
                    require(same(value[name],v),fam+' SQL metadata '+name)
            if fam=='witnesses':
                require(value['object_id'] in {r['object_id'] for r in snap['objects']+snap['withdrawn_objects']},'witness object link')
                require(len(value['raw_bars'])>0,'empty witness raw bars')
                for bar in value['raw_bars']:
                    require(same(bar,raw_by_coord[bar['source_coord']]),'witness full raw bar')
                    bars_checked+=1
                source_ref_walk(value['raw_bars'])
            elif fam=='observations':source_ref_walk(value)
    return {'families':counts,'sql_objects':147,'sql_witnesses':81,'sql_relations':199,'sql_observations':29,
            'raw_revisions':16,'raw_bars_in_witnesses_checked':bars_checked,'source_ref_occurrences_checked':ref_checks,
            'source_coord_occurrences_checked':coord_checks,'json_text_views_checked':json_text_checks,
            'cc006_declared_numeric_ref_fields_stringified_for_public_wire':cc006_numeric_ref_fields}


def catalog_check(db,snap,catalog):
    require(type(catalog) is dict and type(catalog.get('items')) is list,'Q catalog body')
    require(set(catalog)=={'session_id','generation','structure_cut','catalog_revision','index_frontier',
                           'input_frontier','seq_range','catalog_run_status','catalog_evidence','scope','items','counts'},
            'Q catalog exact header')
    for key in ('session_id','structure_cut','catalog_revision','index_frontier','input_frontier',
                'seq_range','catalog_run_status','catalog_evidence','scope'):
        require(same(catalog[key],snap[key]),'catalog native snapshot '+key)
    require(catalog['generation']==snap['generation'],'catalog native generation')
    rows=sorted(table(db,'catalog'),key=lambda r:r['catalog_id'].encode('utf-8'))
    require(len(rows)==len(catalog['items'])==116,'catalog table/item count')
    axes=snap['catalog_evidence']['axes'];statuses={'implemented':0,'not_implemented':0,'run':0,'not_run':0}
    for row,item in zip(rows,catalog['items']):
        require(set(item)=={'id','kind','title','domain','branches','implementation_status','proof_status','run_status','evidence'},'catalog item exact')
        for source_key,wire_key in (('catalog_id','id'),('kind','kind'),('title','title'),('domain','domain')):
            require(same(row[source_key],item[wire_key]),'catalog SQL '+source_key)
        branches=parse_json(row['branches_json'])
        def wire_numbers(v):
            if type(v) is Number and re.fullmatch(r'-?(?:0|[1-9][0-9]*)',v):return str(v)
            if type(v) is list:return [wire_numbers(x) for x in v]
            if type(v) is dict:return {k:wire_numbers(q) for k,q in v.items()}
            return v
        require(same(item['branches'],wire_numbers(branches)),'catalog branches SQL original')
        axis=axes.get(item['id'])
        status=axis if axis is not None else {'impl_status':'not_implemented','proof_status':'not_proved','run_status':'not_run','evidence':{}}
        for a,b in (('impl_status','implementation_status'),('proof_status','proof_status'),('run_status','run_status'),('evidence','evidence')):
            require(same(item[b],status[a]),'catalog axis evidence '+a)
        if item['implementation_status'] in statuses:statuses[item['implementation_status']]+=1
        if item['run_status'] in statuses:statuses[item['run_status']]+=1
    require(same(catalog['counts'],{k:str(v) for k,v in statuses.items()}),'catalog status counts')
    return {'sql_catalog_items_checked':116,'catalog_axis_entries_checked':len(axes),'catalog_status_counts':statuses}


def write(name,value):
    path=HERE/name
    path.write_text(json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':'))+'\n',encoding='utf-8')


def ref_for(value,store=None):
    if type(value) is Ref:
        require(store is not None,'store required for Ref header')
        return ref_for_root(value.root,store.value(value.root))
    root=v_digest(value)
    return ref_for_root(root,value)


def ref_for_root(root,value):
    if type(value) is Ref: raise ValueError('unresolved Ref')
    if value is None:kind,extent='null',None
    elif type(value) is bool:kind,extent='bool',None
    elif type(value) is Number:kind,extent='number',str(len(value.encode('ascii')))
    elif type(value) is str:kind,extent='string',str(len(value.encode('utf-8','strict')))
    elif type(value) is JsonText:kind,extent='json_text',None
    elif type(value) is list:kind,extent='array',str(len(value))
    elif type(value) is dict:kind,extent='object',str(len(value))
    else:raise ValueError('unsupported ref value')
    return {'format':'s-content/1','root':'sc1-'+root.hex(),'kind':kind,'extent':extent}


def as_int(s,allow_negative=False):
    require(type(s) is str and re.fullmatch(r'(?:0|[1-9][0-9]*|-1)',s) is not None,'canonical integer text')
    n=int(s);require(allow_negative or n>=0,'nonnegative integer')
    return n


def record_sort(family,r):
    if family=='objects':
        order=(1,r['kind'],r['fact_key'],r['object_id'],as_int(r['object_revision'])) if 'fact_key' in r else (0,as_int(r['window_start']),as_int(r['window_mid']),as_int(r['window_end']),r['object_id'],as_int(r['object_revision']))
        return order,[r['object_id']]
    if family=='withdrawn_objects':return (as_int(r['withdrawn_generation']),r['object_id'],as_int(r['object_revision'])),[r['object_id']]
    if family=='witnesses':return (r['object_id'],as_int(r['slot']),r['witness_id']),[r['witness_id']]
    if family=='relations':
        key=[r[k] for k in ('subject','relation_type','object')]
        return tuple(key),key
    if family=='observations':return (r['kind'],-1 if r['window_start'] is None else as_int(r['window_start']),r['observation_id']),[r['observation_id']]
    if family=='raw_history':return (as_int(r['seq']),r['identity_key'],as_int(r['revision'])),[r['identity_key'],r['revision']]
    raise ValueError('unknown family')


def logical_projection(snapshot,page,snap,store):
    """Use native S16 facts for row values; Q descriptors only choose order/headers."""
    rows=snapshot['rows'];require(len(rows)==472,'descriptor row count')
    families=('objects','withdrawn_objects','witnesses','relations','observations','raw_history')
    expected=[]
    for fam in families:
        ordered=sorted(snap[fam],key=lambda r:record_sort(fam,r)[0])
        expected.extend((fam,record_sort(fam,r)[1],r) for r in ordered)
    require(len(expected)==len(rows),'expected order count')
    for i,(fam,key,record) in enumerate(expected):
        descriptor=rows[i]
        require(set(descriptor)=={'family','key','record'} and descriptor['family']==fam and same(descriptor['key'],key),'row order/key '+str(i))
        root=store.ref(descriptor['record'])
        require(root==v_digest(record),'row ContentRef root/source '+str(i))
        require(same(store.value(root),record),'row native value '+str(i))
    require(same(page['fixed_cut'],snapshot['fixed_cut']) and page['cut_projection_digest']==snapshot['cut_projection_digest'],'first page/complete header')
    require(same(page['rows'],rows[:len(page['rows'])]),'first page/complete rows')
    fixed=dict(snapshot['fixed_cut'])
    require(set(fixed)=={'session_id','session_generation','structure_cut','cut_generation','catalog_revision','index_frontier','profile_id','profile_hash','input_frontier','seq_range','catalog_run_status','catalog_evidence','scope','rule_revision','history_mode','as_of_generation'},'fixed_cut exact')
    require(fixed['cut_generation']=='16' and fixed['structure_cut']==snap['structure_cut'] and fixed['index_frontier']==snap['index_frontier'],'fixed cut/native publication')
    evidence=store.ref(fixed['catalog_evidence']);fixed['catalog_evidence']=Ref(evidence)
    require(page['catalog']['kind']=='object' and type(page['catalog']['extent']) is str,'Q catalog descriptor header')
    catalog_ref=descriptor_root(page['catalog'])
    counts=page['counts'];require(set(counts)==set(families)|{'total'},'counts exact')
    for fam in families: require(counts[fam]==str(len(snap[fam])),'count '+fam)
    require(counts['total']=='472' and same(page['scope'],snapshot['fixed_cut']['scope']) and page['omissions']==[] and page['order_version']=='s-record-order/2','projection header')
    logical={'fixed_cut':fixed,'catalog':Ref(catalog_ref),
             'rows':[{'family':r['family'],'key':r['key'],'record':Ref(store.ref(r['record']))} for r in rows],
             'counts':counts,'scope':page['scope'],'omissions':page['omissions'],'order_version':page['order_version']}
    root=v_digest(logical)
    require(snapshot['projection_ref']==ref_for_root(root,{'fixed_cut':0,'catalog':0,'rows':0,'counts':0,'scope':0,'omissions':0,'order_version':0}),'projection_ref')
    require(snapshot['cut_projection_digest']=='qcut3-'+f(0x22,root).hex(),'qcut3 projection root')
    return logical,expected


def resolve_value(value,store):
    return store.value(value.root) if type(value) is Ref else value


def path_value(logical,path,store):
    v=logical
    for step in path:
        v=resolve_value(v,store)
        require(type(step) is dict and len(step)==1,'content path step exact')
        if 'field' in step:
            require(type(v) is dict and type(step['field']) is str and step['field'] in v,'content field path')
            v=v[step['field']]
        elif 'index' in step:
            i=as_int(step['index']);require(type(v) is list and i<len(v),'content index path')
            v=v[i]
        elif 'json_text' in step:
            require(step['json_text'] is True and type(v) is JsonText,'content json_text path')
            v=v.inner
        else:raise ValueError('unknown path step')
    return resolve_value(v,store)


def range_value(value,unit,start,end,store):
    if unit=='items' and type(value) is list:
        return [ref_for(v,store) for v in value[start:end]],len(value)
    if unit=='items' and type(value) is dict:
        keys=sorted(value,key=lambda x:x.encode('utf-8','strict'))
        return [{'key':k,'value_ref':ref_for(value[k],store)} for k in keys[start:end]],len(keys)
    if unit=='bytes' and type(value) in (str,Number):
        raw=value.encode('utf-8' if type(value) is str else 'ascii')
        return raw[start:end].hex(),len(raw)
    if unit=='value' and (value is None or type(value) in (bool,JsonText)):
        return ([ref_for(value,store)] if start==0 else []),1
    raise ValueError('unit/value mismatch')


def render_edges(value,store):
    if type(value) is Ref:
        yield from render_edges(store.value(value.root),store)
    elif type(value) is list:
        yield b'['
        for i,v in enumerate(value):
            if i:yield b','
            yield from render_edges(v,store)
        yield b']'
    elif type(value) is dict:
        yield b'{'
        for i,k in enumerate(sorted(value,key=lambda x:x.encode('utf-8','strict'))):
            if i:yield b','
            yield escaped(k);yield b':'
            yield from render_edges(value[k],store)
        yield b'}'
    else:yield from render(value)


def hash_edges(value,store):
    h=hashlib.sha256();size=0
    for part in render_edges(value,store):h.update(part);size+=len(part)
    return h.hexdigest(),size


def http_split(wire):
    at=wire.find(b'\r\n\r\n');require(0<=at<=16384,'HTTP header boundary')
    head=wire[:at];body=wire[at+4:]
    return head,body


def carrier_records(folder):
    """Independent Q3R2 parser. No calls/imports to archive_v2.pack/unpack."""
    folder=Path(folder);manifest=folder/'archive-manifest.jsonl'
    require(manifest.is_file(),'archive manifest missing')
    lines=manifest.read_bytes().splitlines(keepends=True)
    require(lines and all(line.endswith(b'\n') for line in lines),'manifest lines/terminal newline')
    expected_files=set();serial=0
    for chunk_index,line in enumerate(lines,1):
        raw_line=line[:-1];entry=parse_json(raw_line)
        require(b''.join(render(entry))==raw_line,'manifest canonical JSON')
        require(set(entry)=={'chunk','file','format','first_serial','last_serial','records','uncompressed_bytes','uncompressed_sha256','compressed_bytes','compressed_sha256'},'manifest exact fields')
        name=f'archive-chunk-{chunk_index:06}.zst';expected_files.add(name)
        require(entry['format']==ARCHIVE_FORMAT and entry['file']==name,'manifest format/file')
        for k in ('chunk','first_serial','last_serial','records','uncompressed_bytes','compressed_bytes'):
            require(type(entry[k]) is Number and re.fullmatch(r'0|[1-9][0-9]*',entry[k]),'manifest integer '+k)
        require(int(entry['chunk'])==chunk_index and int(entry['first_serial'])==serial+1,'manifest chunk/serial continuity')
        require(0<int(entry['records'])<=256,'manifest bounded records')
        file=folder/name;compressed=file.read_bytes()
        require(len(compressed)==int(entry['compressed_bytes']) and hashlib.sha256(compressed).hexdigest()==entry['compressed_sha256'],'compressed frame SHA/size')
        require(compressed.startswith(bytes.fromhex('28b52ffd')),'zstd frame magic')
        decoded=subprocess.run(['zstd','-q','-dc'],input=compressed,capture_output=True,check=True,timeout=30).stdout
        require(len(decoded)==int(entry['uncompressed_bytes']) and len(decoded)<=MAX_CHUNK and hashlib.sha256(decoded).hexdigest()==entry['uncompressed_sha256'],'decompressed frame SHA/size')
        off=0;within=0
        while off<len(decoded):
            require(len(decoded)-off>=RECORD_HEADER.size,'truncated Q3R2 header')
            magic,ml,ql,rl,crc,qsha,rsha=RECORD_HEADER.unpack_from(decoded,off)
            require(magic==b'Q3R2' and ml<65536 and ql+rl<=MAX_RECORD,'Q3R2 magic/length bound')
            end=off+RECORD_HEADER.size+ml+ql+rl
            require(end<=len(decoded),'truncated Q3R2 payload')
            at=off+RECORD_HEADER.size
            meta_bytes=decoded[at:at+ml];req=decoded[at+ml:at+ml+ql];resp=decoded[at+ml+ql:end]
            require((zlib.crc32(req+resp)&0xffffffff)==crc and hashlib.sha256(req).digest()==qsha and hashlib.sha256(resp).digest()==rsha,'Q3R2 CRC/SHA')
            meta=parse_json(meta_bytes)
            require(b''.join(render(meta))==meta_bytes and set(meta)=={'serial','complete','error','op','code'},'Q3R2 canonical metadata exact')
            require(type(meta['serial']) is Number and re.fullmatch(r'[1-9][0-9]*',meta['serial']),'Q3R2 serial token')
            serial+=1;within+=1
            require(int(meta['serial'])==serial,'Q3R2 serial continuity')
            require(type(meta['complete']) is bool and meta['op']=='content','Q3R2 complete/op')
            request_head,request_body=http_split(req)
            require(request_head.startswith(b'POST /api/v3/content HTTP/1.0\r\n'),'Q3R2 request endpoint')
            request_envelope=parse_json(request_body)
            require(request_envelope['message_id']=='fullbody-'+str(serial),'Q3R2 request/serial identity')
            if meta['complete']:
                require(meta['error'] is None and type(meta['code']) is Number and re.fullmatch(r'[1-5][0-9][0-9]',meta['code']),'Q3R2 complete metadata/status domain')
                response_head,response_body=http_split(resp)
                code=int(response_head.split(b'\r\n')[0].split()[1])
                require(code==int(meta['code']),'Q3R2 metadata HTTP status')
                match=re.search(rb'(?i)\r\ncontent-length:\s*([0-9]+)',b'\r\n'+response_head)
                if match:require(int(match.group(1))==len(response_body),'Q3R2 response Content-Length')
            else:
                code=meta['code']
                require(type(meta['error']) is str and meta['error'] and
                        (code is None or (type(code) is Number and re.fullmatch(r'[1-5][0-9][0-9]',code))),
                        'Q3R2 partial metadata/status domain')
            yield meta,req,resp
            off=end
        require(off==len(decoded) and within==int(entry['records']) and serial==int(entry['last_serial']),'Q3R2 frame exact end/record count')
    found={p.name for p in folder.glob('archive-chunk-*.zst')}
    require(found==expected_files and not list(folder.glob('archive-chunk-*.part')),'archive chunk set/uncommitted part')


def carrier_prefix_check(folder):
    complete=partial=records=wire_bytes=0;combined=hashlib.sha256()
    old=ROOT/'.scratch/issue1394-q3-full-body-20261001/raw-http.jsonl.gz'
    with gzip.open(old,'rb') as legacy:
        for meta,req,resp in carrier_records(folder):
            line=legacy.readline()
            require(line.endswith(b'\n'),'legacy readable line terminal')
            original=parse_json(line)
            require(int(original['serial'])==int(meta['serial']) and original['op']==meta['op'] and int(original['code'])==int(meta['code']),'legacy metadata/serial')
            require(base64.b64decode(original['request_base64'],validate=True)==req and base64.b64decode(original['response_base64'],validate=True)==resp,'Q3R2 vs legacy original wire')
            require(hashlib.sha256(req).hexdigest()==original['request_sha256'] and hashlib.sha256(resp).hexdigest()==original['response_wire_sha256'],'legacy independent wire SHA')
            records+=1;complete+=int(meta['complete']);partial+=int(not meta['complete']);wire_bytes+=len(req)+len(resp)
            combined.update(struct.pack('>I',len(req)));combined.update(req)
            combined.update(struct.pack('>I',len(resp)));combined.update(resp)
        try:tail=legacy.readline()
        except EOFError:tail=b''
        require(tail==b'','legacy archive lacks additional complete line')
    require(records==1835 and complete==1835 and partial==0,'prefix readable record count')
    result={'status':'PASS_Q3R2_PREFIX_BYTES_ONLY','scope':'failed first-run readable prefix; no business closure',
            'records':records,'complete':complete,'partial':partial,'request_plus_response_bytes':wire_bytes,
            'independent_wire_stream_sha256':combined.hexdigest(),'manifest_sha256':hashlib.sha256((Path(folder)/'archive-manifest.jsonl').read_bytes()).hexdigest(),
            'legacy_failed_gzip_sha256':hashlib.sha256(old.read_bytes()).hexdigest(),'every_carrier_wire_equals_legacy_readable_original':True,
            'semantic_oracle_base_sha256':OLD_ORACLE_SHA,'carrier_oracle_source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'format_difference':'Q3R2 independent zstd chunks and manifest replace failed gzip JSONL; native/content semantics unchanged',
            'full_body_source_closure':'UNMEASURED','client_atomic_install':'NOT_ATTEMPTED','client_physical_allocation':'UNMEASURED'}
    write('oracle-v2-prefix-report.json',result)
    return result


def actual_check(db,store,snap,source_checks):
    cfg=json.loads((HERE/'Controller.json').read_bytes(),object_pairs_hook=pairs_unique)
    runner=json.loads((HERE/'RESULT.json').read_bytes(),object_pairs_hook=pairs_unique)
    terminal=json.loads((HERE/'collector-terminal.json').read_bytes(),object_pairs_hook=pairs_unique)
    cleanup=json.loads((HERE/'CLEANUP.json').read_bytes(),object_pairs_hook=pairs_unique)
    require(runner['status']=='PASS_JS_FULL_BODY_PROOF' and terminal['status']=='PASS_ARCHIVE_FLUSHED' and terminal['archive_closed'] is True,'actual fixture/collector terminal')
    require(cleanup['same_q_popen_reaped'] is True and cleanup['same_node_popen_reaped'] is True and cleanup['original_source_unchanged'] is True,'same-handle cleanup/source')
    snapshot=json.loads((HERE/'snapshot.complete.json').read_bytes(),object_pairs_hook=pairs_unique)
    page=json.loads((HERE/'snapshot-0.response.json').read_bytes(),object_pairs_hook=pairs_unique)['payload']
    require(snapshot['pages']==16,'snapshot page count')
    logical,expected=logical_projection(snapshot,page,snap,store)
    # Q-created catalog/projection/row wrapper nodes do not live in the S DB.
    # Recover them from original HTTP bytes, check every F, then demand that
    # every transmitted node belongs to the independently derived projection.
    native_roots=set(store.nodes);proof_nodes={}
    for meta,request,response in carrier_records(HERE):
            require(meta['complete'] is True,'partial HTTP response in proof stream')
            _,raw_body=http_split(response)
            payload=parse_json(raw_body)['payload']
            for n in payload['nodes']:
                require(type(n) is dict and set(n)=={'root','tag','body_hex'},'proof node exact')
                require(type(n['tag']) is str and re.fullmatch(r'[0-9a-f]{2}',n['tag']) and type(n['body_hex']) is str and re.fullmatch(r'(?:[0-9a-f]{2})*',n['body_hex']) and type(n['root']) is str and n['root'].startswith('sc1-') and HEX.fullmatch(n['root'][4:]),'proof node encoding')
                root=bytes.fromhex(n['root'][4:]);item=(int(n['tag'],16),bytes.fromhex(n['body_hex']))
                require(root==f(*item),'proof node F')
                require(root not in store.nodes or store.nodes[root]==item,'HTTP/native node original bytes')
                require(root not in proof_nodes or proof_nodes[root]==item,'same root conflicting HTTP originals')
                proof_nodes[root]=item
    store.nodes.update(proof_nodes)
    projection_root=descriptor_root(snapshot['projection_ref'])
    reachable=store.reachable(projection_root)
    require(set(proof_nodes)<=reachable,'orphan proof node outside anchored projection DAG')
    require(store.ref(snapshot['projection_ref'])==projection_root,'actual projection root header')
    # Decode the complete transport graph with independent Python semantics.
    projection_value=store.value(projection_root)
    require(set(projection_value)=={'fixed_cut','catalog','rows','counts','scope','omissions','order_version'},'decoded projection exact')
    catalog_source=catalog_check(db,snap,projection_value['catalog'])
    for key in ('session_id','structure_cut','catalog_revision','index_frontier','input_frontier','seq_range','catalog_run_status','catalog_evidence','scope'):
        require(same(projection_value['catalog'][key],projection_value['fixed_cut'][key]),'catalog/fixed header '+key)
    require(same(projection_value['catalog']['generation'],projection_value['fixed_cut']['cut_generation']),'catalog generation header')
    projection_hash,projection_bytes=body_hash(projection_value)
    require(hash_edges(logical,store)==(projection_hash,projection_bytes),'native-derived logical projection vs HTTP graph body')
    collector=json.loads((HERE/'body-summary.json').read_bytes(),object_pairs_hook=pairs_unique)
    require(collector['rows']==472 and collector['projection_ref']==snapshot['projection_ref'],'collector root/count')
    require(collector['projection_body_sha256']==projection_hash and collector['projection_body_bytes']==projection_bytes,'full projection canonical bytes')
    rows=(HERE/'row-digests.jsonl').read_text(encoding='utf-8').splitlines()
    require(len(rows)==472,'collector row digest lines')
    output=[]
    for i,(fam,key,value) in enumerate(expected):
        got=json.loads(rows[i],object_pairs_hook=pairs_unique)
        sha,size=body_hash(value)
        require(got['ordinal']==i and got['family']==fam and same(got['row_key'],key),'collector row identity '+str(i))
        require(got['record_ref']==snapshot['rows'][i]['record'] and got['body_sha256']==sha and got['body_bytes']==size,'record canonical digest '+str(i))
        output.append({'ordinal':i,'family':fam,'row_key':key,'record_ref':got['record_ref'],
                       'body_sha256':sha,'body_bytes':size})
    anchor={'kind':'projection','fixed_cut':snapshot['fixed_cut'],'cut_projection_digest':snapshot['cut_projection_digest'],'projection_ref':snapshot['projection_ref']}
    node_roots=set();node_conflicts={};requests=0;response_wire_bytes=0;response_body_bytes=0
    prior=None;range_checks=0;native_proof_nodes=0;temporary_q_nodes=0
    for meta,request,response in carrier_records(HERE):
            requests+=1
            require(int(meta['serial'])==requests and meta['complete'] is True and meta['op']=='content' and int(meta['code'])==200,'HTTP log serial/op/status')
            request_head,request_body=http_split(request);response_head,response_body=http_split(response)
            require(request_head.startswith(b'POST /api/v3/content HTTP/1.0\r\n'),'content endpoint')
            require(response_head.split(b'\r\n')[0].split()[1]==b'200','HTTP status line')
            req=parse_json(request_body);resp=parse_json(response_body)
            require(set(req)=={'schema_revision','session_id','session_generation','source_namespace','source_epoch','message_id','producer_id','producer_epoch','payload_hash','causal_refs','payload'},'request envelope exact')
            require(set(resp)==set(req),'response envelope exact')
            require(req['payload_hash']==hashlib.sha256(b''.join(render(req['payload']))).hexdigest(),'request payload hash')
            require(resp['payload_hash']==hashlib.sha256(b''.join(render(resp['payload']))).hexdigest(),'response payload hash')
            require(req['schema_revision']=='s-session/3' and req['source_namespace']=='formal-fullbody-s16' and req['source_epoch']=='1' and req['producer_id']=='controller-acceptance' and req['producer_epoch']=='1','request source identity')
            require(req['session_id']==cfg['session_id'] and req['session_generation']==cfg['session_generation'],'request session identity')
            require(resp['source_namespace']=='s-observe/replies' and resp['source_epoch']==cfg['query_epoch'] and resp['producer_id']=='Q:'+cfg['session_id'] and resp['producer_epoch']==cfg['query_epoch'],'response source identity')
            require(resp['message_id']=='reply-'+hashlib.sha256(b''.join(render([req['source_namespace'],req['source_epoch'],req['message_id'],resp['payload_hash']]))).hexdigest(),'response message identity')
            require(same(resp['causal_refs'],[{k:req[k] for k in ('source_namespace','source_epoch','message_id','payload_hash')}]),'response causal refs')
            for field in ('schema_revision','session_id','session_generation'):require(same(req[field],resp[field]),'envelope '+field)
            content_req=req['payload'];content=resp['payload']
            require(content_req['op']=='content' and content['ok'] is True,'content op/result')
            if 'content_token' in content_req:
                require(set(content_req)=={'op','content_token'} and prior is not None and same(content_req['content_token'],prior),'content token continuity')
                token=content_req['content_token'];require(token['token_schema']=='s-content-token/1','content token schema')
                effective=token
            else:
                require(set(content_req)=={'op','anchor','path','unit','offset','limit'},'first content request exact')
                effective=content_req
            require(same(effective['anchor'],anchor) and same(content['anchor'],anchor),'projection anchor')
            require(set(content)=={'anchor','path','target_ref','unit','offset','end','total','items','nodes','done','next_token','validated_capture_digest','ok'},'content response exact')
            path=effective['path'];target=path_value(logical,path,store)
            target_ref=ref_for(target,store)
            require(same(content['path'],path) and same(content['target_ref'],target_ref),'content target/path root')
            unit=effective['unit'];start=as_int(effective['offset']);limit=as_int(effective['limit'])
            require(limit>0 and same(content['unit'],unit) and content['offset']==str(start),'content unit/offset')
            _,total=range_value(target,unit,0,0,store)
            require(start<=total,'range start');end=min(start+limit,total)
            items,_=range_value(target,unit,start,end,store)
            require(content['end']==str(end) and content['total']==str(total) and content['done'] is (end==total),'range end/total/done')
            require(same(content['items'],items),'range items/canonical source')
            prior=content['next_token']
            if end<total:
                require(type(prior) is dict and set(prior)=={'token_schema','anchor','path','target_ref','unit','offset','limit','issued_capture_digest','token_checksum'},'next token exact')
                require(prior['token_schema']=='s-content-token/1' and prior['offset']==str(end) and prior['limit']==str(limit) and same(prior['anchor'],anchor) and same(prior['path'],path) and prior['unit']==unit and same(prior['target_ref'],target_ref),'next content token')
                unsigned={k:v for k,v in prior.items() if k!='token_checksum'}
                require(prior['token_checksum']==hashlib.sha256(b''.join(render(unsigned))).hexdigest(),'next token checksum')
            else:require(prior is None,'done next_token')
            local=set()
            for n in content['nodes']:
                require(set(n)=={'root','tag','body_hex'},'proof node exact')
                require(type(n['tag']) is str and re.fullmatch(r'[0-9a-f]{2}',n['tag']) and type(n['body_hex']) is str and re.fullmatch(r'(?:[0-9a-f]{2})*',n['body_hex']) and type(n['root']) is str and n['root'].startswith('sc1-') and HEX.fullmatch(n['root'][4:]),'proof node encoding')
                root=bytes.fromhex(n['root'][4:]);tag=int(n['tag'],16);body=bytes.fromhex(n['body_hex'])
                require(root==f(tag,body),'HTTP proof node F')
                require(n['root'] not in local,'same-response proof duplicate')
                local.add(n['root'])
                prior_node=node_conflicts.get(n['root'])
                require(prior_node is None or prior_node==(tag,body),'cross-response same-root collision')
                node_conflicts[n['root']]=(tag,body);node_roots.add(n['root'])
                if root in native_roots:
                    require(store.node(root)==(tag,body),'HTTP/native node bytes')
                    native_proof_nodes+=1
                else:temporary_q_nodes+=1
            require(snapshot['projection_ref']['root'] in local,'proof includes anchored projection root')
            require(target_ref['root'] in local,'proof includes target header')
            range_checks+=1;response_wire_bytes+=len(response);response_body_bytes+=len(response_body)
    require(requests>0 and requests==collector['content_requests'],'content request count')
    require(response_wire_bytes==collector['response_wire_bytes'] and response_body_bytes==collector['response_body_bytes'],'HTTP byte totals')
    roots=json.loads((HERE/'content-node-roots.json').read_bytes())
    require(set(roots)==node_roots and len(roots)==collector['unique_nodes'],'collector node set')
    require({bytes.fromhex(x[4:]) for x in node_roots}==set(proof_nodes),'raw proof node set both passes')
    manifest=HERE/'archive-manifest.jsonl'
    with (HERE/'oracle-v2-row-report.jsonl').open('x',encoding='utf-8') as out:
        for item in output:out.write(json.dumps(item,ensure_ascii=False,sort_keys=True,separators=(',',':'))+'\n')
    return {'status':'PASS_INDEPENDENT_S16_BODY_SOURCE_HTTP','frozen_fullDB_sha256':FROZEN_SHA,
            'semantic_oracle_base_sha256':OLD_ORACLE_SHA,'carrier_oracle_source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'carrier_format':ARCHIVE_FORMAT,'format_difference':'Q3R2 independent zstd chunks/manifest replace gzip JSONL; semantic/native code inherited from frozen base',
            'archive_chunks':len(manifest.read_bytes().splitlines()),'archive_manifest_sha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),
            'native_nodes_F_verified':store.count,'source_checks':source_checks,'catalog_source_checks':catalog_source,
            'projection_ref':snapshot['projection_ref'],'projection_body_sha256':projection_hash,
            'projection_body_bytes':projection_bytes,'row_body_digests_checked':472,'content_requests':requests,
            'range_responses_checked':range_checks,'unique_proof_nodes':len(node_roots),
            'proof_node_transmissions_native':native_proof_nodes,'proof_node_transmissions_q_temporary':temporary_q_nodes,
            'response_wire_bytes':response_wire_bytes,'response_body_bytes':response_body_bytes,
            'client_atomic_install':'NOT_ATTEMPTED','client_physical_allocation':'UNMEASURED','full_9011':'NOT_ATTEMPTED'}


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--actual',action='store_true'); ap.add_argument('--prefix-test',action='store_true'); args=ap.parse_args()
    require(not (args.actual and args.prefix_test),'choose one mode')
    if args.prefix_test:
        result=carrier_prefix_check(HERE/'prefix-test')
        print('prefix',result['status'],result['records'],flush=True)
        return
    require(hashlib.sha256((ROOT/'.scratch/issue1394-q3-full-body-20261001/independent_oracle.py').read_bytes()).hexdigest()==OLD_ORACLE_SHA,'old semantic oracle SHA drift')
    db,store,pubs,snap=native()
    checked=source_check(db,store,snap)
    print('native nodes',store.count,'source checks',checked,flush=True)
    write('oracle-v2-native-report.json',{'status':'PASS_NATIVE_F_AND_SOURCE','frozen_fullDB_sha256':FROZEN_SHA,'native_nodes_F_verified':store.count,'publication_generation':16,'snapshot_root':'sc1-'+pubs[16]['snapshot_root']['blob_hex'],'source_checks':checked,'semantic_oracle_base_sha256':OLD_ORACLE_SHA,'carrier_oracle_source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'full_body_source_closure':'UNMEASURED'})
    if args.actual:
        result=actual_check(db,store,snap,checked)
        write('oracle-v2-actual-report.json',result)
        (HERE/'oracle-v2-actual-report.md').write_text(
            '# #1394 S16 独立正文与来源核验\n\n'
            '状态：'+result['status']+'。冻结 fullDB、47,671 原生节点、六族 472 条记录、独立 SQL 来源、每条正文的规范字节、全部实际 HTTP 片段与 Q 投影根均逐项核对。\n\n'
            '实测：HTTP '+str(result['content_requests'])+' 次，唯一 proof 节点 '+str(result['unique_proof_nodes'])+'，响应 wire '+str(result['response_wire_bytes'])+' 字节；投影正文 '+str(result['projection_body_bytes'])+' 字节，SHA-256 '+result['projection_body_sha256']+'。详细字段见 oracle-v2-actual-report.json 和 oracle-v2-row-report.jsonl。\n\n'
            '本报告只证明此冻结 S16 与当前 Q3 全文／来源闭包；Client 原子安装、实际物理 allocation、完整 9011、产品前端 #1460 均未测。\n',encoding='utf-8')
        print('actual',result['status'],'requests',result['content_requests'],flush=True)


if __name__=='__main__': main()
