use crossbeam_channel::Receiver;
use rayon::prelude::*;
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[doc(hidden)]
const MAX_BYTES_PER_FILE: usize = 100 * 1024_usize.pow(2);

/// Struct for writing to a JSONL file
#[derive(Serialize, Clone, Default)]
pub struct ThreadPost {
    pub length: usize,
    pub raw_content: String,
    pub thread_id: String,
    pub source: String,
}
#[doc(hidden)]
fn get_chunk_size(bytes: usize, data: &[ThreadPost]) -> usize {
    let num_files = bytes.div_ceil(MAX_BYTES_PER_FILE);
    let num_splits = num_files.max(1);
    data.len().div_ceil(num_splits)
}
#[doc(hidden)]
#[allow(dead_code)]
/// Writes a vector of ThreadPost to a JSONL file
///
/// Legacy code kept for reference
fn _write_jsonl(data: Vec<ThreadPost>, bytes: usize, file_path: PathBuf) -> std::io::Result<()> {
    // Trying to implement rayon
    // Note that the size of the input should be checked before entering here
    let chunk_size = get_chunk_size(bytes, &data);

    let folder = file_path.parent().unwrap().to_str().unwrap();
    let stem = file_path.file_stem().unwrap().to_str().unwrap();
    let extension = file_path.extension().unwrap().to_str().unwrap();

    if data.len() <= chunk_size {
        // TODO: Possible refactor for the naming convention
        let file_path = Path::new(folder).join(format!("{}_0.{}", stem, extension));
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);

        for hashmap in data {
            let json_line = serde_json::to_string(&hashmap).unwrap();
            writeln!(&mut writer, "{}", json_line).unwrap();
        }
        Ok(())
    } else {
        // break the data into chunks of N chunks
        data.par_chunks(chunk_size)
            .enumerate()
            .for_each(|(i, chunk)| {
                let file_path = Path::new(folder).join(format!("{}_{}.{}", stem, i, extension));
                let file = File::create(file_path).unwrap();
                let mut writer = BufWriter::new(file);

                for hashmap in chunk {
                    let json_line = serde_json::to_string(hashmap).unwrap();
                    writeln!(&mut writer, "{}", json_line).unwrap();
                }
            });
        Ok(())
    }
}
#[doc(hidden)]
#[allow(dead_code)]
fn write_jsonl(data: Vec<String>, _bytes: usize, file_path: PathBuf) -> std::io::Result<()> {
    let folder = file_path.parent().unwrap().to_str().unwrap();
    let stem = file_path.file_stem().unwrap().to_str().unwrap();
    let extension = file_path.extension().unwrap().to_str().unwrap();

    let file_path = Path::new(folder).join(format!("{}_0.{}", stem, extension));
    let file = File::create(file_path)?;
    let handle = std::thread::spawn(move || {
        let mut writer = BufWriter::new(file);

        for json_line in data {
            // let json_line = serde_json::to_string(&hashmap).unwrap();
            writeln!(&mut writer, "{}", json_line).unwrap();
        }
    });
    handle.join().unwrap();
    Ok(())
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
/// let write_handle = std::thread::spawn(move || {
///    write_jsonl_receiver(rx, output_folder)
/// });
///
/// tx.send(String::from("Hello")).unwrap();
/// tx.send(String::from("World")).unwrap();
///
/// drop(tx);
/// write_handle.join().unwrap().unwrap();
/// ```
pub fn write_jsonl_receiver(
    receiver: Receiver<String>,
    output_folder: PathBuf,
) -> std::io::Result<()> {
    // Create a all.jsonl file
    let output_path = output_folder.join("all.jsonl");
    let mut writer = BufWriter::with_capacity(1_048_576, File::create(output_path)?);
    while let Ok(data) = receiver.recv() {
        writeln!(&mut writer, "{}", data)?;
    }
    writer.flush()?;
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
        let write_handle =
            std::thread::spawn(move || write_jsonl_receiver(rx, output_folder_clone));

        tx.send(String::from("Hello")).unwrap();
        tx.send(String::from("World")).unwrap();
        drop(tx);

        write_handle.join().unwrap().unwrap();

        let output_path = output_folder.join("all.jsonl");
        let contents = std::fs::read_to_string(output_path).unwrap();

        assert_eq!(contents, "Hello\nWorld\n");
    }
}
