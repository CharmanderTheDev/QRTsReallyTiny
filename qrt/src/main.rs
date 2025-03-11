extern crate rand;

use rand::random;
use std::{
    collections::{HashMap, VecDeque},
    env, fs,
    io::Read,
    path::Path,
    vec::Vec,
};

fn main() {
    let args: Vec<String> = env::args().collect();

    let file: Vec<u8> = fs::read_to_string(&args[1])
        .expect("No such file found")
        .trim()
        .as_bytes()
        .into();

    if let Ok(v) = evaluate(&String::into_bytes(args[2].clone()), &Var::Void) {
        let input = v;
    } else {
        panic!()
    }

    let evaluation = evaluate(&file, &input);

    //println!("{:?}", evaluation);
}

#[derive(Clone, Debug, PartialEq)]
enum Var {
    Void,             //Null type
    Linear(f64),      //Numbers
    Gestalt(Vec<u8>), //Strings
    Set(Vec<Var>),    //Lists
}
impl Var {
    //Custom representation schema for vars for debugging purposes
    fn represent(&self) -> String {
        match self {
            Var::Void => "Void".to_string(),

            Var::Linear(l) => f64::to_string(l),

            Var::Gestalt(g) => "\"".to_string() + &String::from_utf8(g.to_vec()).unwrap() + "\"",

            Var::Set(set) => {
                let mut string: String = "[".to_string();

                for var in set {
                    string.push_str(&var.represent());
                    string.push_str(", ");
                }

                string.push_str("]");

                string
            }
        }
    }
}

#[derive(Clone, Debug)]
enum Abstract {
    Var(Var),     //Values
    Operator(u8), //Generic operators, also include "loops" that haven't been initialized yet.
    Loop(usize), //Loops are a special operator that require metadata pointing to their start location
}
impl Abstract {
    //Custom representation schema for abstract for debugging purposes
    fn represent(&self) -> String {
        match self {
            Abstract::Var(v) => v.represent(),

            Abstract::Operator(o) => {
                let mut buf = vec![0; 1];

                "Operator(".to_string() + (*o as char).encode_utf8(&mut buf) + ")"
            }

            Abstract::Loop(u) => "Loop(".to_string() + u.to_string().as_str() + ")",
        }
    }
}

