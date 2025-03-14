use std::{env, fs, vec::Vec};

mod qrt;
use qrt::{evaluate::evaluate, helpers::unwrap_evaluation, structs::Var};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        println!(
            "Not enough arguments provided. Please provide arguments in the following order:
QRT file name (no extension), debug number (0-3)"
        );
        return;
    }

    let debug: i32 = if let Ok(i) = &args[2].parse() { *i } else { 0 };

    let (showstack, showmap) = match debug {
        0 => (false, false),
        1 => (true, false),
        2 => (false, true),
        3 => (true, true),
        _ => (false, false),
    };

    let file: Vec<u8> = if let Ok(s) = fs::read_to_string(format!("{}.qrt", &args[1])) {
        s.into_bytes()
    } else {
        println!("No such QRT file found");
        return;
    };

    unwrap_evaluation(evaluate(&file, &Var::Void), showstack, showmap);
}
