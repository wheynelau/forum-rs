use crate::globals;
use crate::utils;

/// Text cleaning function
///
/// This function is used by the `process` function to clean the text
///
/// # Arguments
///
/// * `text` - A string reference
///
/// # Returns
///
/// * `String` - The cleaned text
///
/// # Example
///
/// ```
/// let text = "hello--world".to_string();
/// let cleaned_text = clean_text(&text);
/// assert_eq!(cleaned_text, "hello world");
/// ```
fn clean_text(text: String) -> String {
    let cleaned_text = globals::clean_content(&text);
    cleaned_text.trim().to_string()
}
/// Process the thread content
///
/// This function processes the thread content and returns a `ThreadPost` struct
///
/// # Arguments
///
/// * `thread_id` - `String` - The thread id. This is the root of the thread.
/// * `content` - `Vec<String>` - The content of the thread. This is the output from the DFS or BFS traversal,
///     where each element represents a String that is a content of either a thread or a comment. For more info, check the output of the [traverse](../../graph/struct.ThreadGraph.html#method.traverse) function.
/// * `forum_name` - `String` - The name of the forum. Used for tagging.
/// * `use_sentencepiece` - `bool` - Whether to use a tokenizer for counting the number of tokens. If this is set to false,
///     the function will count the number of words split by whitespace.
///
/// # Returns
///
/// * `ThreadPost` - The processed [ThreadPost](../writer/struct.ThreadPost.html) struct
///
/// # Example
///
/// ```
/// let thread_id = "1234".to_string();
/// let content = vec!["thread root".to_string(), "comment".to_string()];
/// let forum_name = "reddit".to_string();
/// let use_sentencepiece = true;
///
/// ```
pub fn process(
    thread_id: String,
    content: Vec<String>,
    forum_name: String,
    use_sentencepiece: bool,
) -> utils::writer::ThreadPost {
    let content: Vec<String> = content
        .into_iter()
        // .with_min_len(1000)
        .map(clean_text)
        .collect();
    let content = content.join("\n");
    let length: usize = match use_sentencepiece {
        true => globals::tokenize(&content).len(),
        false => content.split_whitespace().count(),
    };
    utils::writer::ThreadPost {
        length,
        raw_content: content,
        thread_id,
        source: forum_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex() {
        // Integration test for the regex

        // Add more test cases here
        let test_cases = vec![
            // Test dashes
            ("hello--world", "hello world"),
            ("multiple---dashes", "multiple dashes"),
            ("normal-dash", "normal-dash"), // single dash should remain
            // Test equals signs
            ("title==heading", "title heading"),
            ("multiple===equals", "multiple equals"),
            ("single=equals", "single=equals"), // single equals should remain
            // Test multiple spaces
            ("too    many    spaces", "too many spaces"),
            ("normal spaces", "normal spaces"),
            ("tabs\t\tand    spaces", "tabs and spaces"),
            // Test URLs
            ("check http://example.com here", "check here"),
            ("https://website.com/path", ""),
            ("mixed http://url.com and text", "mixed and text"),
            // Test @ mentions
            ("hello @username world", "hello world"),
            ("@user1 @user2 text", "text"),
            ("email@domain.com text", "text"),
            // Test hashtags
            ("#Hashtag5, #Hashtag2, #Hashtag  ", ""),
            // Test combinations
            ("@user http://example.com  ---separator", "separator"),
            (
                "complex   http://example.com   @user   case",
                "complex case",
            ),
            (
                "shouldn't #SPAM https://spam.com remove someone@spam.com this",
                "shouldn't remove this",
            ),
            // Edge cases
            ("", ""),    // empty string
            ("   ", ""), // only spaces
            ("---", ""), // only dashes
            ("===", ""), // only equals
        ];

        globals::init_regex();

        for (input, expected) in test_cases {
            let result = utils::processing::clean_text(input.to_string());
            println!("{} -> {}", input, result);
            assert_eq!(
                result, expected,
                "Failed on input: '{}'\nExpected: '{}'\nGot: '{}'",
                input, expected, result
            );
        }
    }
}
