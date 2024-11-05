/**
# Parallel module

This module uses `par_iter` for every `jsonl` file in the folder.

*/
pub mod parallel;

/**

# Sender module

This module utilizes the `crossbeam` library to send the threads to the `graph` module.

Contains the function `process_graph` that processes objects sent by the iterator.
*/
pub mod sender;

pub mod graph;