fn evaluate(
    program: &[u8],
    input: &Var,
) -> Result<Var, (String, usize, VecDeque<Abstract>, HashMap<String, Var>)> {
    let mut stack: VecDeque<Abstract> = VecDeque::new();

    let mut map: HashMap<String, Var> = HashMap::new();

    let mut on = 0;

    //This macro should generate match code for unpacking variables of any type
    macro_rules! unpack_var {
        ($vartype:tt, $index:expr, $typmsg:expr) => {{
            if let Abstract::Var(Var::$vartype(x)) = unpack_stack!($index) {
                x.clone()
            } else {
                return return_error!($typmsg)
            }
        }};
    }

    macro_rules! string_from_utf8 {
        ($utf8:expr) => {{
            if let Ok(s) = String::from_utf8($utf8) {
                s
            } else {
                return return_error!("Invalid Gestalt chars")
            }
        }};
    }

    //This is for use inside operation closures where the return type is a simpler Result
    macro_rules! cstring_from_utf8 {
        ($utf8:expr) => {{
            if let Ok(s) = String::from_utf8($utf8) {
                s
            } else {
                return Err("Invalid Gestalt chars")
            }
        }}
    }

    //This is a common piece of code for operations on the stack
    macro_rules! clear_and_progress {
        () => {
            stack.pop_front();
            stack.pop_front();
            stack.pop_front();

            on += 1;
        };
    }

    //This macro generates a type match statements for multiple operation variations.
    macro_rules! multi_operate {
        ( $( ($vartypea:tt, $vartypeb:tt, $outtype:tt $operation:expr) ),*) => {{
            match (stack.get(1).unwrap(), stack.front().unwrap()) {

                (Abstract::Var(Var::Void), _) | (_, Abstract::Var(Var::Void)) => {
                    clear_and_progress!();

                    stack.push_front(Abstract::Var(Var::Void));
                }

                $(
                    (Abstract::Var(Var::$vartypea(a)), Abstract::Var(Var::$vartypeb(b))) => {
                        let result = $operation(
                            a.clone(),
                            b.clone()
                        );

                        clear_and_progress!();
                        
                        match $operation(a.clone(), b.clone()) {
                            Ok(result) => {stack.push_front(Abstract::Var(Var::$outtype(result)));}
                            Err(error) => {return_error!(error)}
                        }
                    }
                )*

                _ => {return_error!("Incorrect types for operation")}
            }
        }};
    }

    macro_rules! return_error {
        ($errtext:expr) => {
            Result::Err(($errtext.to_string(), on, stack, map))
        };
    }

    macro_rules! unpack_stack {
        ($index:expr) => {
            if let Some(a) = stack.get($index) {
                a
            } else {
                return return_error!("Error in unpack_stack!")
            }
        };
    }

    macro_rules! unpack_map {
        ($id:expr) => {
            if let Some(v) = map.get($id) {
                v
            } else {
                return return_error!("Variable not found")
            }
        };
    }

    loop {
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

                if let Ok(number) = string_from_utf8!(gestalt).parse::<f64>() {
                    stack.push_front(Abstract::Var(Var::Linear(number)));
                } else {
                    return return_error!("incorrect linear formatting")
                }
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
                            let killid = if let Some(Abstract::Var(v)) = stack.pop_front() {
                                v
                            } else {
                                return return_error!("pop_front error in finding loop killid")
                            };

                            //pops off killid and baby loop
                            stack.pop_front();
                            stack.pop_front();

                            //pushes on complete loop with correct beginning, and the killid
                            stack.push_front(Abstract::Loop(on + 1));
                            stack.push_front(Abstract::Var(killid));
                        } else if o == &b'?' {
                            if unpack_var!(
                                Linear,
                                0,
                                "Incorrect conditional type given to ? operator"
                            ) > 0.0
                            {
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
                while !matches!(stack.front(), Some(Abstract::Operator(b'['))) {
                    //Adds variables to set in reverse order of q, maintaining original order
                    if let Abstract::Var(v) = unpack_stack!(0) {
                        set.insert(0, v.clone());
                    }
                }

                //removes closing bracket operator
                stack.pop_front();

                on += 1;
                stack.push_front(Abstract::Var(Var::Set(set)));
            }

            //Alias and function assignment
            b'#' => {
                let function = program[on + 1] == b'!';

                on += if function { 2 } else { 1 };

                let mut name: Vec<u8> = Vec::new();

                while program[on] != b'{' {
                    name.push(program[on]);
                    on += 1;
                }

                on += 1;

                if function {
                    //Special function case, save the current "on" as a linear in the map with the given name
                    map.insert(string_from_utf8!(name), Var::Linear(on as f64));

                    //Now find the end of the function definition and set the on past there
                    on = find_bracket_pair(program, on);
                } else {
                    //General variable case, wait for eval and save the name and operator to stack
                    stack.push_front(Abstract::Operator(b'#'));
                    stack.push_front(Abstract::Var(Var::Gestalt(name)));
                }
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
                    unpack_map!(&string_from_utf8!(varname)).clone(),
                ));
            }

            //Terminator character, immediately matches top of stack to var and returns it, if its not a var then it returns void.
            b':' => {
                return match stack.pop_front() {
                    Some(Abstract::Var(v)) => Ok(v),

                    _ => Ok(Var::Void),
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
                match unpack_operator(unpack_stack!(2)) {
                    Some(a) => {
                        match a {
                            //CONTROL

                            //Alias assignment
                            b'#' => {
                                map.insert(
                                    string_from_utf8!(unpack_var!(
                                        Gestalt,
                                        1,
                                        "Invalid variable name"
                                    ))
                                    .to_string(),
                                    match unpack_stack!(0) {
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
                                    (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {Ok(a + b)}),

                                    (Linear, Gestalt, Linear|a: f64, b: Vec<u8>| -> Result<f64, &str> {
                                        if let Ok(b) = cstring_from_utf8!(b).parse::<f64>() {
                                            return Ok(a + b)
                                        } else {
                                            return Err("Could not coerce Gestalt to Linear")
                                        }
                                    }),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Result<Vec<u8>, &str> {
                                        Ok((cstring_from_utf8!(a) + &format!("{}", b)).into())
                                    }),

                                    (Gestalt, Gestalt, Gestalt|a: Vec<u8>, b: Vec<u8>| -> Result<Vec<u8>, &str> {
                                        Ok((cstring_from_utf8!(a) + &cstring_from_utf8!(b)).into())
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
                            b'!' => match (stack.front().unwrap(), stack.get(1).unwrap()) {
                                (Abstract::Var(v), Abstract::Var(Var::Linear(jmp))) => {
                                    let eva = evaluate(&program[*jmp as i64 as usize..], v);

                                    clear_and_progress!();

                                    stack.push_front(Abstract::Var(eva));
                                }

                                (Abstract::Var(v), Abstract::Var(Var::Gestalt(g))) => {
                                    let eva = evaluate(g, v);

                                    clear_and_progress!();

                                    stack.push_front(Abstract::Var(eva));
                                }

                                _ => panic!(),
                            },
                            //Reading/writing files
                            b'@' => match (stack.get(1).unwrap(), stack.front().unwrap()) {
                                (Abstract::Var(Var::Gestalt(g)), Abstract::Var(Var::Void)) => {
                                    let file: Vec<u8> =
                                        fs::read_to_string(String::from_utf8(g.to_vec()).unwrap())
                                            .expect("No such file found")
                                            .as_bytes()
                                            .into();

                                    clear_and_progress!();

                                    stack.push_front(Abstract::Var(Var::Gestalt(file)));
                                }

                                (
                                    Abstract::Var(Var::Gestalt(ga)),
                                    Abstract::Var(Var::Gestalt(gb)),
                                ) => {
                                    //If the file does not exist at the specified path, create one, and open it up either way.
                                    //Read the contents and store them, then write the new contents to the file.
                                    //If the file didnt' exist before, return a Void, if not, return the old contents.

                                    let path = &String::from_utf8(ga.to_vec()).unwrap();
                                    let exists = Path::new(path).exists();

                                    let mut file;

                                    if !exists {
                                        file = fs::File::create(path).unwrap();
                                    }

                                    file = fs::File::open(path).unwrap();

                                    let mut contents = String::new();

                                    file.read_to_string(&mut contents).unwrap();

                                    fs::write(
                                        String::from_utf8(ga.to_vec()).unwrap(),
                                        String::from_utf8(gb.to_vec()).unwrap(),
                                    )
                                    .expect(
                                        &("Unable to write file at ".to_string() + &on.to_string()),
                                    );

                                    clear_and_progress!();

                                    stack.push_front(Abstract::Var(if exists {
                                        Var::Gestalt(contents.into())
                                    } else {
                                        Var::Void
                                    }));
                                }

                                _ => panic!(),
                            },
                            //Set access, macro can't cover these subtypeless sets so its got its own special thingy
                            b'`' => match (stack.get(1).unwrap(), stack.front().unwrap()) {
                                (Abstract::Var(Var::Set(s)), Abstract::Var(Var::Linear(l))) => {
                                    let element =
                                        Abstract::Var(s.get(*l as i64 as usize).unwrap().clone());

                                    clear_and_progress!();

                                    stack.push_front(element);
                                }

                                (Abstract::Var(Var::Gestalt(g)), Abstract::Var(Var::Linear(l))) => {
                                    let char = g[*l as i64 as usize];

                                    clear_and_progress!();

                                    stack.push_front(Abstract::Var(Var::Gestalt(vec![char])));
                                }

                                _ => panic!(),
                            },
                            //Conditional
                            b'?' => {
                                clear_and_progress!();
                            }

                            //Invalid operator
                            _ => {
                                panic!();
                            }
                        }
                    }

                    //In this case, its not an operator, so it must be a loop
                    _ => {
                        if let Abstract::Loop(start) = stack.get(2).unwrap() {
                            let start = *start;
                            let mut recurse = true;

                            //Recurse only falsifies if both outputs are equal, and both evaluate to Vars.
                            //This ensures loops can have varying return types.
                            if let (Abstract::Var(va), Abstract::Var(vb)) =
                                (stack.front().unwrap(), stack.get(1).unwrap())
                            {
                                recurse = va != vb;
                            }
                            if recurse {
                                //pops off secondary argument and starts over at loop's associated on value
                                stack.pop_front();

                                on = start;
                            } else {
                                //removes the whole of the loop code and moves forward
                                clear_and_progress!();
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

fn unpack_operator(packed: &Abstract) -> Option<u8> {
    match packed {
        Abstract::Operator(o) => Some(*o),
        _ => None,
    }
}

//Helper function, used to find the end of secondary args. Expects to start the character directly after the first bracket.
//Returns the position directly after the pairing bracket.
fn find_bracket_pair(program: &[u8], mut on: usize) -> usize {
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
