# Forum rs

To use the rust documentation, you can use the following command. This assumes you have rust installed.  
```bash
cargo doc --open
```

## Pre-requisites

- Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

This works for non root users as well.
- Other packages (Linux tested)
```bash
sudo apt install libssl-dev pkg-config build-essential
```

Single block:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install libssl-dev pkg-config build-essential
```

## Usage

1. Clone this branch
2. chdir to this directory
```bash
cd forum-rs
```
3. Run the following command to run, the initial build might take some time
```bash
# Run with tokenizer
cargo run --release -- --input ./test_data/ --output output --safe false --tokenizer meta-llama/Meta-Llama-3-8B
# Run without tokenizer
cargo run --release -- --input ./test_data/ --output output --safe false
```
Alternatively, you can install and run it anywhere  
```bash
cargo install --path .
clean-reddit --input ./test_data/ --output output --safe false
```
## Additional info:

### Potential issues

403 Error - when using tokenizers: It seems like rust tries to find the token so even using HF_TOKEN does not work.  
The best workaround is to provide the `tokenizer.json` file.

Job killed - this is due to the large memory usage of the program. You can reduce the number of threads to reduce memory usage. See below.

### Huggingface
Depending on the tokenizer used, you may need to run `EXPORT HF_TOKEN=your_token` to set the token for the huggingface library.  
Alternatively, you can download the `tokenizer.json` file and run it with the following command

```bash
cargo run --release -- --input ./test_data/ --output output --safe false --tokenizer tokenizer.json
```
### Parallelism  

The code will run in parallel by default. To reduce parallelism, use this environment variable
```bash
export RAYON_NUM_THREADS=32
```

You can use `--help` to get the following information
```bash
cargo run --release -- --help

Struct for CLI

Usage: clean-reddit [OPTIONS] --input <INPUT> --output <OUTPUT>

Options:
  -i, --input <INPUT>
          Input to the root folder, internally must be in format main/subreddit/*.jsonl

  -o, --output <OUTPUT>
          Output folder for the JSONL files, will write the jsonl as subreddit.jsonl

  -t, --tokenizer <TOKENIZER>
          Tokenizer name .If not provided, will split and count words

  -s, --safe <SAFE>
          If true, will not overwrite existing files, default is true
          
          [default: true]
          [possible values: true, false]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## TODO

Move experimental codes into the main `rs` files.