#![doc = include_str!("../README.md")]

use clap::Parser;
use crossbeam_channel::{unbounded, Sender};
use rayon::prelude::*;
use std::fs::{self};
use std::io::Write;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use std::time::{Duration, Instant};

/**

# Struct for the command line arguments

# Arguments

* `input` - The input folder containing the forum data
* `output` - The output folder where the processed data will be stored
* `tokenizer` - The tokenizer to use for tokenization
* `source` - The source of the data
* `safe` - Whether to overwrite the output folder or not

# Panics

* If the input folder does not exist
* If the output folder does not exist
* If the tokenizer is not valid
* If huggingface hub is not authorized


*/
pub mod args;

/**

# Module for the experimental functions

This module contains functions that may not produce the best performance but are experimental
*/
pub mod experimental;
pub mod forum_thread;
pub mod globals;
pub mod graph;
pub mod utils;

static TOTAL_TIME_GET_THREADS: AtomicU64 = AtomicU64::new(0);
static TOTAL_TIME_CREATE_POSTS: AtomicU64 = AtomicU64::new(0);
static TOTAL_TIME_WRITE_JSONL: AtomicU64 = AtomicU64::new(0);

/// Process the folder
///
/// What this function does:
/// 1. Get the threads from the folder
/// 2. Create the thread posts
/// 3. Write the thread posts to a file
///
/// # Arguments
///
/// * `folder` - `&Path` - The folder containing list of `jsonl` files
/// * `use_sentencepiece` - `&bool` - Whether to use sentencepiece for tokenization, the name does not mean that it
///     will use sentencepiece, it will use the tokenizer specified in the `tokenizer` argument.
/// * `source` - `&String` - The source of the data. This is just for labelling.
/// * `post_tx` - `Sender<String>` - The sender to send the String objects.
///
/// # Example
///
/// ```rust
/// use std::path::Path;
///
/// let folder = Path::new("main_folder/sub1/");
/// let use_sentencepiece = true;
/// let source = "reddit".to_string();
/// let (data_tx, data_rx) = unbounded();
/// process_folder(folder, &use_sentencepiece, &source, data_tx.clone());
///
/// ```
fn process_folder(
    folder: &Path,
    use_sentencepiece: &bool,
    source: &String,
    post_tx: Sender<String>,
) {
    // dbg!(&folder);
    let folder = folder.to_str().unwrap();

    let start = Instant::now();
    let threads: Vec<(String, Vec<String>)> = experimental::sender::get_threads(folder);
    let get_threads_time = start.elapsed().as_secs();
    TOTAL_TIME_GET_THREADS.fetch_add(get_threads_time, Ordering::SeqCst);

    let start = Instant::now();
    forum_thread::sender_thread_posts(threads, *use_sentencepiece, source.to_string(), post_tx);
    let create_posts_time = start.elapsed().as_secs();
    TOTAL_TIME_CREATE_POSTS.fetch_add(create_posts_time, Ordering::SeqCst);

    // if !posts.is_empty() {
    //     let start = Instant::now();
    //     let output_file: PathBuf = Path::new(&out_folder).join(format!("{}.jsonl", forum_id));
    //     utils::writer::write_jsonl(posts, bytes, output_file).unwrap();
    //     let write_jsonl_time = start.elapsed().as_secs();
    //     TOTAL_TIME_WRITE_JSONL.fetch_add(write_jsonl_time, Ordering::SeqCst);
    // }
}
///
/// Entry point of the program
///
/// This function will parse the arguments and start the processing of the folders
///
/// # Arguments
///
/// * `input` - The input folder containing the forum data
/// * `output` - The output folder where the processed data will be stored
/// * `tokenizer` - The tokenizer to use for tokenization
/// * `source` - The source of the data
/// * `safe` - Whether to overwrite the output folder or not
///     
/// # Example
///
/// ```bash
/// cargo run --release -- --input "reddit-graph/test_main_folder/" --output "./output/" \
///     --tokenizer "model-name" or "path-to-tokenizer.json" --source "reddit" --safe false
/// ```
///
/// # Note
///
/// Folders must contain subfolders with the forum data
/// ```plaintext
/// main_folder/  
/// ├── sub1/  
/// │   └── *.jsonl  
/// └── sub2/  
///     └── *.jsonl  
/// ```
///
/// The output folder will contain the processed data
/// ```plaintext
/// output/
/// ├── sub1.jsonl
/// └── sub2.jsonl
/// ```
fn main() -> std::io::Result<()> {
    let args = args::Cli::parse();
    let folder: String = args.input;
    let out_folder: String = args.output;
    let tokenizer: Option<String> = args.tokenizer;
    let source: String = args.source;
    let use_sentencepiece: bool = tokenizer.as_ref().is_some();

    // Initialize regex

    globals::init_regex();
    if let Some(tokenizer) = tokenizer {
        globals::init_tokenizer(&tokenizer);
    }
    // For safety, the output folder is not created if not found
    // Also if not empty, it will panic.
    if !args.safe {
        fs::create_dir_all(&out_folder).expect("Unable to create dir");
        println!("Folder has been created at `{}`", &out_folder)
    } else {
        let entries = fs::read_dir(&out_folder)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();
        if !entries.is_empty() {
            panic!("Output folder is not empty, you can run with `--safe false` to overwrite the files.");
        }
    }

    // let folder = "reddit-graph/test_main_folder/";
    // let out_folder : &str = "./output/";
    let all_folders: Vec<PathBuf> = utils::file::all_folders(&folder).unwrap();

    // Reorder the largest size first
    // This should speed up the parallel processing
    let all_folders = utils::file::reorder_by_size(all_folders);
    let total_folders = all_folders.len();

    // Before the par_iter loop:
    let counter = Arc::new(AtomicUsize::new(0));
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    let counter_clone = counter.clone();
    let start_time_clone = Instant::now();

    let (data_tx, data_rx) = unbounded();
    let data_rx_clone = data_rx.clone();
    // Spawn progress display thread
    let progress_thread = std::thread::spawn(move || {
        while running_clone.load(Ordering::SeqCst) {
            let count = counter_clone.load(Ordering::SeqCst);
            print!(
                "\rProcessed {}/{} folders. Queue to write: {}. Current duration: {:2}m {:.2}s",
                count,
                total_folders,
                data_rx_clone.len(),
                start_time_clone.elapsed().as_secs() / 60,
                start_time_clone.elapsed().as_secs() % 60
            );
            std::io::stdout().flush().unwrap();
            std::thread::sleep(Duration::from_millis(500));
        }
        // One final update after completion
        let count = counter_clone.load(Ordering::SeqCst);
        println!(
            "\rProcessed {}/{} folders. Queue to write: {}.  Current duration: {:2}m {:.2}s",
            count,
            total_folders,
            data_rx_clone.len(),
            start_time_clone.elapsed().as_secs() / 60,
            start_time_clone.elapsed().as_secs() % 60
        );
    });
    rayon::spawn(move || {
        if let Err(e) = utils::writer::write_jsonl_receiver(data_rx, out_folder.into()) {
            eprintln!("Error writing JSONL: {}", e);
        }
    });
    all_folders.par_iter().for_each(|folder| {
        process_folder(folder, &use_sentencepiece, &source, data_tx.clone());
        counter.fetch_add(1, Ordering::SeqCst);
    });
    drop(data_tx);
    // Wait for the receiver to finish
    println!("Completed processing all folders");

    // After the loop completes, stop the progress thread
    running.store(false, Ordering::SeqCst);
    progress_thread.join().unwrap();

    println!();
    let num_threads: u64 = rayon::current_num_threads() as u64;
    println!(
        "Total time taken for get_threads: {:.2}s",
        TOTAL_TIME_GET_THREADS.load(Ordering::SeqCst) / num_threads
    );
    println!(
        "Total time taken for create_posts: {:.2}s",
        TOTAL_TIME_CREATE_POSTS.load(Ordering::SeqCst) / num_threads
    );
    println!(
        "Total time taken for write_jsonl: {:.2}s",
        TOTAL_TIME_WRITE_JSONL.load(Ordering::SeqCst) / num_threads
    );

    Ok(())
}
#[cfg(test)]
mod main_tests {
    use std::collections::HashSet;

