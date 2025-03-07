extern crate rand;

use rand::random;
use std::{
    collections::{HashMap, VecDeque},
    env, fs,
    vec::Vec,
};

fn main() {
    let args: Vec<String> = env::args().collect();

    let file: Vec<u8> = fs::read_to_string(&args[1])
        .expect("No such file found")
        .trim()
        .as_bytes()
        .into();

    let evaluation = evaluate(&file, &Var::Linear(5.0));

    println!("{:?}", evaluation);
}

fn qlog() {
    println!("log!");
}

#[derive(Clone, Debug)]
enum Var {
    Void,             //Null type
    Linear(f64),      //Numbers
    Gestalt(Vec<u8>), //Strings
    Set(Vec<Var>),    //Lists
}
impl Var {
    fn bool(&self) -> bool {
        match self {
            Var::Linear(a) => {
                if a > &0.0 {
                    return true;
                }
                false
            }

            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
enum Abstract {
    Var(Var),     //Values
    Operator(u8), //Generic operators, also include "loops" that haven't been initialized yet.
    Loop(usize), //Loops are a special operator that require metadata pointing to their start location
}

fn evaluate(program: &Vec<u8>, input: &Var) -> Var {
    let mut stack: VecDeque<Abstract> = VecDeque::new();

    let mut map: HashMap<String, Var> = HashMap::new();

    let mut on = 0;

    //This macro should generate match code for unpacking variables of any type
    macro_rules! unpack_var {
        ($vartype:tt, $abs:expr) => {{
            if let Abstract::Var(Var::$vartype(x)) = $abs {
                x.clone()
            } else {
                panic!("Attempted to unpack incorrect variable type")
            }
        }};
    }

    macro_rules! clear_and_progress {
        () => {
            stack.pop_front();
            stack.pop_front();
            stack.pop_front();

            on += 1;
        };
    }

    //This macro should generate code for the general operation case
    macro_rules! operate {
        ($atype:tt, $btype:tt, $restype:tt $operation:expr) => {{
            let result = $operation(
                unpack_var!($atype, stack.get(1).unwrap()),
                unpack_var!($btype, stack.get(0).unwrap()),
            );

            clear_and_progress!();

            stack.push_front(Abstract::Var(Var::$restype(result)));
        }};
    }

    //This macro should generated a type match statements for multiple operation variations.
    //This is one of my most favoritest pieces of code I've written
    macro_rules! multi_operate {
        ( $( ($vartypea:tt, $vartypeb:tt, $outtype:tt $op:expr) ),*) => {{
            match (stack.get(1).unwrap(), stack.front().unwrap()) {

                (Abstract::Var(Var::Void), _) | (_, Abstract::Var(Var::Void)) => {
                    clear_and_progress!();

                    stack.push_front(Abstract::Var(Var::Void));
                }

                $(
                    (Abstract::Var(Var::$vartypea(_)), Abstract::Var(Var::$vartypeb(_))) => {
                        operate!($vartypea, $vartypeb, $outtype$op)
                    }
                )*

                _ => panic!("Incorrect variable types provided at {}: {:?}, {:?}",
                            on,
                            stack.front().unwrap(),
                            stack.get(1).unwrap()
                        )
            }
        }};
    }

    loop {
        //print!("{}", program[on] as char);

        match program[on] {
            //Uncaught whitespace, new line, carriage return, and space respectively.
            10 | 13 | 32 => {
                on += 1;
            }

            //Linear literal
            b'0'..=b'9' => {
                let mut gestalt: Vec<u8> = Vec::new();

                loop {
                    if on >= program.len() {
                        break;
                    }
                    match program[on] {
                        b'0'..=b'9' | b'.' => {
                            gestalt.push(program[on]);
                            on += 1
                        }

                        _ => break,
                    }
                }

                stack.push_front(Abstract::Var(Var::Linear(
                    String::from_utf8(gestalt).unwrap().parse::<f64>().unwrap(),
                )));
            }

            //Gestalt literal
            b'"' => {
                let mut gestalt: Vec<u8> = Vec::new();
                let mut escape = false;
                loop {
                    on += 1;
                    match program[on] {
                        //Matches for quotes (gestalt termination or escaped quote)
                        b'"' => {
                            if !escape {
                                break;
                            } else {
                                escape = false;
                                gestalt.push(b'"')
                            }
                        }

                        //Matches for backslashes (escape or escaped backslash)
                        b'\\' => {
                            if escape {
                                gestalt.push(b'\\');
                            } else {
                                escape = true
                            }
                        }

                        //Matches for other characters and unescapes
                        any => {
                            if escape {
                                escape = false
                            }
                            gestalt.push(any)
                        }
                    }
                }

                on += 1;
                stack.push_front(Abstract::Var(Var::Gestalt(gestalt)));
            }

            //Set literal continuation
            b',' => {
                on += 1;
            }

            //Secondary argument beginning, checks if there is a conditional waiting, and skips code if there is and the latest value in the stack is false (<=0.0).
            //Also checks if there is a "baby" loop, and sets the relevant beginning on it, "maturing" the loop.
            //Also checks for function definitions, adds the given name to the function map and moves past the interior code
            b'{' => {
                match stack.get(1).unwrap() {
                    Abstract::Operator(o) => {
                        if o == &b'~' {
                            //convert latest value to a killid for the matured loop
                            let killid = match stack.pop_front().unwrap() {
                                Abstract::Var(v) => v,
                                _ => panic!("loop was given nonvar kill word"),
                            };

                            //pops off killid and baby loop
                            stack.pop_front();
                            stack.pop_front();

                            //pushes on complete loop with correct beginning, and the killid
                            stack.push_front(Abstract::Loop(on + 1));
                            stack.push_front(Abstract::Var(killid));
                        } else if o == &b'?' {
                            if unpack_var!(Linear, stack.front().unwrap()) > 0.0 {
                                on += 1;
                            } else {
                                on = find_bracket_pair(program, on + 1);
                                stack.pop_front();
                                stack.pop_front(); /*pops conditional and condition*/
                            }
                        }

                        on += 1;
                    }

                    _ => {
                        on += 1;
                    }
                }
            }

            //Set literal end
            b']' => {
                let mut set: Vec<Var> = Vec::new();

                //Breaks if the first element in q is a opening bracket, signaling beginning of set
                while match stack.front() {
                    Some(a) => match a {
                        Abstract::Operator(o) => match o {
                            b'[' => false,
                            _ => true,
                        },
                        _ => true,
                    },
                    _ => true,
                } {
                    //Adds variables to set in reverse order of q, maintaining original order
                    if let Abstract::Var(v) = stack.pop_front().unwrap() {
                        set.insert(0, v);
                    }
                }

                //removes closing bracket operator
                stack.pop_front();

                on += 1;
                stack.push_front(Abstract::Var(Var::Set(set)));
            }

            //Alias Assignment
            b'#' => {
                stack.push_front(Abstract::Operator(b'#'));
                on += 1;

                let mut varname: Vec<u8> = Vec::new();

                while program[on] != b'{' {
                    varname.push(program[on]);
                    on += 1;
                }
                on += 1;

                stack.push_front(Abstract::Var(Var::Gestalt(varname)));
            }

            //Void literal
            b'_' => {
                on += 1;
                stack.push_front(Abstract::Var(Var::Void));
            }

            //Input literal
            b'$' => {
                on += 1;
                stack.push_front(Abstract::Var(input.clone()));
            }

            //Random literal
            b'%' => {
                on += 1;
                stack.push_front(Abstract::Var(Var::Linear(random::<f64>())));
            }

            //Alias (variable referencing)
            b'(' => {
                on += 1;

                let mut varname: Vec<u8> = Vec::new();

                while program[on] != b')' {
                    varname.push(program[on]);
                    on += 1;
                }
                on += 1;

                stack.push_front(Abstract::Var(
                    map.get(&String::from_utf8(varname).unwrap().to_string())
                        .unwrap()
                        .clone(),
                ));
            }

            //Terminator character, immediately matches top of stack to var and returns it, if its not a var then it returns void.
            b';' => {
                return match stack.pop_front() {
                    Some(a) => match a {
                        Abstract::Var(v) => v,
                        _ => Var::Void,
                    },

                    _ => Var::Void,
                }
            }

            //Comments
            b'\\' => {
                on += 1;

                while program[on] != b'\\' {
                    on += 1;
                }

                on += 1;
            }

            //evaluates essentially all operators
            b'}' => {
                match unpack_operator(stack.get(2).unwrap()) {
                    Some(a) => {
                        match a {
                            //CONTROL

                            //Alias assignment
                            b'#' => {
                                map.insert(
                                    String::from_utf8(unpack_var!(Gestalt, stack.get(1).unwrap()))
                                        .unwrap()
                                        .to_string(),
                                    match stack.front().unwrap() {
                                        Abstract::Var(v) => v.clone(),
                                        _ => Var::Void,
                                    },
                                );

                                clear_and_progress!();
                            }

                            //ARTITHMETIC

                            //Addition
                            b'+' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {a + b}),

                                    (Linear, Gestalt, Linear|a: f64, b: Vec<u8>| -> f64 {
                                        a + String::from_utf8(b).unwrap().parse::<f64>().unwrap()
                                    }),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Vec<u8> {
                                        (String::from_utf8(a).unwrap() + &format!("{}", b)).into()
                                    }),

                                    (Gestalt, Gestalt, Gestalt|a: Vec<u8>, b: Vec<u8>| -> Vec<u8> {
                                        (String::from_utf8(a).unwrap() + &String::from_utf8(b).unwrap()).into()
                                    }),

                                    (Set, Linear, Set|a: Vec<Var>, b: f64| -> Vec<Var> {
                                        let mut newset = a.clone();
                                        newset.push(Var::Linear(b));
                                        newset
                                    }),

                                    (Set, Gestalt, Set|a: Vec<Var>, b: Vec<u8>| -> Vec<Var> {
                                        let mut newset = a.clone();
                                        newset.push(Var::Gestalt(b));
                                        newset
                                    })
                                );
                            }
                            //Subtraction
                            b'-' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b:f64| -> f64 {a - b}),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Vec<u8> {
                                        let mut newges = a.clone();
                                        newges.remove(b as i64 as usize);
                                        newges
                                    }),

                                    (Set, Linear, Set|a: Vec<Var>, b: f64| -> Vec<Var> {
                                        let mut newset = a.clone();
                                        newset.remove(b as i64 as usize);
                                        newset
                                    })
                                )
                            }
                            //Multiplication
                            b'*' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {a * b})
                                )
                            }
                            //Division
                            b'/' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {a / b})
                                )
                            }
                            //Exponentiation
                            b'^' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {a.powf(b)})
                                )
                            }

                            //LOGICAL

                            //And
                            b'&' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {
                                        if a > 0.0 && b > 0.0 {1.0} else {0.0}
                                    })


                                )
                            }
                            //Or
                            b'|' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {
                                        if a > 0.0 || b > 0.0 {1.0} else {0.0}
                                    })
                                )
                            }

                            //COMPARISON

                            //Equal to
                            b'=' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {
                                        if a == b {1.0} else {0.0}
                                    }),

                                    (Gestalt, Gestalt, Linear|a: Vec<u8>, b: Vec<u8>| -> f64 {
                                        if a == b {1.0} else {0.0}
                                    })
                                )
                            }
                            //Greater than
                            b'>' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {
                                        if a > b {1.0} else {0.0}
                                    }),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Vec<u8> {
                                        let mut newges = a.clone();
                                        newges.truncate(a.len() - b as i64 as usize);
                                        newges
                                    }),

                                    (Set, Linear, Set|a: Vec<Var>, b: f64| -> Vec<Var> {
                                        let mut newset = a.clone();
                                        newset.truncate(a.len() - b as i64 as usize);
                                        newset
                                    })
                                )
                            }
                            //Less than
                            b'<' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> f64 {
                                        if a < b {1.0} else {0.0}
                                    }),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Vec<u8> {
                                        a[b as i64 as usize..].to_vec()
                                    }),

                                    (Set, Linear, Set|a: Vec<Var>, b: f64| -> Vec<Var> {
                                        a[b as i64 as usize..].to_vec()
                                    })
                                )
                            }

                            //MISC

                            //Evaluation
                            b'!' => {
                                let eva = evaluate(
                                    &unpack_var!(Gestalt, stack.get(1).unwrap()),
                                    match stack.front().unwrap() {
                                        Abstract::Var(v) => &v,
                                        _ => &Var::Void,
                                    },
                                );

                                clear_and_progress!();

                                stack.push_front(Abstract::Var(eva));
                            }
                            //Reading/writing files
                            b'@' => {}
                            //Set access, macro can't cover these subtypeless sets so its got its own special thingy
                            b'`' => {

                                match (stack.get(1).unwrap(), stack.front().unwrap()) {

                                    (Abstract::Var(Var::Set(s)), Abstract::Var(Var::Linear(l))) => {

                                        let element = Abstract::Var(s.get(*l as i64 as usize).unwrap().clone());

                                        clear_and_progress!();

                                        stack.push_front(element);
                                    }

                                    (Abstract::Var(Var::Gestalt(g)), Abstract::Var(Var::Linear(l))) => {

                                        let char = g[*l as i64 as usize];

                                        clear_and_progress!();

                                        stack.push_front(Abstract::Var(Var::Gestalt(vec!(char))));
                                    }

                                    _ => panic!("Incorrect variable types provided at {}: {:?}, {:?}",
                                                on,
                                                stack.front().unwrap(),
                                                stack.get(1).unwrap()
                                            )
                                }
                            }
                            //terminal access
                            b'\'' => {}
                            //Conditional
                            b'?' => {
                                clear_and_progress!();
                            }

                            //Invalid operator
                            _ => {
                                panic!("Invalid operator \"{}\"", a as char);
                            }
                        }
                    }

                    //In this case, its not an operator, so it must be a loop
                    _ => {
                        if let Abstract::Loop(start) = stack.get(2).unwrap() {
                            let start = *start;

                            //If the evaluation of the secondary argument is gestalt equal to the first,
                            //Then the loop is terminated.
                            if unpack_var!(Gestalt, stack.front().unwrap())
                                == unpack_var!(Gestalt, stack.get(1).unwrap())
                            {
                                //removes the whole of the loop code and moves forward
                                clear_and_progress!();
                            } else {
                                //removes the secondary argument and starts over at the loop's associated on value
                                stack.pop_front();

                                on = start;
                            }
                        }
                    }
                }
            }

            //Anything else (valid) should be a normal operator, so they just get appended.
            //Loops are included in here because they are initially appended as uninitialized.
            //Alias beginning is included in here as well.
            _ => {
                stack.push_front(Abstract::Operator(program[on]));
                on += 1;
            }
        }
    }
}

fn lintobool(linear: f64) -> bool {
    if linear > 0.0 {
        true
    } else {
        false
    }
}

fn booltolin(boolean: bool) -> f64 {
    if boolean {
        1.0
    } else {
        0.0
    }
}

fn unpack_operator(packed: &Abstract) -> Option<u8> {
    match packed {
        Abstract::Operator(o) => Some(*o),
        _ => None,
    }
}

//Helper function, used to find the end of secondary args. Expects to start the character directly after the first bracket.
//Returns the position directly after the pairing bracket.
fn find_bracket_pair(program: &Vec<u8>, mut on: usize) -> usize {
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
