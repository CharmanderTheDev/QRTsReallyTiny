use super::{helpers::*, structs::*};

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

    //Used to assign killids to loops
    let mut killidon: usize = 0;

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

                (Abstract::Var(Var::Void(_)), _) | (_, Abstract::Var(Var::Void(_))) => {
                    clear_and_progress!();

                    stack.push_front(Abstract::Var(Var::void()));
                }

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
                    "Error getting index ".to_string() + &format!("{}", $index) + " from stack"
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
    'main: loop {
        //print!("{}", program[on] as char); //Silly debug tool

        //Returns if the end of the program has been reached or exceeded
        if on >= program.len() {
            return match stack.pop_front() {
                Some(Abstract::Var(v)) => Ok(v),

                _ => Ok(Var::void()),
            };
        }

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

                //In the case where a trailing comment exists in the program, the evaluator will detect that on has gone out of bounds
                //And continue back to the loop head, where the evaluator will return the head of the stack as usual.
                while program[on] != b'\\' {
                    on += 1;

                    if on >= program.len() {
                        continue 'main;
                    }
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
                    if let Some(Abstract::Var(v)) = stack.pop_front() {
                        set.insert(0, v.clone());
                    } else {
                        return_error!("Likely: no opening bracket given for set literal")
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
                stack.push_front(Abstract::Var(Var::void()));
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
                            //assigns latest killid to the given variable name, and advances it.
                            map.insert(
                                string_from_utf8!(unpack_var!(
                                    Gestalt,
                                    0,
                                    "Invalid kill variable name given to loop"
                                )),
                                Var::Kill(killidon),
                            );

                            //pops off killid and baby loop
                            stack.pop_front();
                            stack.pop_front();

                            //pushes on complete loop with correct killid, and the loop's starting position as a linear
                            stack.push_front(Abstract::Loop(killidon));
                            stack.push_front(Abstract::Var(Var::Linear((on + 1) as f64)));

                            //advances killidon, and the on into the loop code
                            killidon += 1;
                            on += 1;
                        } else if o == &b'?' {
                            if unpack_var!(Linear, 0, "Invalid conditional type") > 0.0 {
                                on += 1;
                            } else {
                                on = find_bracket_pair(program, on + 1);
                            }

                            stack.pop_front();
                            stack.pop_front(); /*pops conditional and condition*/
                        } else {
                            on += 1;
                        }
                    }

                    _ => {
                        on += 1;
                    }
                }
            }

            //Alias assignment and loop beginning, assigning the given name a relevant killid later.
            b'#' | b'~' => {
                let operator = program[on];

                on += 1;

                let mut name: Vec<u8> = Vec::new();

                while !(program[on] == b'{' || program[on] == b'!') {
                    name.push(program[on]);
                    on += 1;
                }

                if program[on] == b'!' {
                    return_error!("Bangs (!) not allowed in variable names")
                }

                //wait for eval and save the name and operator to stack
                stack.push_front(Abstract::Operator(operator));
                stack.push_front(Abstract::Var(Var::Gestalt(name)));
            }

            //Jump assignment
            b':' => {
                on += 1;

                let mut name: Vec<u8> = Vec::new();

                while !(program[on] == b'{' || program[on] == b'!') {
                    name.push(program[on]);
                    on += 1;
                }

                if program[on] == b'!' {
                    return_error!("Bangs (!) not allowed in function names")
                }

                //Inserts the correct jump place as a variable
                map.insert(string_from_utf8!(name) + "!", Var::Linear((on + 1) as f64));

                //Skips to after the bracket for find_bracket_pair to work correctly
                on = find_bracket_pair(program, on + 2);
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

                //Checks if either varname or varname! exists, since jumps (functions kinda) add bangs in definition
                let var = if map.contains_key(&string_from_utf8!(varname.clone())) {
                    unpack_map!(&string_from_utf8!(varname)).clone()
                } else {
                    return_error!("Variable does not exist")
                };

                if let Var::Kill(killid) = var {
                    //Destroys all values until reaching the loop
                    while stack.get(1) != Some(&Abstract::Loop(killid)) {
                        stack.pop_front();
                    }

                    //Sets the on to after the killed loop
                    on = find_bracket_pair(
                        program,
                        unpack_var!(Linear, 0, "Error getting starting linear in loop kill") as i64
                            as usize,
                    );

                    //Removes both the loop and its starting position linear from the stack
                    stack.pop_front();
                    stack.pop_front();
                } else {
                    stack.push_front(Abstract::Var(var));
                }
            }

            //Closing bracket, evaluates all operators
            b'}' => {
                //Loops dont check the second index of stack, so they get looked at first to avoid error
                if let Abstract::Loop(_) = unpack_stack!(1) {
                    //If the end of the loop has been reached, that means no kill variable was invoked, and recursion can simply take place
                    on = unpack_var!(Linear, 0, "Error retrieving loop start for recursion") as i64
                        as usize;
                        
                } else {
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
                                        )),
                                        match unpack_stack!(0) {
                                            Abstract::Var(v) => v.clone(),
                                            _ => Var::void(),
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
                                        }),

                                        (Set, Set, Set|mut a: Vec<Var>, b: Vec<Var>| -> Result<Vec<Var>, &str> {
                                            for var in b {a.push(var.clone())}
                                            Ok(a)
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
                                        (Linear, Linear, Linear|a: f64, b: f64| -> Result<f64, &str> {Ok(a.powf(b))}),

                                        //Special case for giving the length of a Set
                                        (Set, Void, Linear|a: Vec<Var>, _b: ()| -> Result<f64, &str> {Ok(a.len() as f64)})
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
                                    multi_operate!(
                                        (Void, Void, Linear|_a: (), _b: ()| -> Result<f64, &str> {Ok(1.0)}),

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
                                        match evaluate(&program[*jmp as i64 as usize..], v) {
                                            Ok(eva) => {
                                                clear_and_progress!();
                                                stack.push_front(Abstract::Var(eva))
                                            }
                                            Err((msg, funcon, funclineon, stack, map)) => {
                                                return Result::Err((
                                                    msg + " \n(In function evaluated at "
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
                                        let eva = match evaluate(g, v) {
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
                                    (
                                        Abstract::Var(Var::Gestalt(g)),
                                        Abstract::Var(Var::Void(_)),
                                    ) => {
                                        let file: Vec<u8> =
                                            match fs::read_to_string(string_from_utf8!(g.to_vec()))
                                            {
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
                                            &("Unable to write file at ".to_string()
                                                + &on.to_string()),
                                        );

                                        clear_and_progress!();

                                        stack.push_front(Abstract::Var(if exists {
                                            Var::Gestalt(contents.into())
                                        } else {
                                            Var::void()
                                        }));
                                    }

                                    _ => return_error!("Invalid operand types"),
                                },

                                //Set & gestalt indexing, macro can't cover these subtypeless sets so its got its own special thingy
                                b'`' => match (unpack_stack!(1), unpack_stack!(0)) {
                                    (Abstract::Var(Var::Set(s)), Abstract::Var(Var::Linear(l))) => {
                                        let element = Abstract::Var(
                                            match s.get(*l as i64 as usize) {
                                                Some(i) => i,
                                                _ => {
                                                    return_error!(
                                                        "Could not get index ".to_string()
                                                            + &format!("{}", *l as i64 as usize)
                                                            + " from Set"
                                                    )
                                                }
                                            }
                                            .clone(),
                                        );

                                        clear_and_progress!();

                                        stack.push_front(element);
                                    }

                                    (
                                        Abstract::Var(Var::Gestalt(g)),
                                        Abstract::Var(Var::Linear(l)),
                                    ) => {
                                        let char = g[*l as i64 as usize];

                                        clear_and_progress!();

                                        stack.push_front(Abstract::Var(Var::Gestalt(vec![char])));
                                    }

                                    //Special modulus functionality
                                    (
                                        Abstract::Var(Var::Linear(a)),
                                        Abstract::Var(Var::Linear(b)),
                                    ) => {
                                        let result = a % b;

                                        clear_and_progress!();

                                        stack.push_front(Abstract::Var(Var::Linear(result)));
                                    }

                                    _ => return_error!("Invalid types for operator"),
                                },

                                //Conditional, everything should've already been handled by the opening bracket.
                                //If this point is reached, then
                                b'?' => {}

                                //Invalid operator
                                _ => return_error!("Invalid operator"),
                            }
                        }

                        //In this case, its not an operator, so it must be a loop
                        _ => {
                            return_error!("Invalid value in place of operator")
                        }
                    }
                }
            }

            //Terminator character, immediately matches top of stack to var and returns it, if its not a var then it returns void.
            b';' => {
                return match stack.pop_front() {
                    Some(Abstract::Var(v)) => Ok(v),

                    _ => Ok(Var::void()),
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
