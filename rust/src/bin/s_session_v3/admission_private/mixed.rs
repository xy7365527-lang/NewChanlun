//! #1527：旧域 mixed 私有准入全生命周期。外层生产 gate 在验收完成前保持关闭。
use super::*;
use super::mixed_accounting::MixedAccounting;
use super::mixed_sql::MixedPolicy;
use crate::v3::admission_anchor::terminal::FixedFailure;
use crate::v3::audit_budget::LegacyObserver;
use crate::v3::business_json::{MixedFailureScope,MixedJsonError};
use sc::capture_allocation::native_owned::NativeValue;
use std::cell::Cell;

pub(crate) struct SealedPrefix {
    protocol: Option<NativeValue<String>>,
    epoch: NativeValue<String>,
}
impl SealedPrefix {
    pub(crate) fn require(&self,prefix:&RejectionPrefix<'_>)->Result<(),FixedFailure>{
        match prefix {
            RejectionPrefix::None=>Ok(()),
            _=>Err(FixedFailure::new("MixedProtocolMismatch","business",
                "旧域封存结果不得充当 s-session/3 启动审计")),
        }
    }
}
fn typed<T>(shared:&MixedAccounting<'_,'_>,result:Result<T,MixedJsonError>)->Result<T,FixedFailure>{
    result.map_err(|error|match error{
        MixedJsonError::Resource(error)=>shared.codec_failure(error),
        MixedJsonError::Business(error)=>shared.failure("MixedBusinessRejected",&error),
    })
}
fn quick_check(conn:&rusqlite::Connection,shared:&MixedAccounting<'_,'_>)->Result<(),FixedFailure>{
    let fail=||shared.failure("PrivateSqliteCheckFailed","私有SQLite完整性检查失败");
    let mut stmt=conn.prepare("PRAGMA quick_check").map_err(|_|fail())?;
    let mut rows=stmt.query([]).map_err(|_|fail())?;
    let mut seen=false;
    while let Some(row)=rows.next().map_err(|_|fail())?{
        let value=row.get_ref(0).map_err(|_|fail())?;
        let value=value.as_str().map_err(|_|fail())?;
        if value!="ok"||seen{return Err(shared.failure("PrivateSqliteIntegrityRejected","完整性结果拒绝"));}
        seen=true;
    }
    if !seen{return Err(shared.failure("PrivateSqliteIntegrityEmpty","完整性结果缺失"));}
    Ok(())
}
fn business(conn:&rusqlite::Connection,shared:&MixedAccounting<'_,'_>)->Result<SealedPrefix,FixedFailure>{
    let tx=conn.unchecked_transaction().map_err(|_|shared.failure("PrivateBeginFailed","私有事务打开失败"))?;
    let result=(||{
        let protocol=typed(shared,crate::audit::mixed_profile::meta_text(&tx,"protocol_revision",shared.work))?;
        match protocol.as_ref().map(|p|p.as_str()){
            None|Some(super::super::PREVIOUS_REVISION)=>{}
            _=>return Err(shared.failure("MixedProtocolUnsupported","mixed旧域SQL合同不接纳此协议")),
        }
        let raw=typed(shared,crate::v2::mixed_root::startup(&tx,shared.work,shared))?;
        drop(raw);
        let epoch=typed(shared,crate::audit::mixed_profile::meta_text(&tx,"writer_epoch",shared.work))?
            .ok_or_else(||shared.failure("MixedEpochMissing","完整核后缺writer_epoch"))?;
        Ok(SealedPrefix{protocol,epoch})
    })();
    shared.set_phase(ErrorPhase::Rollback);
    let disabled=conn.progress_handler(0,None::<fn()->bool>)
        .map_err(|_|shared.failure("PrivateProgressDisableFailed","撤下VM计算门失败"));
    let ended=tx.rollback().map_err(|_|shared.failure("PrivateRollbackFailed","私有只读事务回滚失败"));
    shared.set_phase(ErrorPhase::Gate);
    let gate=LegacyObserver::precheck(shared).map_err(|e|shared.codec_failure(e));
    match result{
        Ok(sealed)=>{disabled?;ended?;gate?;Ok(sealed)}
        Err(error)=>Err(error),
    }
}
pub(super) fn validate(
    source:&MeasuredDomain, account:&mut AdmissionAccounting<'_>,
)->Result<SealedPrefix,FixedFailure>{
    let _failure_scope=MixedFailureScope::enter()
        .map_err(|e|FixedFailure::new("MixedScopeInvalid","business",e))?;
    let policy=Cell::new(MixedPolicy::Bootstrap);
    let shared=MixedAccounting::new(account,&policy)
        .ok_or_else(||FixedFailure::new("MixedWorkMissing","business","缺当前mixed WorkScope"))?;
    let result=(||{
        LegacyObserver::precheck(&shared).map_err(|e|shared.codec_failure(e))?;
        if matches!(source.medium,Medium::RollbackJournal){
            return Err(shared.failure("D2ProductionCapacityMissing","mixed合同未新增D2映射"));
        }
        let grant=shared.reserve().map_err(|e|shared.codec_failure(e))?;
        let result=(||{
            let main=source.main_fd().map_err(|e|shared.failure("PrivateSourceOpenFailed",&e))?;
            let wal=source.wal_fd().map_err(|e|shared.failure("PrivateSourceOpenFailed",&e))?;
            let mut io=&shared;
            let registration=fd_wal::Registration::new_mixed(main,wal,&mut io,&policy)
                .map_err(|_|shared.failure("PrivateVfsRegistrationFailed","私有VFS注册失败"))?;
            let result=(||{
                let session=registration.connect(true)
                    .map_err(|_|shared.failure("PrivateSqliteOpenFailed","私有连接失败"))?;
                let result=quick_check(session.connection(),&shared)
                    .and_then(|_|business(session.connection(),&shared));
                shared.set_phase(ErrorPhase::Close);
                let closed=session.close().map_err(|_|shared.failure("PrivateSqliteCloseFailed","私有连接关闭失败"));
                match result{Ok(sealed)=>{closed?;Ok(sealed)},Err(error)=>Err(error)}
            })();
            shared.set_phase(ErrorPhase::Registration);
            let cleanup=registration.finish()
                .map_err(|_|shared.failure("PrivateVfsCleanupFailed","私有注册撤销失败"));
            match result{Ok(sealed)=>{cleanup?;Ok(sealed)},Err(error)=>Err(error)}
        })();
        shared.set_phase(ErrorPhase::Reconcile);
        let reconciled=shared.reconcile(&grant).map_err(|e|shared.codec_failure(e));
        shared.set_phase(ErrorPhase::Deadline);
        let deadline=LegacyObserver::precheck(&shared).map_err(|e|shared.codec_failure(e));
        match result{Ok(sealed)=>{reconciled?;deadline?;Ok(sealed)},Err(error)=>Err(error)}
    })();
    if result.is_err(){shared.publish_failure();}
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_private_actual_vfs_root_and_operation_seal(){
        crate::v2::with_mixed_control_fixture(|conn|{
            let original:String=conn.query_row("PRAGMA database_list",[],|r|r.get(2)).unwrap();
            let path=std::path::Path::new(&original).with_file_name("mixed-private.sqlite");
            conn.execute("VACUUM INTO ?1",rusqlite::params![path.to_str().unwrap()]).unwrap();
            let prepared=crate::v3::admission_anchor::test_prepare_mixed(&path,1<<26,1<<26);
            let consumed=prepared.consume().unwrap();
            let source=MeasuredDomain::measure(&consumed).unwrap();
            let mut started=consumed.into_job(source.existing).unwrap();
            let result=started.with_accounting(|account|{
                let result=validate(&source,account);
                if let Err(error)=&result{
                    panic!("mixed private {} {} {}",error.code,error.phase,error.detail());
                }
                result
            });
            let sealed=match result{
                Ok(sealed)=>sealed,
                Err(_)=>panic!("mixed private work failed"),
            };
            assert_eq!(sealed.protocol.as_ref().map(|p|p.as_str()),Some("s-session/2"));
            assert_eq!(sealed.epoch.as_str(),"1");
            assert!(sealed.require(&RejectionPrefix::None).is_ok());
            assert!(sealed.require(&RejectionPrefix::UnboundRecover).is_err());
            let mut finished=started.seal().unwrap();
            assert!(finished.accounting().mixed_scope().is_none());
            assert!(finished.accounting().snapshot().records>0);
            assert!(finished.accounting().snapshot().nodes>0);
            source.verify_before_open().unwrap();
            drop(sealed);
        });
    }
    #[test]
    fn mixed_private_failed_parent_preserves_cache_for_next_real_admission(){
        crate::v2::with_mixed_control_fixture(|conn|{
            let original:String=conn.query_row("PRAGMA database_list",[],|r|r.get(2)).unwrap();
            let path=std::path::Path::new(&original).with_file_name("mixed-recovery.sqlite");
            conn.execute("VACUUM INTO ?1",rusqlite::params![path.to_str().unwrap()]).unwrap();
            crate::cache::configure(1<<20);
            for pass in 0..3 {
                let command=crate::v3::admission_anchor::CommandAccounting::begin();
                let wire=if pass==1 {4096} else {1<<26};
                let consumed=crate::v3::admission_anchor::test_prepare_mixed_wire(&path,1<<26,1<<26,wire).consume().unwrap();
                let source=MeasuredDomain::measure(&consumed).unwrap();
                let mut started=consumed.into_job(source.existing).unwrap();
                let result=started.with_accounting(|account|validate(&source,account));
                if pass==1 {
                    assert!(result.is_err());
                    drop(started);
                }else{
                    assert!(result.is_ok());
                    if pass==2 { assert_eq!(crate::cache::last_resumed(),2); }
                    let _finished=started.seal().unwrap();
                }
                source.verify_before_open().unwrap();
                drop(command);
            }
            crate::cache::configure(0);
        });
    }
    #[test]
    fn mixed_private_real_vfs_wire_refusal_keeps_original_cause(){
        use crate::v3::admission_anchor::{CommandAccounting,test_prepare_mixed_wire};
        crate::v2::with_mixed_control_fixture(|conn|{
            let original:String=conn.query_row("PRAGMA database_list",[],|r|r.get(2)).unwrap();
            let path=std::path::Path::new(&original).with_file_name("mixed-wire.sqlite");
            conn.execute("VACUUM INTO ?1",rusqlite::params![path.to_str().unwrap()]).unwrap();
            let command=CommandAccounting::begin();
            let prepared=test_prepare_mixed_wire(&path,1<<26,1<<26,4096);
            let consumed=prepared.consume().unwrap();
            let source=MeasuredDomain::measure(&consumed).unwrap();
            let mut started=consumed.into_job(source.existing).unwrap();
            let result=started.with_accounting(|account|validate(&source,account));
            let error=match result{
                Err(sc::capture_allocation::scratch::WorkError::Operation(error))=>error,
                _=>panic!("expected original callback wire failure"),
            };
            assert_eq!(error.code,sc::job::LIMIT_EXCEEDED);
            let before=started.accounting().snapshot();
            assert!(before.wire_bytes>4096);
            assert_eq!(before.stopped,Some(sc::job::LIMIT_EXCEEDED));
            let mut output=Vec::new();
            let receipt=command.report("",&mut output);
            assert_eq!(receipt.job.as_ref().unwrap().wire_bytes,before.wire_bytes);
            assert_eq!(receipt.actual_stderr_bytes,output.len() as u128);
            assert_eq!(receipt.unaccepted_stderr_bytes,output.len() as u128);
            let value:serde_json::Value=serde_json::from_slice(&output).unwrap();
            assert_eq!(value["code"],sc::job::LIMIT_EXCEEDED);
            assert_eq!(value["phase"],"业务返回");
            source.verify_before_open().unwrap();
        });
    }

}
