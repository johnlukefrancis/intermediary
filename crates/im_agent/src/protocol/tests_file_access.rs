// Path: crates/im_agent/src/protocol/tests_file_access.rs
// Description: File staging, reading and directory listing wire contracts

use serde_json::json;

use super::{UiCommand, UiResponse};

#[test]
fn legacy_stage_file_result_windows_path_alias() {
    let json = json!({
        "type": "stageFileResult",
        "repoId": "repo",
        "path": "src/main.ts",
        "windowsPath": "C:\\staging\\repo\\src\\main.ts",
        "wslPath": "/mnt/c/staging/repo/src/main.ts",
        "bytesCopied": 128,
        "mtimeMs": 1700000000000_u64
    });

    let response: UiResponse = serde_json::from_value(json).expect("parse legacy stageFileResult");
    match response {
        UiResponse::StageFileResult(result) => {
            assert_eq!(result.host_path, "C:\\staging\\repo\\src\\main.ts");
            assert_eq!(
                result.wsl_path.as_deref(),
                Some("/mnt/c/staging/repo/src/main.ts")
            );
        }
        _ => panic!("expected StageFileResult"),
    }
}

#[test]
fn stage_file_result_accepts_host_and_legacy_windows_paths_together() {
    let json = json!({
        "type": "stageFileResult",
        "repoId": "repo",
        "path": "src/main.ts",
        "hostPath": "C:\\staging\\repo\\src\\main.ts",
        "windowsPath": "C:\\staging\\repo\\src\\main.ts",
        "wslPath": "/mnt/c/staging/repo/src/main.ts",
        "bytesCopied": 128,
        "mtimeMs": 1700000000000_u64
    });

    let response: UiResponse =
        serde_json::from_value(json).expect("parse dual-path stageFileResult");
    match response {
        UiResponse::StageFileResult(result) => {
            assert_eq!(result.host_path, "C:\\staging\\repo\\src\\main.ts");
            assert_eq!(
                result.legacy_windows_path.as_deref(),
                Some("C:\\staging\\repo\\src\\main.ts")
            );
        }
        _ => panic!("expected StageFileResult"),
    }
}

#[test]
fn read_text_file_command_and_result_roundtrip() {
    let command_json = json!({
        "type": "readTextFile",
        "repoId": "repo",
        "path": "docs/guide.md"
    });
    let command: UiCommand = serde_json::from_value(command_json).expect("parse readTextFile");
    match command {
        UiCommand::ReadTextFile(command) => {
            assert_eq!(command.repo_id, "repo");
            assert_eq!(command.path, "docs/guide.md");
        }
        _ => panic!("expected ReadTextFile"),
    }

    let response_json = json!({
        "type": "readTextFileResult",
        "repoId": "repo",
        "path": "docs/guide.md",
        "content": "# Guide\n",
        "bytes": 8,
        "mtimeMs": 1700000000000_u64,
        "encoding": "utf-8"
    });
    let response: UiResponse =
        serde_json::from_value(response_json).expect("parse readTextFileResult");
    match response {
        UiResponse::ReadTextFileResult(result) => {
            assert_eq!(result.repo_id, "repo");
            assert_eq!(result.path, "docs/guide.md");
            assert_eq!(result.content, "# Guide\n");
            assert_eq!(result.encoding, "utf-8");
        }
        _ => panic!("expected ReadTextFileResult"),
    }
}

#[test]
fn read_image_file_command_and_result_roundtrip() {
    let command_json = json!({
        "type": "readImageFile",
        "repoId": "repo",
        "path": "docs/capture.png"
    });
    let command: UiCommand = serde_json::from_value(command_json).expect("parse readImageFile");
    match command {
        UiCommand::ReadImageFile(command) => {
            assert_eq!(command.repo_id, "repo");
            assert_eq!(command.path, "docs/capture.png");
            assert_eq!(command.max_bytes, None, "maxBytes is optional on the wire");
            let wire = serde_json::to_value(UiCommand::ReadImageFile(command)).expect("serialize");
            assert!(
                wire.get("maxBytes").is_none(),
                "an absent bound is not serialized"
            );
        }
        _ => panic!("expected ReadImageFile"),
    }
    let bounded: UiCommand = serde_json::from_value(json!({
        "type": "readImageFile",
        "repoId": "repo",
        "path": "docs/capture.png",
        "maxBytes": 4_194_304_u64
    }))
    .expect("parse bounded readImageFile");
    match bounded {
        UiCommand::ReadImageFile(command) => assert_eq!(command.max_bytes, Some(4_194_304)),
        _ => panic!("expected ReadImageFile"),
    }

    let response_json = json!({
        "type": "readImageFileResult",
        "repoId": "repo",
        "path": "docs/capture.png",
        "dataBase64": "cG5n",
        "mimeType": "image/png",
        "bytes": 3,
        "mtimeMs": 1700000000000_u64
    });
    let response: UiResponse =
        serde_json::from_value(response_json).expect("parse readImageFileResult");
    match response {
        UiResponse::ReadImageFileResult(result) => {
            assert_eq!(result.repo_id, "repo");
            assert_eq!(result.path, "docs/capture.png");
            assert_eq!(result.data_base64, "cG5n");
            assert_eq!(result.mime_type, "image/png");
        }
        _ => panic!("expected ReadImageFileResult"),
    }
}

#[test]
fn list_repo_directory_command_and_result_roundtrip() {
    let command_json = json!({
        "type": "listRepoDirectory",
        "repoId": "repo",
        "path": "app/src"
    });
    let command: UiCommand = serde_json::from_value(command_json).expect("parse listRepoDirectory");
    match command {
        UiCommand::ListRepoDirectory(command) => {
            assert_eq!(command.repo_id, "repo");
            assert_eq!(command.path, "app/src");
        }
        _ => panic!("expected ListRepoDirectory"),
    }

    let response_json = json!({
        "type": "listRepoDirectoryResult",
        "repoId": "repo",
        "path": "app/src",
        "dirs": ["app/src/components"],
        "files": ["app/src/main.ts"]
    });
    let response: UiResponse =
        serde_json::from_value(response_json).expect("parse listRepoDirectoryResult");
    match response {
        UiResponse::ListRepoDirectoryResult(result) => {
            assert_eq!(result.repo_id, "repo");
            assert_eq!(result.path, "app/src");
            assert_eq!(result.dirs, vec!["app/src/components".to_string()]);
            assert_eq!(result.files, vec!["app/src/main.ts".to_string()]);
        }
        _ => panic!("expected ListRepoDirectoryResult"),
    }
}
