use std::io::{self, Write};
use rand::seq::SliceRandom;

const WORDS: &[&str] = &[
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango",
    "uniform", "victor", "whiskey", "xray", "yankee", "zulu",
    "apple", "banana", "cherry", "grape", "lemon", "mango", "orange", "peach", "pear", "plum",
    "forest", "river", "mountain", "ocean", "desert", "island", "valley", "stream", "lake", "glacier",
    "cloud", "star", "moon", "sun", "earth", "sky", "wind", "rain", "snow", "thunder",
    "bright", "dark", "fast", "slow", "happy", "sad", "big", "small", "green", "blue",
    "red", "yellow", "white", "black", "silver", "golden", "crystal", "iron", "stone", "wooden",
    "fire", "water", "air", "land", "rock", "sand", "soil", "metal", "plant", "leaf",
];

pub fn generate_random_id_phrase() -> String {
    let mut rng = rand::thread_rng();

    let word1 = WORDS.choose(&mut rng).unwrap();
    let word2 = WORDS.choose(&mut rng).unwrap();
    let word3 = WORDS.choose(&mut rng).unwrap();

    format!("{}-{}-{}", word1, word2, word3)
}

pub fn get_input_with_default(prompt: &str, default_value: &str) -> String {
    print!("{} [default: {}]: ", prompt, default_value);
    io::stdout().flush().expect("Failed to flush stdout"); // Ensure prompt is displayed immediately

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read input line");

    let trimmed_input = input.trim();
    if trimmed_input.is_empty() {
        default_value.to_string()
    } else {
        trimmed_input.to_string()
    }
}