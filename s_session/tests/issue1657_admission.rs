use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64,Ordering};
static NEXT: AtomicU64=AtomicU64::new(0);
struct Fixture { root:PathBuf, db:PathBuf, state:PathBuf, raw:String }
impl Fixture {
    fn new()->Self {
        let root=std::env::temp_dir().join(format!("issue1657-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&root).unwrap();
        let root=root.canonicalize().unwrap();
        let db=root.join("source.sqlite"); std::fs::write(&db,b"fixture").unwrap();
        std::fs::set_permissions(&db,std::fs::Permissions::from_mode(0o600)).unwrap();
        let state=root.join("state"); std::fs::create_dir(&state).unwrap();
        let mut values=strict_json(CONTRACT.as_bytes()).unwrap();
        for value in values.as_object_mut().unwrap().values_mut() { *value=json!("1000000"); }
        Self{root,db,state,raw:canonical_json(&values)}
    }
    fn contract(&self)->Contract<'_> { Contract{version:"test-only/1",canonical:&self.raw} }
    fn site(&self)->Site { Site::open(&self.db,&self.state,unsafe{libc::geteuid()}).unwrap() }
    fn issue(&self)->Value { prepare(&self.site(),&self.contract(),"test-only/1",self.raw.as_bytes(),1000000).unwrap() }
    fn limits(&self)->PathBuf { self.state.join(PUBLISHED).join(LIMITS_NAME) }
    fn no_anchor(&self) { assert!(!self.state.join(PUBLISHED).exists()); assert!(!self.state.join(PENDING).exists()); }
}
impl Drop for Fixture { fn drop(&mut self) { let _=std::fs::remove_dir_all(&self.root); } }
#[test]
fn admission_prepare_roundtrip_and_explicit_revoke_then_reissue() {
    let f=Fixture::new(); f.issue();
    let prepared=load_for_db(&f.db,&f.limits(),ControlEnv::from_values(None,None)).unwrap();
    drop(prepared);
    let result=revoke(&f.site(),"operator-1").unwrap();
    let archive=Path::new(result["archive"].as_str().unwrap());
    assert!(archive.join(PUBLISHED).join(BINDING_NAME).exists());
    assert!(archive.join("intent.json").exists());
    f.no_anchor();
    let owner:Value=serde_json::from_slice(&std::fs::read(sidecar(&f.db,".control-owner.json")).unwrap()).unwrap();
    assert_eq!(owner,json!({"db":f.db,"state_dir":f.state}));
    f.issue();
    assert!(load_for_db(&f.db,&f.limits(),ControlEnv::from_values(None,None)).is_ok());
}
#[test]
fn admission_prepare_rejects_forged_limits_sha() {
    let f=Fixture::new(); f.issue(); std::fs::write(f.limits(),b"{}").unwrap();
    assert_eq!(load_for_db(&f.db,&f.limits(),ControlEnv::from_values(None,None)).unwrap_err().kind,ErrorKind::Changed);
}
#[test]
fn admission_prepare_rejects_relocated_db_identity() {
    let f=Fixture::new(); f.issue(); let other=f.root.join("copy.sqlite");
    std::fs::copy(&f.db,&other).unwrap();
    std::fs::copy(sidecar(&f.db,".control-owner.json"),sidecar(&other,".control-owner.json")).unwrap();
    assert_eq!(load_for_db(&other,&f.limits(),ControlEnv::from_values(None,None)).unwrap_err().kind,ErrorKind::Identity);
}
#[test]
fn admission_prepare_rejects_superseded_contract() {
    let f=Fixture::new();
    assert_eq!(prepare(&f.site(),&f.contract(),"test-only/0",f.raw.as_bytes(),1000000).unwrap_err(),"AdmissionContractSupersededOrUnknown"); f.no_anchor();
}
#[test]
fn admission_prepare_rejects_missing_contract_version() {
    let f=Fixture::new();
    assert_eq!(prepare(&f.site(),&f.contract(),"",f.raw.as_bytes(),1000000).unwrap_err(),"AdmissionContractVersionMissing"); f.no_anchor();
    assert!(dispatch(&["s".into(),"admission-prepare".into()]).unwrap_err().contains("--contract-version"));
}
#[test]
fn admission_prepare_rejects_existing_published_anchor() {
    let f=Fixture::new(); f.issue(); let original=std::fs::read(f.limits()).unwrap();
    assert_eq!(prepare(&f.site(),&f.contract(),"test-only/1",f.raw.as_bytes(),1000000).unwrap_err(),"AdmissionExistingAnchor");
    assert_eq!(original,std::fs::read(f.limits()).unwrap());
}
#[test]
fn admission_prepare_rejects_live_writer() {
    let f=Fixture::new(); let _writer=Lock::acquire(&f.db,".writer.lock").unwrap();
    assert!(Site::open(&f.db,&f.state,unsafe{libc::geteuid()}).err().unwrap().contains("StaleWriter")); f.no_anchor();
}
#[test]
fn admission_prepare_rejects_foreign_uid() {
    let f=Fixture::new();
    assert_eq!(Site::open(&f.db,&f.state,unsafe{libc::geteuid()}.wrapping_add(1)).err().unwrap(),"AdmissionForeignUid"); f.no_anchor();
}
#[test]
fn admission_prepare_rejects_unapproved_contract() {
    let f=Fixture::new(); let mut v=strict_json(f.raw.as_bytes()).unwrap(); v["max_total_requests"]=json!("1000001");
    assert_eq!(prepare(&f.site(),&f.contract(),"test-only/1",canonical_json(&v).as_bytes(),1000000).unwrap_err(),"AdmissionContractUnapproved"); f.no_anchor();
}
#[test]
fn admission_prepare_rejects_awaiting_measurement_contract() {
    let f=Fixture::new();
    assert_eq!(prepare(&f.site(),&CURRENT,VERSION,CONTRACT.as_bytes(),1000000).unwrap_err(),"AdmissionContractAwaitingMeasurement"); f.no_anchor();
    let path=f.root.join("contract.json"); std::fs::write(&path,CONTRACT).unwrap();
    let args=vec!["s".into(),"admission-prepare".into(),"--contract-version".into(),VERSION.into(),"--contract".into(),path.to_str().unwrap().into(),"--startup-timeout-ms".into(),"1000000".into()];
    assert_eq!(dispatch(&args).unwrap_err(),"AdmissionContractAwaitingMeasurement"); f.no_anchor();
}
#[test]
fn admission_prepare_rejects_pending_and_revokes_incomplete_pending() {
    let f=Fixture::new(); let site=f.site(); mkdir(&site.state.file,PENDING).unwrap();
    assert_eq!(prepare(&site,&f.contract(),"test-only/1",f.raw.as_bytes(),1000000).unwrap_err(),"AdmissionExistingAnchor");
    revoke(&site,"partial-1").unwrap(); f.no_anchor();
    assert!(f.state.join("s-admission-revoked-partial-1").join(PENDING).exists());
}
#[test]
fn admission_prepare_rejects_insufficient_startup_window() {
    let f=Fixture::new();
    assert_eq!(prepare(&f.site(),&f.contract(),"test-only/1",f.raw.as_bytes(),999999).unwrap_err(),"AdmissionStartupWindowTooShort"); f.no_anchor();
}
#[test]
fn admission_prepare_rejects_db_mode_and_symlink_state() {
    let f=Fixture::new(); std::fs::set_permissions(&f.db,std::fs::Permissions::from_mode(0o666)).unwrap();
    assert_eq!(Site::open(&f.db,&f.state,unsafe{libc::geteuid()}).err().unwrap(),"AdmissionDatabaseIdentity");
    std::fs::set_permissions(&f.db,std::fs::Permissions::from_mode(0o600)).unwrap();
    let link=f.root.join("link"); std::os::unix::fs::symlink(&f.state,&link).unwrap();
    assert!(Site::open(&f.db,&link,unsafe{libc::geteuid()}).is_err()); f.no_anchor();
}
#[test]
fn admission_prepare_revoke_resumes_archived_anchor_before_owner_commit() {
    let f=Fixture::new(); f.issue(); let site=f.site();
    let archive=mkdir(&site.state.file,REVOKING).unwrap();
    let dir=open_at(&site.state.file,OsStr::new(PUBLISHED),true).unwrap();
    let intent=json!({"id":"resume-1","db":f.db,"state_dir":f.state,"owner":site.owner().unwrap().unwrap().0,"containers":{PUBLISHED:identity(&dir).unwrap()}});
    let mut file=create_file(&archive,OsStr::new("intent.json")).unwrap(); write_file(&mut file,canonical_json(&intent).as_bytes()).unwrap();
    move_new(&site.state.file,PUBLISHED,&archive,PUBLISHED).unwrap();
    assert!(revoke(&site,"different-id").is_err());
    revoke(&site,"resume-1").unwrap(); f.no_anchor();
    assert!(f.state.join("s-admission-revoked-resume-1").join(PUBLISHED).exists());
}
