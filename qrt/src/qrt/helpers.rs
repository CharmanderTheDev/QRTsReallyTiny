use super::structs::*;

pub fn unwrap_evaluation(error: Evaluation, showstack: bool, showmap: bool) -> Option<Var> {
    let (msg, on, lineon, stack, map) = match error {
        Ok(v) => return Some(v),
        Err((msg, on, lineon, stack, map)) => (msg, on, lineon, stack, map),
    };

    if showmap {
        println!("\n\nVARIABLE MAP:");
        for alias in map.into_iter() {
            print!("{}: ", alias.0);
            println!("{}", alias.1.represent())
        }
    }

    if showstack {
        println!("\n\nSTACK DUMP: ");
        for element in stack.into_iter().rev() {
            println!("{}", element.represent())
        }
    }

    println!(
        "\n\nERROR WHEN EXECUTING QRT CODE ON LINE {} AND CHARACTER {}:",
        lineon, on
    );
    println!("{}", msg);

    None
}

pub fn unpack_operator(packed: &Abstract) -> Option<u8> {
    match packed {
        Abstract::Operator(o) => Some(*o),
        _ => None,
    }
}

//Helper function, used to find the end of secondary args. Expects to start the character directly after the first bracket.
//Returns the position directly after the pairing bracket.
pub fn find_bracket_pair(program: &[u8], mut on: usize) -> usize {
    let (mut bracket_number, mut gestalt, mut escape) = (1, false, false);

    while bracket_number != 0 {
        match program[on] {
            //Matches for opening brackets
            b'{' => {
                if !gestalt {
                    bracket_number += 1;
                }
            }

            //Matches for closing brackets
            b'}' => {
                if !gestalt {
                    bracket_number -= 1;
                }
            }

            //Matches for quotes (gestalt initiation, termination, or escaped quote)
            b'"' => {
                if !gestalt {
                    gestalt = true;
                } else if !escape {
                    gestalt = false;
                }
            }

            //Matches for backslashes (escape initiation or escaped backslash)
            b'\\' => {
                if !escape {
                    escape = true
                }
            }

            //Matches for other characters and unescapes
            _ => {
                if escape {
                    escape = false
                }
            }
        }

        on += 1;
    }

    on
}