//! #1527：旧域真实根遍历，所有原件与缓存访问受同一 mixed 父账约束。
use super::*;
use crate::v3::{audit_budget::LegacyObserver,audit_store::NativeTree,business_json::MixedJsonError,native_shared::NativeArc};
type Text=NativeValue<String>;
type Cert=Option<NativeArc<cache::RootCert>>;
fn text(value:&str,work:&WorkScope)->Result<Text,MixedJsonError>{Text::try_from_str(value,Some(work)).map_err(MixedJsonError::Resource)}
fn formatted(args:fmt::Arguments<'_>,work:&WorkScope)->Result<Text,MixedJsonError>{Text::try_format(args,Some(work)).map_err(MixedJsonError::Resource)}
fn push<T>(v:&mut CodecVec<T>,item:T,work:&WorkScope)->Result<(),MixedJsonError>{
    v.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
    v.try_push(item).map_err(MixedJsonError::Resource)
}
fn put<K:Ord,V>(tree:&mut NativeTree<K,V>,key:K,value:V,work:&WorkScope)->Result<(),MixedJsonError>{
    tree.insert_admitted(key,value,&NativeAdmission(Some(work))).map(|_|()).map_err(MixedJsonError::Resource)
}
fn sql_text<'a>(row:&'a rusqlite::Row<'_>,column:usize)->rusqlite::Result<&'a str>{
    let value=row.get_ref(column)?;
    value.as_str().map_err(|error|match error{
        rusqlite::types::FromSqlError::InvalidType=>rusqlite::Error::InvalidColumnType(column,row.as_ref().column_name(column).unwrap_or("").to_string(),value.data_type()),
        other=>rusqlite::Error::FromSqlConversionFailure(column,value.data_type(),Box::new(other)),
    })
}
fn copy_header(header:&BatchControlHeader,work:&WorkScope)->Result<BatchControlHeader,MixedJsonError>{
    let copy=|item:&Option<NativeArc<SharedHeaderText>>|item.as_ref().map(|value|value.share_or_copy_mixed(work,|v,w|{
        Ok(SharedHeaderText{text:v.text.try_clone_mixed(w)?})
    })).transpose().map_err(MixedJsonError::Resource);
    Ok(BatchControlHeader{protocol_revision:copy(&header.protocol_revision)?,session_generation:copy(&header.session_generation)?,clock_plan_hash:copy(&header.clock_plan_hash)?,semantic_commit_ns:copy(&header.semantic_commit_ns)?})
}
fn parse_batch(bytes:&[u8],id:&str,work:&WorkScope)->Result<NativeArc<SharedPlainJson>,MixedJsonError>{
    let parsed=crate::v3::business_json::mixed_batch_json_owner(bytes,id,work)?;
    if !parsed.is_object(){return Err(format!("StorageUnavailable：batch {id} 规范字节非 JSON 对象").into());}
    NativeArc::try_new_shallow(SharedPlainJson{value:parsed},Some(work)).map_err(MixedJsonError::Resource)
}
fn batch_ref(conn:&Connection,reader:&mut evidence_store::Reader<'_>,id:&str,work:&WorkScope)->Result<(BatchRef,Cert),MixedJsonError>{
    if evidence_store::format_typed(conn,Some(work))?.graph(){
        let(blob,len)=read_batch_row_typed(conn,id,None,Some(work))?;
        if !blob.is_empty(){return Err(format!("StorageUnavailable：图域 batch `{id}` 仍带整块字节（与私有根混写），拒绝写入").into());}
        let key=formatted(format_args!("batch/{id}"),work)?;
        if let Some(proof)=reader.prove_root_typed(&key)?{
            verify_batch_root_binding(id,len,proof.declared_length,&proof.content_sha256)?;
            if let Some(header)=cache::batch_header_get(&key,id,len,&proof.root_cert,&proof.closure){
                return Ok((BatchRef::CertifiedHeader(copy_header(&header,work)?),Some(proof.root_cert)));
            }
            return Ok((BatchRef::PendingFull,Some(proof.root_cert)));
        }
    }
    Ok((BatchRef::PendingFull,None))
}
fn load_batch(conn:&Connection,reader:&mut evidence_store::Reader<'_>,id:&str,with_value:bool,work:&WorkScope)->Result<(NativeArc<[u8]>,Option<NativeArc<SharedPlainJson>>,Cert),MixedJsonError>{
    #[cfg(test)]cache::counter(|c|c.batch_full_reads+=1);
    let(blob,len,graph)=read_batch_storage_typed(conn,id,None,Some(work))?;
    let key=formatted(format_args!("batch/{id}"),work)?;
    if !graph{
        let bytes=NativeArc::<[u8]>::try_copy_slice(&blob,Some(work)).map_err(MixedJsonError::Resource)?;
        let digest=blob_digest(&blob);
        let value=if with_value{
            match cache::handle_get_blob(&key,&blob,&digest).and_then(|(_,value)|value){
                Some(value)=>Some(value.share_or_copy_mixed(work,|v,w|Ok(SharedPlainJson{value:v.value.try_clone_mixed(w)?})).map_err(MixedJsonError::Resource)?),
                None=>{
                    let value=parse_batch(&bytes,id,work)?;
                    cache::handle_put_blob_typed(&key,digest,bytes.clone(),Some(value.clone()),Some(work)).map_err(MixedJsonError::Resource)?;
                    Some(value)
                }
            }
        }else{
            cache::handle_put_blob_typed(&key,digest,bytes.clone(),None,Some(work)).map_err(MixedJsonError::Resource)?;
            None
        };
        return Ok((bytes,value,None));
    }
    let root=reader.read_root_typed(&key,with_value)?;
    #[cfg(test)]cache::counter(|c|if root.reused{c.batch_handle_hits+=1;}else{c.batch_expansions+=1;});
    verify_batch_root_binding(id,len,root.declared_length,&root.content_sha256)?;
    if let(Some(value),Some(cert))=(&root.value,&root.root_cert){
        cache::batch_header_put_typed(&key,id,len,cert,root.closure,BatchControlHeader::from_verified_batch_typed(value,Some(work)).map_err(MixedJsonError::Resource)?,Some(work)).map_err(MixedJsonError::Resource)?;
    }
    Ok((root.bytes,root.value,root.root_cert))
}
struct Reachable{
    batches:NativeTree<Text,BatchRef>,
    roots:NativeTree<Text,[u8;32]>,
    originals:cache::mixed::Originals,
    _capture:cache::CaptureGuard,
}

fn root_keys(conn:&Connection,work:&WorkScope)->Result<CodecVec<Text>,MixedJsonError>{
    let mut stmt=conn.prepare("SELECT root_key FROM s_evidence_roots ORDER BY root_key").map_err(|e|format!("StorageUnavailable：读私有根表失败：{e}"))?;
    let mut rows=stmt.query([]).map_err(|e|format!("StorageUnavailable：读私有根表失败：{e}"))?;
    let mut keys=CodecVec::default();
    while let Some(row)=rows.next().map_err(|e|format!("StorageUnavailable：私有根行损坏：{e}"))?{
        let key=sql_text(row,0).map_err(|e|format!("StorageUnavailable：私有根行损坏：{e}"))?;
        push(&mut keys,text(key,work)?,work)?;
    }
    Ok(keys)
}
fn capture(conn:&Connection,work:&WorkScope,observer:&dyn LegacyObserver)->Result<Reachable,MixedJsonError>{
    let mut ids=CodecVec::default();
    {
        let mut stmt=conn.prepare("SELECT d.index_frontier FROM structure_deltas d").map_err(|e|e.to_string())?;
        let mut rows=stmt.query([]).map_err(|e|e.to_string())?;
        while let Some(row)=rows.next().map_err(|e|e.to_string())?{
            push(&mut ids,text(sql_text(row,0).map_err(|e|e.to_string())?,work)?,work)?;
        }
    }
    let format=evidence_store::format_typed(conn,Some(work))?;
    let mut reader=evidence_store::Reader::new_mixed(conn,EVIDENCE_READ_BUDGET,work,observer)?;
    let mut batches=NativeTree::default();
    let mut originals=cache::mixed::Originals::default();
    for id in ids.iter(){
        let(reference,cert)=batch_ref(conn,&mut reader,id,work)?;
        if format.graph(){put(&mut originals,formatted(format_args!("batch/{}",id.as_ref()),work)?,cert,work)?;}
        put(&mut batches,id.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,reference,work)?;
    }
    let mut roots=NativeTree::default();
    if format.graph(){
        for key in root_keys(conn,work)?.iter(){
            let(kind,record)=evidence_store::root_kind(key)?;
            match kind{
                evidence_store::RootKind::Batch=>{
                    let present:i64=conn.query_row("SELECT COUNT(*) FROM batches WHERE batch_id=?1",params![record],|r|r.get(0)).map_err(|e|format!("StorageUnavailable：核私有根批次行失败：{e}"))?;
                    if present!=1{return Err(format!("StorageUnavailable：私有根 `{}` 没有对应批次行（不可达残留），拒绝写入",key.as_ref()).into());}
                    if !batches.contains_key(record){
                        let(_,_,cert)=load_batch(conn,&mut reader,record,false,work)?;
                        put(&mut originals,key.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,cert,work)?;
                    }
                }
                evidence_store::RootKind::Delta=>{
                    if !format.records_in_graph(){return Err(format!("StorageUnavailable：格式 `{}` 未声明 Delta 根 `{}`（不得混代）",format.tag(),key.as_ref()).into());}
                    let generation=record.parse::<i64>().map_err(|_|format!("StorageUnavailable：Delta 根 `{}` 代数非规范十进制",key.as_ref()))?;
                    let present:i64=conn.query_row("SELECT COUNT(*) FROM structure_deltas WHERE generation=?1",params![generation],|r|r.get(0)).map_err(|e|format!("StorageUnavailable：核 Delta 根行失败：{e}"))?;
                    if present!=1{return Err(format!("StorageUnavailable：私有根 `{}` 没有对应 Delta 行（不可达残留），拒绝写入",key.as_ref()).into());}
                    let verified=reader.read_root_typed(key,false)?;
                    put(&mut roots,key.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,verified.closure,work)?;
                    put(&mut originals,key.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,verified.root_cert,work)?;
                }
                evidence_store::RootKind::Fact=>{
                    if !format.records_in_graph(){return Err(format!("StorageUnavailable：格式 `{}` 未声明结构事实根 `{}`（不得混代）",format.tag(),key.as_ref()).into());}
                    let present:i64=conn.query_row("SELECT COUNT(*) FROM structure_facts WHERE object_id=?1",params![record],|r|r.get(0)).map_err(|e|format!("StorageUnavailable：核结构事实根行失败：{e}"))?;
                    if present!=1{return Err(format!("StorageUnavailable：私有根 `{}` 没有对应结构事实行（不可达残留），拒绝写入",key.as_ref()).into());}
                    let verified=reader.read_root_typed(key,false)?;
                    put(&mut roots,key.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,verified.closure,work)?;
                    put(&mut originals,key.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,verified.root_cert,work)?;
                }
            }
        }
        evidence_store::ensure_all_nodes_reachable_mixed(conn,cache::capture_reachable_len(reader.conn_id()),work)?;
    }else if evidence_store::graph_tables_present(conn)?{
        let roots=root_keys(conn,work)?;
        let(nodes,_)=evidence_store::stats(conn)?;
        if !roots.is_empty()||nodes!=0{return Err(format!("StorageUnavailable：整块域出现私有根/节点 {} 根 / {nodes} 节点，拒绝写入",roots.len()).into());}
    }
    Ok(Reachable{batches,roots,originals,_capture:reader.into_capture()})
}
