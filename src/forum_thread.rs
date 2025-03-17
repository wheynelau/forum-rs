use serde::{Deserialize, Serialize};

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
    use_sentencepiece: &bool,
    forum_name: &str,
    thread_receiver: crossbeam_channel::Receiver<(String, Vec<String>)>,
    sender_rx: crossbeam_channel::Sender<String>,
) {
    while let Ok((thread_id, content)) = thread_receiver.recv() {
        let threadpost =
            utils::processing::process(thread_id, content, forum_name, use_sentencepiece);
        // This sends after the processing
        if let Ok(json_str) = serde_json::to_string(&threadpost) {
            let _ = sender_rx.send(json_str);
        }
    }
}
//     if std::env::var("BENCHMARK").unwrap_or("0".to_string()) == *"1" {
//         for (thread_id, content) in threads {
//             let threadpost =
//                 utils::processing::process(&thread_id, &content, &forum_name, use_sentencepiece);
//             // This sends after the processing
//             if let Ok(json_str) = serde_json::to_string(&threadpost) {
//                 let _ = sender_rx.send(json_str);
//             }
//         }
//         return;
//     }
//     threads
//         .into_par_iter()
//         .with_min_len(50)
//         .for_each(|(thread_id, content)| {
//             let threadpost =
//                 utils::processing::process(&thread_id, &content, &forum_name, use_sentencepiece);
//             // This sends after the processing
//             if let Ok(json_str) = serde_json::to_string(&threadpost) {
//                 let _ = sender_rx.send(json_str);
//             }
//         });
// }
