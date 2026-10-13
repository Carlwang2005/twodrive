#[path = "support/opaque_provider.rs"]
mod opaque;
use opaque::OpaqueProvider;
use std::sync::Arc;
use twodrive_dev::{
    model::CloudBackend,
    provider::{Backend, Capabilities},
};

#[test]
fn file_actions_resolve_opaque_ids_and_keep_shallow_browsing() {
    let p = OpaqueProvider::default();
    p.create_folder("/folder").unwrap();
    let entry = p.upload("/folder/file", b"bytes".to_vec()).unwrap();
    assert_ne!(entry.remote_id, entry.path);
    // The original desktop/backup expression fails for this provider.
    assert!(
        p.download_to(&entry.path, &mut vec![], &mut |_| Ok(()))
            .is_err()
    );
    let b = Backend::new(p);
    assert_eq!(b.list_directory("/").unwrap().len(), 1);
    assert_eq!(b.list_directory("/folder").unwrap().len(), 1);
    let mut data = vec![];
    b.download_path_to("/folder/file", &mut data, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(data, b"bytes");
    let moved = b.rename_path("/folder/file", "/folder/moved").unwrap();
    assert_eq!(moved.remote_id, entry.remote_id);
    b.delete_path("/folder/moved").unwrap();
    assert!(b.list_directory("/folder").unwrap().is_empty());
}
#[test]
fn conditional_upload_and_pause_preserve_remote_content() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::write(&source, b"first").unwrap();
    let b = Backend::new(OpaqueProvider::default());
    let entry = b
        .upload_path("/file", &source, None, Arc::new(|_, _| Ok(())))
        .unwrap();
    std::fs::write(&source, b"next").unwrap();
    assert!(
        b.upload_path("/file", &source, None, Arc::new(|_, _| Ok(())))
            .is_err()
    );
    assert!(
        b.upload_path("/file", &source, Some("stale"), Arc::new(|_, _| Ok(())))
            .is_err()
    );
    assert_eq!(b.download(&entry.remote_id).unwrap(), b"first");
    let next = b
        .upload_path("/file", &source, Some(&entry.etag), Arc::new(|_, _| Ok(())))
        .unwrap();
    assert_eq!(b.download(&next.remote_id).unwrap(), b"next");
    let paused = dir.path().join("paused");
    std::fs::write(&paused, b"true").unwrap();
    let paused = b.clone().with_pause_file(paused);
    assert!(
        paused
            .upload_path("/new", &source, None, Arc::new(|_, _| Ok(())))
            .is_err()
    );
    assert!(b.get_metadata_by_path("/new").unwrap().is_none());
    assert!(
        b.upload_path(
            "/cancel",
            &source,
            None,
            Arc::new(|_, _| anyhow::bail!("cancelled"))
        )
        .is_err()
    );
    assert!(b.get_metadata_by_path("/cancel").unwrap().is_none());
}
#[test]
fn unsafe_legacy_upload_defaults_and_mutations_are_not_offered() {
    let p = OpaqueProvider::default().with_caps(Capabilities::default());
    let entry = p.upload("/file", b"unchanged".to_vec()).unwrap();
    let b = Backend::new(p);
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::write(&source, b"replacement").unwrap();
    assert!(!b.capabilities().backup());
    assert!(
        b.upload_path("/file", &source, Some(&entry.etag), Arc::new(|_, _| Ok(())))
            .is_err()
    );
    assert!(b.rename_path("/file", "/renamed").is_err());
    assert!(b.delete_path("/file").is_err());
    assert!(b.create_folder("/folder").is_err());
    assert_eq!(b.download(&entry.remote_id).unwrap(), b"unchanged");
    assert!(b.delete_path("/").is_err());
    assert!(b.entry("/../escape").is_err());
}
