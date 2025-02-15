// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::io::{BufRead, Read};
use std::process::exit;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

/// 读取标准输入的第一个字符，并丢弃该行其余内容
fn read_first_char_and_clear() -> Option<char> {
    // 锁定标准输入，获得一个缓冲读取器
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut input_line = String::new();

    // 读取一整行（包括第一个字符及后续所有字符）
    if reader.read_line(&mut input_line).is_ok() {
        // 返回该行的第一个字符（如果有的话）
        input_line.chars().next()
    } else {
        None
    }
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    // println!("random word: {}", secret_word);
    // Your code here! :)
    println!("Welcome to CS110L Hangman!");
    let mut flags = vec![false; secret_word.len()];
    let mut guessed = HashSet::new();
    let mut left = 5;
    loop {
        print!("The word so far is ");
        for i in 0..secret_word_chars.len() {
            if flags[i] {
                print!("{}", secret_word_chars[i]);
            } else {
                print!("-");
            }
        }
        println!();
        print!("You have guessed the following letters: ");
        for ch in guessed.iter() {
            print!("{} ", ch);
        }
        println!();
        println!("You have {} guesses left", left);
        let ch = read_first_char_and_clear().unwrap();
        guessed.insert(ch);
        let mut flag: bool = false;
        for i in 0..secret_word_chars.len() {
            if secret_word_chars[i] == ch {
                flags[i] = true;
                flag = true;
            }
        }
        if (!flag) {
            left -= 1;
            println!("Sorry, that letter is not in the word")
        }
        println!();
        let mut win: bool = true;
        for i in 0..secret_word_chars.len() {
            if (!flags[i]) {
                win = false;
                break;
            }
        }
        if (win) {
            println!(
                "Congratulations you guessed the secret word: {}",
                secret_word
            );
            exit(0);
        }
        if (left == 0) {
            break;
        }
    }
    println!("\nSorry, you ran out of guesses!");
}
