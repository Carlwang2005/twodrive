//! Provider-neutral local-file IPC. Never registers roots, copies or hydrates.
use anyhow::{Context, ensure};
use serde_json::{Value, json};
use std::path::Path;
use twodrive_local_files::migration::{self, SourceKind};

pub fn capabilities() -> Value {
    json!({"schema_version":1,"metadata_preflight":cfg!(windows),
        "native_mount":false,"import":false,"cloud_writes":false})
}

pub fn preflight(args: &Value) -> anyhow::Result<Value> {
    ensure!(cfg!(windows), "local metadata preflight requires Windows");
    ensure!(
        args["schema_version"] == 1,
        "unsupported local-file schema version"
    );
    let kind: SourceKind =
        serde_json::from_value(args["source_kind"].clone()).context("invalid source provenance")?;
    let path = args["source_path"]
        .as_str()
        .context("missing source path")?;
    ensure!(
        !path.contains('\0') && path.len() <= 32767 && Path::new(path).is_absolute(),
        "source path must be absolute and bounded"
    );
    let decision = migration::preflight(Path::new(path), kind)
        .context("local metadata could not be inspected")?;
    // No path, filenames, content, raw attributes or account identity echoed.
    Ok(
        json!({"schema_version":1,"decision":decision,"approval_required":true,
        "content_read":false,"hydration_requested":false,"import_performed":false,
        "cloud_upload_completion":"unverified"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_mount_is_not_implied_by_preflight() {
        assert_eq!(capabilities()["native_mount"], false);
        assert_eq!(capabilities()["import"], false);
        assert_eq!(capabilities()["metadata_preflight"], cfg!(windows));
    }
    #[cfg(not(windows))]
    #[test]
    fn non_windows_preflight_is_explicitly_unsupported() {
        assert!(preflight(&json!({"schema_version":1})).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn generated_file_has_redacted_metadata_only_ipc_result() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("generated-local-file.txt");
        std::fs::write(&path, b"generated IPC fixture").unwrap();
        let report = preflight(
            &json!({"schema_version":1,"source_path":path,"source_kind":"ordinary_directory"}),
        )
        .unwrap();
        assert_eq!(report["decision"], "full_file_needs_approved_plan");
        assert_eq!(report["content_read"], false);
        assert!(!report.to_string().contains("generated-local-file"));
        assert_eq!(std::fs::read(path).unwrap(), b"generated IPC fixture");
        assert!(preflight(&json!({"schema_version":2})).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn cache_and_unknown_are_refused_without_even_opening_the_path() {
        let path = std::env::temp_dir().join("nonexistent-generated-file");
        for (kind, decision) in [
            ("private_cache", "unsupported_cache"),
            ("virtual_mount", "unsupported_virtual_mount"),
            ("unknown", "unknown_provenance"),
        ] {
            assert_eq!(
                preflight(&json!({"schema_version":1,"source_path":path,"source_kind":kind}))
                    .unwrap()["decision"],
                decision
            );
        }
    }
}