    use super::*;
    use pretty_assertions::assert_eq;
    #[test]
    fn test_path() {
        let initial_path = Path::new("forum_folder/output/something.jsonl");
        let folder = initial_path.parent().unwrap().to_str().unwrap();
        let stem = initial_path.file_stem().unwrap().to_str().unwrap();
        let extension = initial_path.extension().unwrap().to_str().unwrap();
        let new_file = format!("{}/{}_new.{}", folder, stem, extension);
        assert_eq!(new_file, "forum_folder/output/something_new.jsonl");
    }

    // TODO: Add the test for this integration test
    #[test]
    #[ignore]
    fn test_threads_integration() {
        // this needs to have a folder with jsonl files
        let folder = "test_data/";
        // Skip test if file is not found
        if !Path::new(folder).exists() {}
        globals::init_regex();
        let folder = String::from(folder);
        let threads: Vec<(String, Vec<String>)> = experimental::parallel::get_threads(&folder);
        let previous_implementation = experimental::parallel::_get_threads(&folder);
        let sender_threads: Vec<(String, Vec<String>)> = experimental::sender::get_threads(&folder);

        assert_eq!(threads.len(), 42);
        assert_eq!(previous_implementation.len(), 42);
        assert_eq!(sender_threads.len(), 42);

        // check if roots are same
        let mut sender_roots: HashSet<String> = HashSet::new();
        let mut parallel_roots: HashSet<String> = HashSet::new();

        for (root, _) in threads.iter() {
            sender_roots.insert(root.clone());
        }

        for (root, _) in previous_implementation.iter() {
            parallel_roots.insert(root.clone());
        }

        assert_eq!(sender_roots, parallel_roots);
    }
}
