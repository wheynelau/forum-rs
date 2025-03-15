use crossbeam_channel::Receiver;
use serde::Serialize;
use std::path::PathBuf;

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
) -> std::io::Result<()> {
    use tokio::fs::File;
    use tokio::io::{AsyncWriteExt, BufWriter};

    // Create a all.jsonl file
    let output_path = output_folder.join("all.jsonl");
    let file = File::create(output_path).await?;
    let mut writer = BufWriter::with_capacity(1_048_576, file);

    while let Ok(data) = receiver.recv() {
        writer.write_all(format!("{}\n", data).as_bytes()).await?;
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
    #[test]
    fn test_get_chunk_size() {
        let post = ThreadPost::default();
        // repeat 1000 for data
        let data: Vec<ThreadPost> = vec![post; 1000];
        // Total size 300MB
        let bytes = 300 * 1024_usize.pow(2);
        let chunk_size = super::get_chunk_size(bytes, &data);
        // 1000 / 3 ceil = 334
        assert_eq!(chunk_size, 334);
    }

    #[test]
    fn test_receiver() {
        let temp_dir = TempDir::new().unwrap();
        let output_folder = temp_dir.path().to_path_buf();
        let output_folder_clone = output_folder.clone();
        let (tx, rx) = bounded(1000);

        // Create a tokio runtime for the test
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.spawn(async move { write_jsonl_receiver(rx, output_folder_clone).await });

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
