use super::repository::Repository;
use std::path::Path;
#[test]
fn analysis_recovery_paging_and_invalid_filter() {
    let r = Repository::open(Path::new(":memory:")).unwrap();
    for i in 0..51 {
        r.conn.execute("INSERT INTO video_analyses(id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,created_at,updated_at) VALUES(?1,'keep.wmv','fp','transcribing','standard','deepseek-flash','v0.5.0-1',1,1)",[format!("a{i:03}")]).unwrap();
    }
    assert_eq!(r.list(None, 1).unwrap().items.len(), 50);
    assert_eq!(r.list(None, 2).unwrap().items.len(), 1);
    assert_eq!(r.recover().unwrap(), 51);
    assert_eq!(r.get("a000").unwrap().status, "interrupted");
    assert_eq!(r.recover().unwrap(), 0);
    assert!(r.list(Some("' OR 1=1"), 1).is_err());
    assert!(r.get("missing").is_err());
    assert_eq!(r.list(Some("completed"), 1).unwrap().total, 0);
}
