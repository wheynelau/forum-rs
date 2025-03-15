use crossbeam_channel::{unbounded, Receiver, Sender};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::forum_thread;
use crate::graph;
use crate::utils;

#[allow(dead_code)]
fn process_line(rx: Receiver<String>, tx: Sender<forum_thread::Post>) {
    while let Ok(line) = rx.recv() {
        if let Ok(json) = serde_json::from_str::<forum_thread::JsonStruct>(&line) {
            if let Some(thread) = forum_thread::Post::from_json_struct(json) {
                tx.send(thread).unwrap();
            }
        }
    }
}

/**

# Process graph

Needs to be launched by a thread.

# Arguments

* `rx` - The receiver channel
* `threadgraph` - The thread graph

# Returns

* `ThreadGraph` - The thread graph
* `Vec<forum_thread::Post>` - The comments

# Example

This is very simple example. `thread_post_from_line` is a function
that parses a line into a `forum_thread::Post` struct.

```plaintext
use crossbeam_channel::{unbounded, Receiver};
let (post_tx, post_rx) = unbounded();
let graph_handle = std::thread::spawn(move || process_graph(post_rx, threadgraph, comments));

// Iterate over the entries

for line in reader.lines() {

    // Parse the line into the appropriate struct
    let thread = thread_post_from_line(line);

    post_tx.send(thread).unwrap();
}

// Drop the sender to signal the end of the stream
drop(post_tx);

```
*/
fn process_graph(rx: Receiver<forum_thread::Post>) -> graph::ThreadGraph {
    let mut threadgraph = graph::ThreadGraph::new();
    while let Ok(thread) = rx.recv() {
        threadgraph.add_post(thread);
    }
    threadgraph
}

pub fn get_threads(path: &str) -> Vec<(String, Vec<String>)> {
    let entries = utils::file::single_folder(path);
    let (post_tx, post_rx) = unbounded();
    // let (string_tx, string_rx) = bounded(1000);

    // let line_handle = std::thread::spawn(move || {
    //     process_line(string_rx, post_tx);
    // });

    let graph_handle = std::thread::spawn(move || process_graph(post_rx));
    // let threadgraph = Arc::new(Mutex::new(graph::ThreadGraph::new()));
    // let comments = Arc::new(Mutex::new(Vec::with_capacity(10000)));
    // this shouldn't be parallelized for safety
    entries.par_iter().for_each(|entry| {
        let fp = File::open(entry).unwrap();
        let reader = BufReader::new(fp);

        reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| {
                serde_json::from_str::<forum_thread::JsonStruct>(&line)
                    .ok()
                    .and_then(forum_thread::Post::from_json_struct)
            })
            .for_each(|post| {
                post_tx.send(post).unwrap();
            });
    });

    // Drop the sender to signal the end of the stream
    drop(post_tx);

    // Wait for the graph processing to complete
    let threadgraph = graph_handle.join().unwrap();
    threadgraph.traverse()
}
