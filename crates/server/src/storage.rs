use std::path::PathBuf;
use tokio::fs;

/// Base directory for file storage
#[derive(Clone)]
pub struct Storage {
    pub base_dir: PathBuf,
}

impl Storage {
    pub fn new(base_dir: &str) -> Self {
        Self {
            base_dir: PathBuf::from(base_dir),
        }
    }
}

/// Initialize storage directory
pub async fn init(base_dir: &str) -> Storage {
    let storage = Storage::new(base_dir);
    fs::create_dir_all(&storage.base_dir)
        .await
        .expect("Failed to create storage directory");
    tracing::info!("File storage ready (dir={})", base_dir);
    storage
}

/// Upload a file
pub async fn upload(storage: &Storage, key: &str, data: &[u8], _content_type: &str) -> Result<(), String> {
    let path = storage.base_dir.join(key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directory: {e}"))?;
    }
    fs::write(&path, data)
        .await
        .map_err(|e| format!("Failed to write file: {e}"))?;
    Ok(())
}

/// Download a file
pub async fn download(storage: &Storage, key: &str) -> Result<Vec<u8>, String> {
    let path = storage.base_dir.join(key);
    fs::read(&path)
        .await
        .map_err(|e| format!("Failed to read file: {e}"))
}

/// Delete all files with a given prefix
pub async fn delete_prefix(storage: &Storage, prefix: &str) -> Result<(), String> {
    let path = storage.base_dir.join(prefix);
    let dir = if path.is_dir() {
        path
    } else if let Some(parent) = path.parent() {
        // prefix might be "123/" — treat as directory
        if parent.exists() { parent.to_path_buf() } else { return Ok(()); }
    } else {
        return Ok(());
    };

    if dir.exists() {
        fs::remove_dir_all(&dir)
            .await
            .map_err(|e| format!("Failed to delete: {e}"))?;
    }
    Ok(())
}

/// Delete a single file
pub async fn delete_file(storage: &Storage, key: &str) -> Result<(), String> {
    let path = storage.base_dir.join(key);
    if path.exists() {
        fs::remove_file(&path)
            .await
            .map_err(|e| format!("Failed to delete file: {e}"))?;
    }
    Ok(())
}
