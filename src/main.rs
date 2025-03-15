#![doc = include_str!("../README.md")]

use clap::Parser;
use crossbeam_channel::{unbounded, Sender};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::{self};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::runtime::Runtime;

use std::time::{Duration, Instant};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

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
fn process_folder(folder: &Path, use_sentencepiece: &bool, source: &str, post_tx: Sender<String>) {
    // dbg!(&folder);
    let folder = folder.to_str().unwrap();

    let start = Instant::now();
    let threads: Vec<(String, Vec<String>)> = experimental::sender::get_threads(folder);
    let get_threads_time = start.elapsed().as_secs();
    TOTAL_TIME_GET_THREADS.fetch_add(get_threads_time, Ordering::SeqCst);

    let start = Instant::now();
    forum_thread::sender_thread_posts(threads, use_sentencepiece, source.to_string(), post_tx);
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
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
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
            .expect("Unable to read dir")
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .expect("Error collecting entries");
        if !entries.is_empty() {
            panic!("Output folder is not empty, you can run with `--safe false` to overwrite the files.");
        }
    }

    // let folder = "reddit-graph/test_main_folder/";
    // let out_folder : &str = "./output/";
    let all_folders: Vec<PathBuf> =
        utils::file::all_folders(&folder).expect("Unable to get all folders");

    // Reorder the largest size first
    // This should speed up the parallel processing
    // let all_folders = utils::file::reorder_by_size(all_folders);
    let total_folders = all_folders.len();
    println!("First folder: {:?}", all_folders[0]);

    // Before loop
    let counter = Arc::new(AtomicUsize::new(0));
    let (data_tx, data_rx) = unbounded();
    let data_rx_clone = data_rx.clone();

    // Create a progress bar
    let pb = ProgressBar::new(total_folders as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} folders {msg}")
            .unwrap()
    );

    // Clone progress bar and wrap in Arc for thread-safe updates
    let pb_clone = Arc::new(pb);
    let pb_thread = pb_clone.clone();

    // Create and use a tokio runtime for async tasks
    let rt = Runtime::new().expect("Unable to create tokio runtime");
    let out_folder_path = PathBuf::from(out_folder);

    // Spawn the async task for writing JSONL data
    rt.spawn(async move {
        if let Err(e) = utils::writer::write_jsonl_receiver(data_rx, out_folder_path).await {
            eprintln!("Error writing JSONL: {}", e);
        }
    });

    // Create a clone of counter for the update thread
    let counter_clone = counter.clone();

    // Spawn a thread to periodically update the progress bar with queue size
    let update_thread = std::thread::spawn(move || {
        while !data_rx_clone.is_empty() || counter_clone.load(Ordering::SeqCst) < total_folders {
            pb_thread.set_message(format!("Queue: {}", data_rx_clone.len()));
            std::thread::sleep(Duration::from_millis(500));
        }
    });

    // Use rayon's parallel iterator for folder processing
    all_folders.into_par_iter().for_each(|folder| {
        process_folder(&folder, &use_sentencepiece, &source, data_tx.clone());
        let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
        pb_clone.set_position(count as u64);
    });

    drop(data_tx);
    // Wait for the receiver to finish
    println!("Completed processing all folders");

    // Finish the progress bar
    pb_clone.finish_with_message("Processing complete");

    // Wait for the update thread to finish
    if let Err(e) = update_thread.join() {
        eprintln!("Error joining update thread: {:?}", e);
    }

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

    // Wait for a moment to ensure all async tasks complete
    std::thread::sleep(Duration::from_millis(100));

    Ok(())
}
#[cfg(test)]
mod main_tests {

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
}
