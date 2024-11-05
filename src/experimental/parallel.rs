use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};

use crate::experimental;
use crate::forum_thread;
use crate::graph;
use crate::utils;

pub fn _get_threads(path: &str) -> Vec<(String, Vec<String>)> {
    let entries = utils::file::single_folder(path);
    let threadgraph = Arc::new(Mutex::new(graph::ThreadGraph::new()));
    let comments = Arc::new(Mutex::new(Vec::with_capacity(10000)));

    // let loop_start = std::time::Instant::now();
    // this shouldn't be parallelized for safety
    entries.par_iter().for_each(|entry| {
        let fp = File::open(entry).unwrap();

        let reader = BufReader::new(fp);

        let threadgraph = Arc::clone(&threadgraph);
        let comments = Arc::clone(&comments);

        reader.lines().for_each(|line| {
            if let Ok(line) = line {
                let json: forum_thread::JsonStruct = serde_json::from_str(&line).unwrap();
                if let Some(thread) = forum_thread::Post::from_json_struct(json) {
                    // Lock the mutex only when needed
                    let mut graph = threadgraph.lock().unwrap();
                    let thread_node = graph.add_node(thread.clone());
                    if thread.is_thread {
                        graph.add_threads(thread_node);
                    } else {
                        drop(graph);
                        comments.lock().unwrap().push(thread);
                    };
                }
            }
        });
    });
    // add edges
    // println!("Time taken for loop: {:.2?}", loop_start.elapsed());
    let mut threadgraph = threadgraph.lock().unwrap();
    // let comment_time = std::time::Instant::now();
    let comments = comments.lock().unwrap();
    for comment in comments.iter() {
        threadgraph.add_edge(&comment.parent_post_id, &comment.id);
    }
    // println!("Time taken for comments: {:.2?}", comment_time.elapsed());
    // let traverse_time = std::time::Instant::now();
    threadgraph.traverse()
    // println!("Time taken for traverse: {:.2?}", traverse_time.elapsed());
    // threads
}

pub fn get_threads(path: &str) -> Vec<(String, Vec<String>)> {
    let entries = utils::file::single_folder(path);
    let threadgraph = Arc::new(Mutex::new(experimental::graph::ThreadGraph::new()));

    // let loop_start = std::time::Instant::now();
    // this shouldn't be parallelized for safety
    entries.par_iter().for_each(|entry| {
        let fp = File::open(entry).unwrap();

        let reader = BufReader::new(fp);

        let threadgraph = Arc::clone(&threadgraph);

        reader.lines().for_each(|line| {
            if let Ok(line) = line {
                let json: forum_thread::JsonStruct = serde_json::from_str(&line).unwrap();
                if let Some(thread) = forum_thread::Post::from_json_struct(json) {
                    // Lock the mutex only when needed
                    let mut graph = threadgraph.lock().unwrap();
                    graph.add_post(thread.clone());
                }
            }
        });
    });
    let threadgraph = threadgraph.lock().unwrap();

    threadgraph.traverse()
    // println!("Time taken for traverse: {:.2?}", traverse_time.elapsed());
    // threads
}

// Kept for reference
// fn rayon_scope () {

//     rayon::scope_fifo(|s| {
//         for folder in all_folders {
//             // Clone the values that need to be shared
//             let out_folder = out_folder.clone();
//             let source = source.clone();
//             let use_sentencepiece = use_sentencepiece.clone();
//             let counter = Arc::clone(&counter);
//             s.spawn_fifo(move |_| {
//                 process_folder(&folder, &out_folder, &use_sentencepiece, &source);
//                 counter.fetch_add(1, Ordering::SeqCst);
//             });
//         }
//     });
// }
