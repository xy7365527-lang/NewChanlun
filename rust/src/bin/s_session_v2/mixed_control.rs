//! #1527：控制表模式与换代历史的持费读取。
use super::*;
use crate::v3::{audit_budget::LegacyObserver,audit_store::NativeTree,business_json::MixedJsonError};
type Text=NativeValue<String>;
pub(crate) type EpochFacts=CodecVec<(Text,i64)>;
pub(crate) struct Schema {tables:NativeTree<Text,Text>}
struct Normalized<'a>(&'a str);
impl fmt::Display for Normalized<'_> {
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        normalize_schema_chunks(self.0,|chunk|f.write_str(chunk))
    }
}
fn normalized(value:&str,work:&WorkScope)->Result<Text,MixedJsonError>{
    Text::try_format(format_args!("{}",Normalized(value)),Some(work)).map_err(MixedJsonError::Resource)
}
fn sql_text(row:&mut CheckedRow<'_>,column:usize,work:&WorkScope)->Result<Text,MixedJsonError>{
    Text::try_from_str(row.text(column).map_err(err)?,Some(work)).map_err(MixedJsonError::Resource)
}
impl Schema {
    fn new()->Self {Self{tables:Default::default()}}
    pub(crate) fn add(&mut self,label:&str,ddl:&str,work:&WorkScope)->Result<(),MixedJsonError>{
        ddl_table_entries(label,ddl,|name,sql|{
            let key=Text::try_from_str(name,Some(work)).map_err(MixedJsonError::Resource)?;
            let value=normalized(sql,work)?;
            self.tables.insert_admitted(key,value,&NativeAdmission(Some(work))).map_err(MixedJsonError::Resource)?;
            Ok(())
        })
    }
    pub(crate) fn verify(&self,conn:&Connection,work:&WorkScope,observer:&dyn LegacyObserver)->Result<(),MixedJsonError>{
        observer.check_work(work).map_err(MixedJsonError::Resource)?;
        let mut stmt=CheckedStatement::prepare(conn,c"SELECT type,name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY name").map_err(err)?;
        let mut seen=crate::v3::audit_store::NativeSet::default();
        while let Some(mut row)=stmt.next().map_err(err)? {
            let kind=sql_text(&mut row,0,work)?;
            let name=sql_text(&mut row,1,work)?;
            let sql=sql_text(&mut row,2,work)?;
            if kind.as_str()!="table" {return Err(err(format!("模式/约束不匹配 {}/{}",kind.as_ref(),name.as_ref())).into());}
            let value=normalized(&sql,work)?;
            if self.tables.get(name.as_str()).map(|v|v.as_str())!=Some(value.as_str()) {
                return Err(err(format!("模式/约束不匹配 {}/{}",kind.as_ref(),name.as_ref())).into());
            }
            seen.insert_admitted(name,&NativeAdmission(Some(work))).map_err(MixedJsonError::Resource)?;
        }
        stmt.finalize().map_err(err)?;
        if seen.len()!=self.tables.len() {return Err(err("缺必需表/约束").into());}
        for table in self.tables.keys() {
            let mut columns=CheckedStatement::prepare(conn,c"SELECT type,\"notnull\",pk FROM pragma_table_info(?1) ORDER BY cid").map_err(err)?;
            columns.bind_text(1,table).map_err(err)?;
            let mut declarations=CodecVec::default();
            while let Some(mut row)=columns.next().map_err(err)? {
                let item=(sql_text(&mut row,0,work)?,row.integer(1).map_err(err)?,row.integer(2).map_err(err)?);
                declarations.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
                declarations.try_push(item).map_err(MixedJsonError::Resource)?;
            }
            columns.finalize().map_err(err)?;
            let sql=Text::try_format(format_args!("SELECT * FROM {}\0",table.as_ref()),Some(work)).map_err(MixedJsonError::Resource)?;
            let sql=CStr::from_bytes_with_nul(sql.as_bytes()).map_err(err)?;
            let mut rows=CheckedStatement::prepare(conn,sql).map_err(err)?;
            while let Some(mut row)=rows.next().map_err(err)? {
                for (i,(kind,notnull,pk)) in declarations.iter().enumerate() {
                    let storage=row.column_type(i).map_err(err)?;
                    if storage==rusqlite::types::Type::Blob {row.blob(i).map_err(err)?;}
                    let text=if storage==rusqlite::types::Type::Text {Some(row.text_bytes(i).map_err(err)?)}else{None};
                    if !valid_storage_cell(kind,*notnull,*pk,storage,text) {
                        return Err(err(format!("{} 第{i}列实际类型/非空约束损坏",table.as_ref())).into());
                    }
                }
            }
            rows.finalize().map_err(err)?;
        }
        Ok(())
    }
}
pub(crate) fn verify_schema(conn:&Connection,enabled:bool,work:&WorkScope,observer:&dyn LegacyObserver)->Result<Schema,MixedJsonError>{
    let ohlc=tb02a::enabled_admitted(conn,Some(work))?;
    let mut expected=Schema::new();
    expected.add("schema常量",SCHEMA,work)?;
    if enabled {expected.add("schema常量",SCHEMA_V2,work)?;}
    if ohlc {expected.add("schema常量",tb02a::SCHEMA,work)?;}
    let delta:i64=conn.query_row("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?1",params![delta_storage::TABLE],|r|r.get(0)).map_err(err)?;
    if delta==1 {expected.add("Delta DDL",delta_storage::DDL,work)?;}
    let graph:i64=conn.query_row("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name IN (?1,?2,?3)",
        params![evidence_store::FORMAT_TABLE,evidence_store::NODES_TABLE,evidence_store::ROOTS_TABLE],|r|r.get(0)).map_err(err)?;
    if graph!=0 {
        if graph!=3 {return Err(err("私有证据图表不齐全（三表必须同生）").into());}
        expected.add("证据图 DDL",evidence_store::DDL,work)?;
    }
    expected.verify(conn,work,observer)?;
    delta_storage::read_mode(conn)?;
    evidence_store::read_format_typed(conn,None,Some(work))?;
    Ok(expected)
}
pub(crate) fn active(conn:&Connection,work:&WorkScope)->Result<bool,MixedJsonError>{
    let marker=crate::audit::mixed_profile::meta_text(conn,"protocol_revision",work)?;
    let tables:i64=conn.query_row("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name IN ('s_protocol_meta','s_input_messages','s_clock_events','s_delivery_policy','s_delivery_refs')",[],|r|r.get(0)).map_err(err)?;
    match(marker.as_ref().map(|v|v.as_str()),tables){
        (None,0)=>Ok(false),(Some("s-session/2"),5)=>Ok(true),_=>Err(err("协议标记/控制表缺失或不支持").into()),
    }
}
pub(crate) fn epoch_history(conn:&Connection,enabled:bool,work:&WorkScope)->Result<EpochFacts,MixedJsonError>{
    let generation=crate::audit::mixed_profile::meta_text(conn,"generation",work)?.ok_or_else(||err("StorageUnavailable：meta.generation 缺失"))?;
    let generation=parse_canonical_i64(&generation,"meta.generation").map_err(|e|format!("StorageUnavailable：{e}"))?;
    let current=crate::audit::mixed_profile::meta_text(conn,"writer_epoch",work)?.ok_or_else(||err("缺writer_epoch"))?;
    let current=parse_writer_epoch(&current,"meta.writer_epoch").map_err(err)?;
    let mut query=CheckedStatement::prepare(conn,c"SELECT ordinal,from_epoch,to_epoch,generation_at_transition,advance_state_at_transition,transitioned_at FROM writer_epoch_history ORDER BY ordinal").map_err(err)?;
    let(mut previous_epoch,mut previous_generation,mut ordinal)=(None,0,0i64);
    let mut facts=CodecVec::default();
    while let Some(mut row)=query.next().map_err(err)?{
        ordinal+=1;
        let actual=row.integer(0).map_err(err)?;
        let from=parse_writer_epoch(&sql_text(&mut row,1,work)?,"epoch.from").map_err(err)?;
        let to=parse_writer_epoch(&sql_text(&mut row,2,work)?,"epoch.to").map_err(err)?;
        let at=nonnegative(&sql_text(&mut row,3,work)?,"epoch.generation").map_err(err)?;
        let state=sql_text(&mut row,4,work)?;
        let ns=sql_text(&mut row,5,work)?;
        nonnegative(&ns,"epoch.transitioned_at").map_err(err)?;
        validate_epoch_step(actual,ordinal,from,to,at,&state,previous_epoch,previous_generation,generation,enabled)?;
        previous_epoch=Some(to);previous_generation=at;
        facts.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
        facts.try_push((ns,at)).map_err(MixedJsonError::Resource)?;
    }
    query.finalize().map_err(err)?;
    if previous_epoch.is_some_and(|last|last!=current) {return Err(err("writer_epoch与最后换代结果不一致").into());}
    Ok(facts)
}
