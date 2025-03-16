use crossbeam_channel::Receiver;
use serde::Serialize;
use std::path::PathBuf;

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

/// Struct for writing to a JSONL file
#[derive(Serialize, Clone, Default)]
pub struct ThreadPost {
    pub length: usize,
    pub raw_content: String,
    pub thread_id: String,
    pub source: String,
}

/// # JSONL Handler
///
/// Takes a receiver and writes the data to a JSONL file. The receiver should be a string format.
///
/// # Arguments
///
/// * `receiver` - `Receiver<String>` - The receiver channel that receives the data
/// * `output_folder` - `PathBuf` - The output folder where the JSONL file will be written.
///   Right now the output is hardcoded to `all.jsonl`
///
///
/// # Example
///
/// ```
/// let (tx, rx) = bounded(1000); // This can be unbounded
/// let rt = Runtime::new().unwrap();
/// let handle = rt.spawn(async {
///    write_jsonl_receiver(rx, output_folder).await
/// });
///
/// tx.send(String::from("Hello")).unwrap();
/// tx.send(String::from("World")).unwrap();
///
/// drop(tx);
/// rt.block_on(handle).unwrap().unwrap();
/// ```
pub async fn write_jsonl_receiver(
    receiver: Receiver<String>,
    output_folder: PathBuf,
    total_bytes: Arc<AtomicU64>,
) -> std::io::Result<()> {
    // Create a all.jsonl file
    let output_path = output_folder.join("all.jsonl");
    let file = File::create(output_path).await?;
    let mut writer = BufWriter::with_capacity(1_048_576, file);

    while let Ok(data) = receiver.recv() {
        let data = format!("{}\n", data);
        let bytes = writer.write(data.as_bytes()).await?;
        total_bytes.fetch_add(bytes as u64, Ordering::SeqCst);
    }

    writer.flush().await?;
    println!("Finished writing to all.jsonl");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;
    use tokio::runtime::Runtime;
    #[test]
    fn test_receiver() {
        let temp_dir = TempDir::new().unwrap();
        let output_folder = temp_dir.path().to_path_buf();
        let output_folder_clone = output_folder.clone();
        let (tx, rx) = bounded(1000);
        let rt = Runtime::new().expect("Unable to create tokio runtime");
        // Create a tokio runtime for the test
        let total_bytes = Arc::new(AtomicU64::new(0));
        let total_bytes_clone = total_bytes.clone();
        let handle = rt.spawn(async move {
            write_jsonl_receiver(rx, output_folder_clone, total_bytes_clone).await
        });

        tx.send(String::from("Hello")).unwrap();
        tx.send(String::from("World")).unwrap();
        drop(tx);

        // Wait for the async task to complete
        rt.block_on(handle).unwrap().unwrap();

        let output_path = output_folder.join("all.jsonl");
        let contents = std::fs::read_to_string(output_path).unwrap();

        assert_eq!(contents, "Hello\nWorld\n");
    }
}
