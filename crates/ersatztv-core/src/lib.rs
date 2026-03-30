use std::time::{Duration, Instant};

use tokio::fs::{read_dir, remove_dir, remove_file};

pub const READY_FILE_NAME: &str = ".ready";

pub async fn empty_folder(output_folder: &std::path::Path) -> Result<(), std::io::Error> {
    let mut entries = read_dir(output_folder).await?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(file_type) = entry.file_type().await {
            if file_type.is_dir() {
                Box::pin(empty_folder(&entry.path())).await?;
                remove_dir(entry.path()).await?;
            } else {
                remove_file(entry.path()).await?;
            }
        }
    }

    Ok(())
}

pub async fn wait_for_file(path: &std::path::Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if path.exists() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(200)).await
    }
}
