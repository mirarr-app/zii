use std::path::{Path, PathBuf};
use anyhow::Context;

pub struct TrashItem {
    pub original_path: PathBuf,
    pub temporary_backup: Option<PathBuf>,
}

pub struct TrashManager {
    history: Vec<TrashItem>,
    backup_dir: PathBuf,
}

impl TrashManager {
    pub fn new() -> Self {
        let backup_dir = std::env::temp_dir().join("zii_trash_backup");
        let _ = std::fs::create_dir_all(&backup_dir);
        Self {
            history: Vec::new(),
            backup_dir,
        }
    }

    /// Move file to FreeDesktop trash and keep a rapid-restore backup for instant 'u' undo
    pub fn move_to_trash(&mut self, path: &Path) -> anyhow::Result<()> {
        let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !canon.exists() {
            anyhow::bail!("File does not exist: {:?}", path);
        }

        // Create quick temp backup for immediate undo
        let filename = canon
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let backup_path = self.backup_dir.join(format!("{}_{}", uuid_timestamp(), filename));

        let _ = std::fs::copy(&canon, &backup_path);

        // Move to FreeDesktop trash
        trash::delete(&canon).with_context(|| format!("Failed to move {:?} to trash", canon))?;

        self.history.push(TrashItem {
            original_path: canon,
            temporary_backup: Some(backup_path),
        });

        Ok(())
    }

    /// Restore the most recently trashed file back to its original location
    pub fn restore_last(&mut self) -> anyhow::Result<Option<PathBuf>> {
        if let Some(item) = self.history.pop() {
            if let Some(backup) = item.temporary_backup {
                if backup.exists() {
                    std::fs::copy(&backup, &item.original_path)?;
                    let _ = std::fs::remove_file(backup);
                    return Ok(Some(item.original_path));
                }
            }
        }
        Ok(None)
    }

    /// Permanently remove file
    pub fn delete_permanent(&mut self, path: &Path) -> anyhow::Result<()> {
        let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if canon.exists() {
            std::fs::remove_file(&canon)?;
        }
        Ok(())
    }
}

fn uuid_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
