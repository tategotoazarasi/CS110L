use std::fs::File;
use std::io::BufRead;
use std::process;
use std::{env, io};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];
    let file = File::open(filename).unwrap();
    let reader = io::BufReader::new(file);
    let mut line_cnt = 0;
    let mut word_cnt = 0;
    let mut char_cnt = 0;
    let mut flag_prev_non_space = false;
    // Read character by character
    for line in reader.lines() {
        match line {
            Ok(line) => {
                line_cnt += 1;
                for c in line.chars() {
                    if (!c.is_whitespace()) {
                        char_cnt += 1;
                        flag_prev_non_space = true;
                    } else {
                        if (flag_prev_non_space) {
                            word_cnt += 1;
                        }
                        flag_prev_non_space = false;
                    }
                    //println!("{}", c);
                }
            }
            Err(_) => {}
        }
    }
    if (flag_prev_non_space) {
        word_cnt += 1;
    }
    println!("{}\t{}\t{}\t{}", line_cnt, word_cnt, char_cnt, filename);
}
