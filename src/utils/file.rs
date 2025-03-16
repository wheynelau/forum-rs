use std::fs;
use std::io;
use std::path::PathBuf;

/// This returns the subfolders in a specified folder, do not use this direct output for
/// the main function, as it does not provide a `Vec<jsonl path>`.
///
/// For simplicity, additional parent folders are not accounted for, and no recursion is done.
/// The folder structure should be as follows:
///
/// main_folder
///|-- test_folder
///|   |-- 10.jsonl
///|   |-- 11.jsonl
///|   |-- 12.jsonl
///
/// * `forum_folder` - A string reference to the main folder
///
/// # Example
///
/// ```
/// let all_folders = all_folders("forum_folder");
/// ```
///
pub fn all_folders(forum_folder: &str) -> Result<Vec<PathBuf>, io::Error> {
    let subfolders = fs::read_dir(forum_folder)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    // Print total subfolders
    println!("Total subfolders: {}", &subfolders.len());
    Ok(subfolders)
}

/// Get all files in a forum subfolder
///
/// The folder should contain JSONL files for the downstream tasks
/// folder
/// |-- jsonl
/// |-- jsonl
/// `|-- jsonl
///
/// # Example
///
/// ```
/// let entries = single_folder("forum/subforum");
/// ```
pub fn single_folder(folder: &str) -> Vec<PathBuf> {
    fs::read_dir(folder)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap()
}

/// Get the size of a folder
/// Does not handle recursion
///
/// # Example
///
/// ```
/// let size = folder_size(&PathBuf::from("forum_folder")).unwrap();
/// ```
fn folder_size(folder: &PathBuf) -> Result<u64, io::Error> {
    let mut size: u64 = 0;

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::metadata(&path)?;

        if metadata.is_file() {
            size += metadata.len();
        } else if metadata.is_dir() {
            panic!(
                "There should not be any subfolders in the main folder {:?}",
                path
            );
        }
    }

    Ok(size)
}
/// Sort by largest first
///
/// This takes a vector of paths and sorts them by size, largest first.
/// It does not handle recursion for the inner folders.
///
/// # Arguments
///
/// * `folder` - `Vec<PathBuf>` - The folder to sort
///
/// # Returns
///
/// * `Vec<PathBuf>` - The sorted folder
///
///
/// # Example
///
/// ```
/// let sorted_folder = reorder_by_size(folder);
/// ```
#[allow(dead_code)]
pub fn reorder_by_size(mut folder: Vec<PathBuf>) -> Vec<PathBuf> {
    folder.sort_by_cached_key(|path| {
        // Use a default size of 0 if there's an error calculating the folder size
        folder_size(path).unwrap_or(0)
    });

    // Reverse to get largest first
    folder.reverse();

    folder
}
