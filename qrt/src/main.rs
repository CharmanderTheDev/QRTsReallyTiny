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
    Control,      //Continue evaluation after actions
    Operator(u8), //Generic operators, also include "loops" that haven't been initialized yet.
    Conditional,  //Special conditional operator, could honestly be replaced with Operator(b'?').
    Loop(usize),  //Loops contains start of looping code, on "on"
}

fn evaluate(program: &Vec<u8>, input: &Var) -> Var {
    let mut stack: VecDeque<Abstract> = VecDeque::new();

    let mut map: HashMap<String, Var> = HashMap::new();

    let mut on = 0;

    loop {
        match program[on] {
            //Uncaught whitespace
            10 | 32 => {
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

            //Set literal begin
            b'[' => {
                on += 1;
                stack.push_front(Abstract::Operator(b'['));
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
                        }

                        else if o == &b'?' {
                            match unpack_linear(stack.front().unwrap()) {
                                Some(b) => {
                                    if b > 0.0 {
                                        on += 1;
                                    } else {
                                        on = find_bracket_pair(program, on + 1);
                                        stack.pop_front();
                                        stack.pop_front(); /*pops conditional and condition*/
                                    }
                                }
                                None => {
                                    panic!("conditional was given nonlinear argument")
                                }
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
                    None => true,
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

                    None => Var::Void,
                }
            }

            //Comments
            b'\\' => {}

            //evaluates a ton of "normal" operators (artithmetic, boolean, comparison, etc.)
            //Should also handle loop recursion, and termination if secondary variable has
            //evaluated to the relevant kill. Should contain recursive function evaluation
            //capabilities. Should handle environmental terminal calls ("unlimited functionality, apostrophe operator")
            //
            b'}' => {
                //Current problem: when reaching the closing bracket of a successful conditional,
                //Is unable to find stack.get(2), due to the internal code of the conditional
                //Not actually returning a value and instead being a control function.
                //Possible solution, implement the control type or return a null.

                match unpack_operator(stack.get(2).unwrap()) {
                    Some(a) => {
                        match a {
                            //CONTROL

                            //Alias assignment
                            b'#' => {
                                map.insert(
                                    String::from_utf8(
                                        unpack_gestalt(stack.get(1).unwrap()).unwrap(),
                                    )
                                    .unwrap()
                                    .to_string(),
                                    match stack.front().unwrap() {
                                        Abstract::Var(v) => v.clone(),
                                        _ => Var::Void,
                                    },
                                );

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();

                                on += 1;
                            }

                            //ARTITHMETIC

                            //Addition
                            b'+' => {
                                let sum = unpack_linear(stack.front().unwrap()).unwrap()
                                    + unpack_linear(stack.get(1).unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(sum)));
                            }
                            //Subtraction
                            b'-' => {
                                let difference = unpack_linear(stack.get(1).unwrap()).unwrap()
                                    - unpack_linear(stack.front().unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(difference)));
                            }

                            //Multiplication
                            b'*' => {
                                let product = unpack_linear(stack.front().unwrap()).unwrap()
                                    * unpack_linear(stack.get(1).unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(product)));
                            }

                            //Division
                            b'/' => {
                                let quotient = unpack_linear(stack.get(1).unwrap()).unwrap()
                                    / unpack_linear(stack.front().unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(quotient)));
                            }

                            //Exponentiation
                            b'^' => {
                                let power = unpack_linear(stack.get(1).unwrap())
                                    .unwrap()
                                    .powf(unpack_linear(stack.front().unwrap()).unwrap());

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(power)));
                            }

                            //LOGICAL

                            //And
                            b'&' => {
                                let truth = unpack_bool(stack.front().unwrap()).unwrap()
                                    && unpack_bool(stack.get(1).unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(if truth {
                                    1.0
                                } else {
                                    0.0
                                })))
                            }

                            //Or
                            b'|' => {
                                let truth = unpack_bool(stack.front().unwrap()).unwrap()
                                    || unpack_bool(stack.get(1).unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(if truth {
                                    1.0
                                } else {
                                    0.0
                                })))
                            }

                            //COMPARISON

                            //Greater than
                            b'>' => {
                                let truth = unpack_linear(stack.get(1).unwrap()).unwrap()
                                    > unpack_linear(stack.front().unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(if truth {
                                    1.0
                                } else {
                                    0.0
                                })))
                            }

                            //Equal to
                            b'=' => {
                                let truth = unpack_linear(stack.get(1).unwrap()).unwrap()
                                    == unpack_linear(stack.front().unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(if truth {
                                    1.0
                                } else {
                                    0.0
                                })))
                            }

                            //Less than
                            b'<' => {
                                let truth = unpack_linear(stack.get(1).unwrap()).unwrap()
                                    < unpack_linear(stack.front().unwrap()).unwrap();

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();
                                on += 1;

                                stack.push_front(Abstract::Var(Var::Linear(if truth {
                                    1.0
                                } else {
                                    0.0
                                })))
                            }

                            //MISC

                            //Evaluation
                            b'!' => {
                                let eva = evaluate(
                                    &unpack_gestalt(stack.get(1).unwrap()).unwrap(),
                                    match stack.front().unwrap() {
                                        Abstract::Var(v) => v,
                                        _ => &Var::Void,
                                    },
                                );

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();

                                stack.push_front(Abstract::Var(eva));

                                on += 1;
                            }

                            //Reading/writing files
                            b'@' => {}

                            //Set access
                            b'`' => {}

                            //terminal access
                            b'\'' => {}

                            //Function definition
                            b':' => {}

                            //Conditional
                            b'?' => { 

                                stack.pop_front();
                                stack.pop_front();
                                stack.pop_front();

                                on += 1;
                            }

                            //Invalid operator
                            _ => {
                                panic!("Invalid operator");
                            }
                        }
                    }

                    //In this case, its not an operator, so it must be a loop
                    None => {
                        match stack.get(2).unwrap() {

                            Abstract::Loop(start) => {
                                let start = *start;
                                
                                //If the evaluation of the secondary argument is gestalt equal to the first,
                                //Then the loop is terminated.
                                if unpack_gestalt(stack.front().unwrap()).unwrap()
                                    == unpack_gestalt(stack.get(1).unwrap()).unwrap()
                                {
                                    //removes the whole of the loop code and moves forward
                                    stack.pop_front();
                                    stack.pop_front();
                                    stack.pop_front();

                                    on += 1;
                                } else {
                                    //removes the secondary argument and starts over at the loop's associated on value
                                    stack.pop_front();

                                    on = start;
                                }
                            }

                            _ => {}
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

//Removes comments and whitespace
fn compile(program: &Vec<u8>) {}

fn operate<Atype, Btype, Outtype>(
    arg1: Atype,
    arg2: Btype,
    func: fn(Atype, Btype) -> Outtype,
    stack: VecDeque<Abstract>,
) {
}

fn unpack_linear(packed: &Abstract) -> Option<f64> {
    match packed {
        Abstract::Var(v) => match v {
            Var::Linear(b) => Some(*b),

            _ => None,
        },

        _ => None,
    }
}

fn unpack_gestalt(packed: &Abstract) -> Option<Vec<u8>> {
    match packed {
        Abstract::Var(v) => match v {
            Var::Gestalt(b) => Some(b.clone()),

            _ => None,
        },
        _ => None,
    }
}

fn unpack_operator(packed: &Abstract) -> Option<u8> {
    match packed {
        Abstract::Operator(o) => Some(*o),
        _ => None,
    }
}

fn unpack_bool(packed: &Abstract) -> Option<bool> {
    match packed {
        Abstract::Var(v) => Some(v.bool()),

        _ => None,
    }
}

fn unpack_set(packed: &Abstract) -> Option<&Vec<Var>> {
    match packed {
        Abstract::Var(v) => match v {
            Var::Set(s) => Some(s),

            _ => None,
        },
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
