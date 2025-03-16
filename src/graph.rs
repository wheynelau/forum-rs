use crate::forum_thread::Post;
use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;
use petgraph::Graph;
use rayon::prelude::*;
use std::collections::HashMap;

///
/// ThreadGraph is a struct that represents a graph of threads and comments
///
#[derive(Default)]
pub struct ThreadGraph {
    graph: Graph<String, (), petgraph::Directed>,
    node_map: HashMap<String, NodeIndex>,
    threads: Vec<NodeIndex>,
    allthreads: Vec<Post>,
}
impl ThreadGraph {
    /// Constructs a new `ThreadGraph` with a `DiGraph` and a `HashMap`
    ///
    /// # Returns
    ///
    /// * `ThreadGraph` - A new `ThreadGraph`
    ///
    /// # Example
    ///
    /// ```
    /// let threadgraph = ThreadGraph::new();
    /// ```
    pub fn new() -> Self {
        ThreadGraph {
            graph: Graph::new(),
            node_map: HashMap::with_capacity(10000),
            threads: Vec::with_capacity(10000),
            allthreads: Vec::with_capacity(10000),
        }
    }
    /// Adds a node to the graph and returns the index
    ///
    /// If the node already exists, it will return the index of the existing node,
    /// otherwise it will add the node to the graph and return the index. At the same time,
    /// it adds the node into the `allthreads` vector and the `node_map` hashmap.
    ///
    /// # Arguments
    ///
    /// * `id` - `&String` - The id of the post
    ///
    /// # Returns
    ///
    /// * `NodeIndex` - The index of the node
    ///
    /// # Example
    ///
    /// ```rust
    /// let post = Post::default();
    /// let idx = threadgraph.add_node(&post.id);
    ///
    /// assert_eq!(idx.index(), 0);
    /// ```
    fn add_node(&mut self, id: &String) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(id) {
            idx
        } else {
            let idx = self.graph.add_node(id.clone());
            self.allthreads.push(Post::default());
            self.node_map.insert(id.to_string(), idx);
            idx
        }
    }
    /// Adds an edge to the graph
    ///
    /// This function will add an edge from `from_id` to `to_id` if `from_id` exists in the graph.
    /// If it doesn't exist, it will create a placeholder post and add it to the graph.
    ///
    /// The reason for the above implementation is due to some of the threads being detached from the main thread.
    pub fn add_post(&mut self, post: Post) {
        // Every id should be unique, use this to update the node_map
        let from_idx = self.add_node(&post.parent_post_id);
        let to_id = self.add_node(&post.id);
        self.allthreads[to_id.index()] = post;
        if from_idx == to_id {
            return;
        }
        self.graph.add_edge(from_idx, to_id, ());
    }

    pub fn show_roots(&self) -> Vec<NodeIndex> {
        let mut roots_idx: Vec<NodeIndex> = Vec::new();
        for node in self.graph.node_indices() {
            let incoming_count = self
                .graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .count();
            if incoming_count == 0 {
                roots_idx.push(node);
            }
        }
        roots_idx
    }
    fn single_dfs(&self, start: &NodeIndex) -> (String, Vec<String>) {
        // skip if not root
        let mut dfs = Dfs::new(&self.graph, *start);
        let mut threads: Vec<usize> = Vec::with_capacity(50); // Pre-allocate with a reasonable size

        while let Some(visited) = dfs.next(&self.graph) {
            threads.push(visited.index());
        }
        let root_id = self.graph[*start].clone();
        let vec_string: Vec<String> = threads
            .iter()
            .map(|thread| self.allthreads[*thread].pagetext.clone())
            .collect();
        let vec_string = {
            let mut vs = vec_string;
            vs.shrink_to_fit();
            vs
        };
        (root_id, vec_string)
    }
    /// Traverse the graph and return a vector of threads
    ///
    /// # Returns
    ///
    /// * `Vec<(String, Vec<String>)>` - A vector of tuples where the first element is the root post id and the second element is a vector of pagetext
    ///
    /// # Example
    ///
    /// ```
    /// let threads = threadgraph.traverse();
    /// threads[0].0 // root post id
    /// threads[0].1 // vector of pagetext
    /// ```
    pub fn traverse(&mut self) -> Vec<(String, Vec<String>)> {
        let roots = self.show_roots();

        let mut final_threads: Vec<(String, Vec<String>)> = Vec::with_capacity(self.threads.len());
        if std::env::var("BENCHMARK").unwrap_or("0".to_string()) == *"1" {
            roots.iter().for_each(|start| {
                final_threads.push(self.single_dfs(start));
            });
        } else {
            roots
                .par_iter()
                .with_min_len(100)
                .map(|start| self.single_dfs(start))
                .collect_into_vec(&mut final_threads);
        }
        // explicit clear
        self.threads.clear();
        self.allthreads.clear();
        self.node_map.clear();
        final_threads
    }

    pub fn show_threads(&self) {
        for node in self.graph.node_indices() {
            println!("{:?}", self.graph[node]);
        }
    }
    pub fn add_threads(&mut self, idx: NodeIndex) {
        self.threads.push(idx);
    }
    pub fn is_in_map(&self, id: &String) -> bool {
        self.node_map.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::izip;
    use pretty_assertions::assert_eq;
    use rand::prelude::*;

    fn setup() -> (ThreadGraph, Vec<Post>) {
        let test_cases = vec![
            // first graph, basic 1>2&3
            ("1", true, "1", "1", "1"),
            ("2", true, "2", "2", "2"),
            ("3", false, "3", "1", "1"),
            ("4", false, "4", "3", "3"),
            ("5", false, "5", "3", "3"),
            ("6", false, "6", "4", "4"),
            ("7", false, "7", "2", "2"),
            ("8", false, "8", "7", "7"),
            ("9", false, "9", "7", "7"),
            ("10", false, "10", "8", "8"),
            // detached thread
            ("11", false, "11", "12", "12"),
        ];

        let graph = ThreadGraph::new();
        let posts = test_cases
            .into_iter()
            .map(|(id, is_thread, pagetext, parent_post_id, root_post_id)| {
                Post::new(id, is_thread, pagetext, parent_post_id, root_post_id)
            })
            .collect();
        (graph, posts)
    }

    /// Test the basic functionality of the graph
    ///
    #[test]
    fn test_functional_graph() {
        // TODO: There should be a more idiomatic way to do this
        // assumes dfs
        let mut target: Vec<(&str, Vec<&str>)> = vec![
            ("2", vec!["2", "7", "9", "8", "10"]),
            ("1", vec!["1", "3", "5", "4", "6"]),
            ("12", vec!["", "11"]),
        ];
        let mut alternative_target: Vec<(&str, Vec<&str>)> = vec![
            ("2", vec!["2", "7", "8", "10", "9"]),
            ("1", vec!["1", "3", "4", "6", "5"]),
            ("12", vec!["11", ""]),
        ];
        // sort target
        target.sort_by(|a, b| a.0.cmp(b.0));
        alternative_target.sort_by(|a, b| a.0.cmp(b.0));

        // run a loop for better determinism
        for _ in 0..10 {
            let (mut graph, mut posts) = setup();
            posts.shuffle(&mut thread_rng());
            for post in posts.iter() {
                graph.add_post(post.clone());
            }

            assert_eq!(graph.graph.node_count(), 12);
            assert_eq!(graph.graph.edge_count(), 9);

            let mut threads = graph.traverse();
            threads.sort_by(|a, b| a.0.cmp(&b.0));

            assert_eq!(threads.len(), target.len());

            // check against target and alternative target
            for (result, x, y) in izip!(threads, &target, &alternative_target) {
                assert_eq!(result.0, x.0);
                assert!(result.1 == x.1 || result.1 == y.1);
            }
        }
    }
}
