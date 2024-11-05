use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::graph;
use crate::utils;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct JsonStruct {
    id: String,
    is_thread: String,
    pagetext: String,
    parent_post_id: String,
    root_post_id: String,
}
#[derive(Clone, Debug, Default)]
pub struct Post {
    pub id: String,
    pub is_thread: bool,
    pub pagetext: String,
    pub parent_post_id: String,
    pub root_post_id: String,
}

impl Post {
    pub fn new<I: Into<String>>(
        id: I,
        is_thread: bool,
        pagetext: I,
        parent_post_id: I,
        root_post_id: I,
    ) -> Self {
        Post {
            id: id.into(),
            is_thread,
            pagetext: pagetext.into(),
            parent_post_id: parent_post_id.into(),
            root_post_id: root_post_id.into(),
        }
    }

    pub fn placeholder(id: String) -> Self {
        Post {
            id: id.clone(),
            is_thread: true,
            pagetext: "".to_string(),
            parent_post_id: id.clone(),
            root_post_id: id,
        }
    }
    pub fn from_json_struct(json: JsonStruct) -> Option<Self> {
        Some(Post {
            id: json.id,
            is_thread: json.is_thread == "Y",
            pagetext: json.pagetext,
            parent_post_id: json.parent_post_id,
            root_post_id: json.root_post_id,
        })
    }
}

pub fn sender_thread_posts(
    threads: Vec<(String, Vec<String>)>,
    use_sentencepiece: bool,
    forum_name: String,
    sender_rx: crossbeam_channel::Sender<String>,
) {
    // Parallel processing for large number of threads
    threads
        .par_iter()
        .with_min_len(50)
        .for_each(|(thread_id, content)| {
            let threadpost = utils::processing::process(
                thread_id.to_string(),
                content.to_vec(),
                forum_name.to_string(),
                use_sentencepiece,
            );
            sender_rx
                .send(serde_json::to_string(&threadpost).unwrap())
                .unwrap();
        });
}

/// Creates a Vector of BTreeMap for the JSONL file
pub fn create_thread_posts(
    _forum_id: &str,
    threads: Vec<(String, Vec<String>)>,
    use_sentencepiece: bool,
    forum_name: String,
) -> (Vec<String>, usize) {
    let byte_counter = AtomicUsize::new(0);
    let posts = if threads.len() > 5000 {
        // Parallel processing for large number of threads
        let mut posts: Vec<String> = Vec::with_capacity(threads.len());
        threads
            .par_iter()
            .map(|(thread_id, content)| {
                let threadpost = utils::processing::process(
                    thread_id.to_string(),
                    content.to_vec(),
                    forum_name.to_string(),
                    use_sentencepiece,
                );
                byte_counter.fetch_add(threadpost.raw_content.len(), Ordering::Relaxed);
                serde_json::to_string(&threadpost).unwrap()
            })
            .collect_into_vec(&mut posts);
        posts
    } else {
        // Sequential processing for smaller number of threads
        let posts: Vec<String> = threads
            .iter()
            .map(|(thread_id, content)| {
                let threadpost = utils::processing::process(
                    thread_id.to_string(),
                    content.to_vec(),
                    forum_name.to_string(),
                    use_sentencepiece,
                );
                byte_counter.fetch_add(threadpost.raw_content.len(), Ordering::Relaxed);
                serde_json::to_string(&threadpost).unwrap()
            })
            .collect();
        posts
    };

    (posts, byte_counter.into_inner())
}

#[allow(dead_code)]
fn get_threads(path: &str) -> Vec<(String, Vec<String>)> {
    let entries = utils::file::single_folder(path);
    let mut threadgraph = graph::ThreadGraph::new();
    let mut comments: Vec<Post> = Vec::with_capacity(10000);

    // let loop_start = std::time::Instant::now();
    // this shouldn't be parallelized for safety
    entries.iter().for_each(|entry| {
        let fp = File::open(entry).unwrap();
        let reader = BufReader::new(fp);
        let threads: Vec<Post> = reader
            .lines()
            .par_bridge()
            .filter_map(|line| line.ok())
            .filter_map(|line| {
                serde_json::from_str::<JsonStruct>(&line)
                    .ok()
                    .and_then(Post::from_json_struct)
            })
            .collect();

        for thread in threads {
            let thread_node = threadgraph.add_node(thread.clone());
            match thread.is_thread {
                true => threadgraph.add_threads(thread_node),
                false => comments.push(thread),
            }
        }
    });

    //println!("Time taken for loop: {:.2?}", loop_start.elapsed());
    // add edges
    // let comments = comments.lock().unwrap();
    // let mut threadgraph = threadgraph.lock().unwrap();
    // let comment_time = std::time::Instant::now();
    // for comment in comments.iter() {
    //     threadgraph.add_edge(&comment.parent_post_id, &comment.id);
    // }
    // println!("Time taken for comments: {:.2?}", comment_time.elapsed());
    // let traverse_time = std::time::Instant::now();
    threadgraph.traverse()
    // println!("Time taken for traverse: {:.2?}", traverse_time.elapsed());
    // threads
}
