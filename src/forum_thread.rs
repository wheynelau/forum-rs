use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

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
    use_sentencepiece: &bool,
    forum_name: String,
    sender_rx: crossbeam_channel::Sender<String>,
) {
    threads
        .into_par_iter()
        .with_min_len(50)
        .for_each(|(thread_id, content)| {
            let threadpost = utils::processing::process(
                &thread_id,
                &content,
                &forum_name,
                use_sentencepiece,
            );
            // This sends after the processing
            if let Ok(json_str) = serde_json::to_string(&threadpost) {
                let _ = sender_rx.send(json_str);
            }
        });
}

/// Creates a Vector of BTreeMap for the JSONL file
pub fn create_thread_posts(
    _forum_id: &str,
    threads: Vec<(String, Vec<String>)>,
    use_sentencepiece: &bool,
    forum_name: String,
) -> (Vec<String>, usize) {
    let byte_counter = AtomicUsize::new(0);
    let posts = if threads.len() > 5000 {
        // Parallel processing for large number of threads
        let mut posts: Vec<String> = Vec::with_capacity(threads.len());
        threads
            .into_par_iter()
            .map(|(thread_id, content)| {
                let threadpost = utils::processing::process(
                    &thread_id,
                    &content,
                    &forum_name,
                    use_sentencepiece,
                );
                byte_counter.fetch_add(threadpost.raw_content.len(), Ordering::Relaxed);
                serde_json::to_string(&threadpost).unwrap_or_default()
            })
            .collect_into_vec(&mut posts);

        // Shrink to fit to release unused memory
        posts.shrink_to_fit();
        posts
    } else {
        // Sequential processing for smaller number of threads
        let mut posts: Vec<String> = threads
            .into_iter()
            .map(|(thread_id, content)| {
                let threadpost = utils::processing::process(
                    &thread_id,
                    &content,
                    &forum_name,
                    use_sentencepiece,
                );
                byte_counter.fetch_add(threadpost.raw_content.len(), Ordering::Relaxed);
                serde_json::to_string(&threadpost).unwrap_or_default()
            })
            .collect();

        // Shrink to fit to release unused memory
        posts.shrink_to_fit();
        posts
    };

    (posts, byte_counter.into_inner())
}
