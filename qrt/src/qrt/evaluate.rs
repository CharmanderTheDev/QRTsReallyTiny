use super::{structs::*, helpers::*};

use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::Read,
    path::Path,
    vec::Vec,
};

extern crate rand;
use rand::random;


//This is the big one, our 750-line function that evaluates all QRT code with a little help.
pub fn evaluate(program: &[u8], input: &Var) -> Evaluation {

    //This is used to store the state of our program
    let mut stack: VecDeque<Abstract> = VecDeque::new();

    //This is used to store declared variables within the program
    let mut map: HashMap<String, Var> = HashMap::new();

    //This is used to store our place in evaluation
    let mut on = 0;

    //This macro coerces a Var to the desired type, throwing an error if it fails.
    macro_rules! unpack_var {
        ($vartype:tt, $index:expr, $typmsg:expr) => {{
            if let Abstract::Var(Var::$vartype(x)) = unpack_stack!($index) {
                x.clone()
            } else {
                return_error!($typmsg);
            }
        }};
    }

    //This macro converts a Vec<u8> (gestalt inner type) to a string, throwing the relevant error without unwrap.
    macro_rules! string_from_utf8 {
        ($utf8:expr) => {{
            if let Ok(s) = String::from_utf8($utf8) {
                s
            } else {
                return_error!("Invalid Gestalt chars");
            }
        }};
    }

    //This is for use inside operation closures where the return type is a simpler Result with only a msg.
    macro_rules! cstring_from_utf8 {
        ($utf8:expr) => {{
            if let Ok(s) = String::from_utf8($utf8) {
                s
            } else {
                return Err("Invalid Gestalt chars");
            }
        }};
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

    //This macro generates mutliple type match statements for multiple operation variations (Linear-Linear, Gestalt-Linear, etc.)
    macro_rules! multi_operate {
        ( $( ($vartypea:tt, $vartypeb:tt, $outtype:tt $operation:expr) ),*) => {{
            match (unpack_stack!(1), unpack_stack!(0)) {

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

                        match result {
                            Ok(result) => {stack.push_front(Abstract::Var(Var::$outtype(result)));}
                            Err(error) => {return_error!(error)}
                        }
                    }
                )*

                _ => {return_error!("Invalid operand types")}
            }
        }};
    }

    //This macro finds the current line, and returns the given error message along with the slew of sometimes-needed debug info
    macro_rules! return_error {
        ($errtext:expr) => {{
            let (mut i, mut linecount) = (on, 0);

            while i != 0 {
                //Checks for newlines
                if program[i] == 10 {
                    linecount += 1;
                }

                i -= 1;
            }

            return Result::Err(($errtext.to_string(), on, linecount, stack, map));
        }};
    }

    //This macro takes an item off the stack and essentially unwraps it with our custom error sytem
    macro_rules! unpack_stack {
        ($index:expr) => {
            if let Some(a) = stack.get($index) {
                a
            } else {
                return_error!(
                    "Error getting index ".to_string() + &format!("{}", $index) + "from stack"
                );
            }
        };
    }

    //This macro attempts to find an item on the map and automatically unwraps it with our custom error system
    macro_rules! unpack_map {
        ($id:expr) => {
            if let Some(v) = map.get($id) {
                v
            } else {
                return_error!("Variable not found");
            }
        };
    }

    //This is the main evaluation loop
    loop {
        //print!("{}", program[on] as char); //Silly debug tool

        match program[on] {

            //Space, tab, carriage return, and new line. Essentially whitespace skipping.
            19 | 32 | 13 | 10 => {
                on += 1;
            }

            //Set literal continuation, yes its redundant but its nicer.
            b',' => {
                on += 1;
            }

            //Comments
            b'\\' => {
                on += 1;

                while program[on] != b'\\' {
                    on += 1;
                }

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
                    return_error!("Incorrect linear formatting");
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

            //Set literal end (Beginning bracket should have already been pushed by last match)
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

            //Void literal
            b'_' => {
                on += 1;
                stack.push_front(Abstract::Var(Var::Void));
            }

            //Input reference
            b'$' => {
                on += 1;
                stack.push_front(Abstract::Var(input.clone()));
            }

            //Random reference
            b'%' => {
                on += 1;
                stack.push_front(Abstract::Var(Var::Linear(random::<f64>())));
            }

            //Secondary argument beginning, checks if there is a conditional waiting, and skips code if there is and the latest value in the stack is false (<=0.0).
            //Also checks if there is a "baby" loop, and sets the relevant beginning on it, "maturing" the loop.
            //Also checks for function definitions, adds the given name to the function map and moves past the interior code
            b'{' => {
                match if let Some(a) = stack.get(1) {
                    a
                } else {
                    return_error!("Error finding operator for opening bracket");
                } {
                    Abstract::Operator(o) => {
                        if o == &b'~' {
                            //convert latest value to a killid for the matured loop
                            let killid = if let Some(Abstract::Var(v)) = stack.pop_front() {
                                v
                            } else {
                                return_error!("Pop_front error in finding loop killid");
                            };

                            //pops off killid and baby loop
                            stack.pop_front();
                            stack.pop_front();

                            //pushes on complete loop with correct beginning, and the killid
                            stack.push_front(Abstract::Loop(on + 1));
                            stack.push_front(Abstract::Var(killid));
                        } else if o == &b'?' {
                            if unpack_var!(Linear, 0, "Invalid operand types") > 0.0 {
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

            //Alias and function assignment
            b'#' => {
                //If a bang follows the hashtag, its a function
                let function = program[on + 1] == b'!';

                on += if function { 2 } else { 1 };

                let mut name: Vec<u8> = Vec::new();

                while !(program[on] == b'{' || program[on] == b'!') {
                    name.push(program[on]);
                    on += 1;
                }

                if program[on] == b'!' {
                    return_error!("Bangs (!) not allowed in variable names")
                }

                on += 1;

                if function {
                    
                    //Special function case, save the current "on" as a linear in the map with the given name, and give it a fancy name for debugging
                    map.insert(
                        (string_from_utf8!(name) + if function { "!" } else { "" }),
                        Var::Linear(on as f64),
                    );

                    //Now find the end of the function definition and set the "on" past there
                    on = find_bracket_pair(program, on);

                } else {

                    //General variable case, wait for eval and save the name and operator to stack
                    stack.push_front(Abstract::Operator(b'#'));
                    stack.push_front(Abstract::Var(Var::Gestalt(name)));
                }
            }

            //Alias referencing
            b'(' => {
                on += 1;

                let mut varname: Vec<u8> = Vec::new();

                while program[on] != b')' {
                    varname.push(program[on]);
                    on += 1;
                }
                on += 1;

                //Checks if either varname or varname! exists, since functions add bangs in definition
                if map.contains_key(&string_from_utf8!(varname.clone())) {
                    stack.push_front(Abstract::Var(
                        unpack_map!(&string_from_utf8!(varname)).clone(),
                    ));
                } else {
                    varname.push(b'!');
                    stack.push_front(Abstract::Var(
                        unpack_map!(&string_from_utf8!(varname)).clone(),
                    ));
                }
            }

            //Closing bracket, evaluates all operators
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
                                            Ok(a + b)
                                        } else {
                                            Err("Could not coerce Gestalt to Linear")
                                        }
                                    }),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Result<Vec<u8>, &str> {
                                        Ok((cstring_from_utf8!(a) + &format!("{}", b)).into())
                                    }),

                                    (Gestalt, Gestalt, Gestalt|a: Vec<u8>, b: Vec<u8>| -> Result<Vec<u8>, &str> {
                                        Ok((cstring_from_utf8!(a) + &cstring_from_utf8!(b)).into())
                                    }),

                                    (Set, Linear, Set|a: Vec<Var>, b: f64| -> Result<Vec<Var>, &str> {
                                        let mut newset = a.clone();
                                        newset.push(Var::Linear(b));
                                        Ok(newset)
                                    }),

                                    (Set, Gestalt, Set|a: Vec<Var>, b: Vec<u8>| -> Result<Vec<Var>, &str> {
                                        let mut newset = a.clone();
                                        newset.push(Var::Gestalt(b));
                                        Ok(newset)
                                    })
                                );
                            }
                            //Subtraction
                            b'-' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b:f64| -> Result<f64, &str> {Ok(a - b)}),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Result<Vec<u8>, &str> {
                                        let mut newges = a.clone();
                                        newges.remove(b as i64 as usize);
                                        Ok(newges)
                                    }),

                                    (Set, Linear, Set|a: Vec<Var>, b: f64| -> Result<Vec<Var>, &str> {
                                        let mut newset = a.clone();
                                        newset.remove(b as i64 as usize);
                                        Ok(newset)
                                    })
                                )
                            }
                            //Multiplication
                            b'*' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {Ok(a * b)})
                                )
                            }
                            //Division
                            b'/' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {Ok(a / b)})
                                )
                            }
                            //Exponentiation
                            b'^' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {Ok(a.powf(b))})
                                )
                            }

                            //LOGICAL

                            //And
                            b'&' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {
                                        if a > 0.0 && b > 0.0 {Ok(1.0)} else {Ok(0.0)}
                                    })
                                )
                            }
                            //Or
                            b'|' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {
                                        if a > 0.0 || b > 0.0 {Ok(1.0)} else {Ok(0.0)}
                                    })
                                )
                            }

                            //COMPARISON

                            //Equal to
                            b'=' => {
                                //Special case for two void variables, will return essentially a true
                                if let (Abstract::Var(Var::Void), Abstract::Var(Var::Void)) =
                                    (unpack_stack!(0), unpack_stack!(1))
                                {
                                    clear_and_progress!();

                                    stack.push_front(Abstract::Var(Var::Linear(1.0)))
                                } else {
                                    multi_operate!(
                                        (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {
                                            if a == b {Ok(1.0)} else {Ok(0.0)}
                                        }),

                                        (Gestalt, Gestalt, Linear|a: Vec<u8>, b: Vec<u8>| -> Result<f64, &str> {
                                            if a == b {Ok(1.0)} else {Ok(0.0)}
                                        }),

                                        (Set, Set, Linear|a: Vec<Var>, b: Vec<Var>| -> Result<f64, &str> {
                                            if a == b {Ok(1.0)} else {Ok(0.0)}
                                        })
                                    )
                                }
                            }
                            //Greater than
                            b'>' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {
                                        if a > b {Ok(1.0)} else {Ok(0.0)}
                                    }),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Result<Vec<u8>, &str> {
                                        let mut newges = a.clone();
                                        newges.truncate(a.len() - b as i64 as usize);
                                        Ok(newges)
                                    }),

                                    (Set, Linear, Set|a: Vec<Var>, b: f64| -> Result<Vec<Var>, &str> {
                                        let mut newset = a.clone();
                                        newset.truncate(a.len() - b as i64 as usize);
                                        Ok(newset)
                                    })
                                )
                            }
                            //Less than
                            b'<' => {
                                multi_operate!(
                                    (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {
                                        if a < b {Ok(1.0)} else {Ok(0.0)}
                                    }),

                                    (Gestalt, Linear, Gestalt|a: Vec<u8>, b: f64| -> Result<Vec<u8>, &str> {
                                        Ok(a[b as i64 as usize..].to_vec())
                                    }),

                                    (Set, Linear, Set|a: Vec<Var>, b: f64| -> Result<Vec<Var>, &str> {
                                        Ok(a[b as i64 as usize..].to_vec())
                                    })
                                )
                            }

                            //MISC

                            //Evaluation
                            b'!' => match (unpack_stack!(0), unpack_stack!(1)) {
                                (Abstract::Var(v), Abstract::Var(Var::Linear(jmp))) => {
                                    //If the evaluation itself throws an error, that error and its interior stack/map are
                                    //Given as the error, along with a notification of what function threw the error.
                                    match evaluate(&program[jmp.clone() as i64 as usize..], &v) {
                                        Ok(eva) => {
                                            clear_and_progress!();
                                            stack.push_front(Abstract::Var(eva))
                                        }
                                        Err((msg, funcon, funclineon, stack, map)) => {
                                            return Result::Err((
                                                msg + "(In function evaluated at "
                                                    + &format!("{}", on)
                                                    + ")",
                                                funcon,
                                                funclineon,
                                                stack,
                                                map,
                                            ))
                                        }
                                    }
                                }

                                (Abstract::Var(v), Abstract::Var(Var::Gestalt(g))) => {
                                    let eva = match evaluate(&g, &v) {
                                        Ok(eva) => eva,
                                        Err((msg, funcon, funclineon, stack, map)) => {
                                            return Result::Err((
                                                msg + "(In function evaluated at "
                                                    + &format!("{}", on)
                                                    + ")",
                                                funcon,
                                                funclineon,
                                                stack,
                                                map,
                                            ))
                                        }
                                    };

                                    clear_and_progress!();

                                    stack.push_front(Abstract::Var(eva));
                                }

                                _ => return_error!("Invalid operand types"),
                            },
                            //Reading/writing files
                            b'@' => match (unpack_stack!(1), unpack_stack!(0)) {

                                //For a gestalt and a void, we're just reading, no writing.
                                (Abstract::Var(Var::Gestalt(g)), Abstract::Var(Var::Void)) => {
                                    let file: Vec<u8> =
                                        match fs::read_to_string(string_from_utf8!(g.to_vec())) {
                                            Ok(s) => s.into_bytes(),
                                            Err(_) => return_error!("Error in opening file"),
                                        };

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

                                    let path = string_from_utf8!(ga.to_vec());
                                    let exists = Path::new(&path).exists();

                                    let mut file;

                                    if !exists {
                                        match fs::File::create(path.clone()) {
                                            Ok(f) => f,
                                            Err(_) => {
                                                return_error!("Error in creating file")
                                            }
                                        };
                                    }

                                    file = match fs::File::open(&path) {
                                        Ok(f) => f,
                                        Err(_) => return_error!("Error in opening file"),
                                    };

                                    let mut contents = String::new();

                                    match file.read_to_string(&mut contents) {
                                        Ok(_) => (),
                                        Err(_) => {
                                            return_error!("Error reading file to string")
                                        }
                                    }

                                    fs::write(
                                        string_from_utf8!(ga.to_vec()),
                                        string_from_utf8!(gb.to_vec()),
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

                                _ => return_error!("Invalid operand types"),
                            },

                            //Set access, macro can't cover these subtypeless sets so its got its own special thingy
                            b'`' => match (unpack_stack!(1), unpack_stack!(0)) {
                                (Abstract::Var(Var::Set(s)), Abstract::Var(Var::Linear(l))) => {
                                    let element = Abstract::Var(
                                        match s.get(l.clone() as i64 as usize) {
                                            Some(i) => i,
                                            _ => {
                                                return_error!(
                                                    "Could not get index ".to_string()
                                                        + &format!("{}", l.clone() as i64 as usize)
                                                        + " from Set"
                                                )
                                            }
                                        }
                                        .clone(),
                                    );

                                    clear_and_progress!();

                                    stack.push_front(element);
                                }

                                (Abstract::Var(Var::Gestalt(g)), Abstract::Var(Var::Linear(l))) => {
                                    let char = g[l.clone() as i64 as usize];

                                    clear_and_progress!();

                                    stack.push_front(Abstract::Var(Var::Gestalt(vec![char])));
                                }

                                _ => return_error!("Invalid types for operator"),
                            },

                            //Conditional, everything should've already been handled by the opening bracket.
                            b'?' => {
                                clear_and_progress!();
                            }

                            //Invalid operator
                            _ => return_error!("Invalid operator"),
                        }
                    }

                    //In this case, its not an operator, so it must be a loop
                    _ => {
                        if let Abstract::Loop(start) = unpack_stack!(2) {
                            let start = start.clone();
                            let mut recurse = true;

                            //Recurse only falsifies if both outputs are equal, and both evaluate to Vars.
                            //This ensures loops can have varying return types.
                            if let (Abstract::Var(va), Abstract::Var(vb)) =
                                (unpack_stack!(0), unpack_stack!(1))
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

            //Terminator character, immediately matches top of stack to var and returns it, if its not a var then it returns void.
            b':' => {
                return match stack.pop_front() {
                    Some(Abstract::Var(v)) => Ok(v),

                    _ => Ok(Var::Void),
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