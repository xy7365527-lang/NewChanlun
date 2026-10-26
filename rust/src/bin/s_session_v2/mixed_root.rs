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
    if evidence_store::format_typed(conn,None,Some(work))?.graph(){
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
