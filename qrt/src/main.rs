extern crate rand;

use std::{collections::{HashMap, VecDeque}, env, fs, vec::Vec, rand::random};

fn main() {
    let args: Vec<String> = env::args().collect();

    let file = fs::read_to_string(&args[1]).expect("No such file found").into_bytes();

    let evaluation = evaluate(&file, &Var::Linear(1.2));

    println!("{}",
        match evaluation {
            Var::Gestalt(g) => String::from_utf8(g).unwrap(),
            Var::Linear(l) => l.to_string(),
            Var::Set(_) => "set".to_string(),
            _=> String::from("unhandled")
        }
    );

}

#[derive(Clone)]
enum Var {
    Void, //Null type
    Linear(f64), //Numbers
    Gestalt(Vec<u8>), //Strings
    Set(Vec<Var>), //Lists
    Kill(usize) //Loop killer

} impl Var {
    fn bool(&self) -> bool {
        match self {
            Var::Linear(a) => {if a>&0.0 {return true}false}

            _ => false
        }
    }
}

#[derive(Clone)]
enum Abstract {
    Var(Var), //Values
    Control, //Continue evaluation after actions
    Operator(u8), //Generic operators, also include "loops" that haven't been initialized yet.
    Conditional,
    Loop(usize), //Loops contains start of looping code, on "on"
}


fn evaluate(program: &Vec<u8>, input: &Var) -> Var {

    let mut stack: VecDeque<Abstract> = VecDeque::new();

    let mut map: HashMap<String, Var> = HashMap::new();

    let mut on = 0;

    loop {
        
        match program[on] {
            //Linear literal
            b'0'..b'9' | b'.' => {
                let mut gestalt: Vec<u8>  = Vec::new();

                loop {
                    if on>=program.len() {break}
                    match program[on] {
                        b'0'..b'9' | b'.' => {
                            gestalt.push(program[on]);
                            on+=1
                        }

                        _ => {
                            break
                        }
                    }
                }

                stack.push_front(Abstract::Var(Var::Linear(String::from_utf8(gestalt).unwrap().parse::<f64>().unwrap())));
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
                            if !escape {break} 
                            else {
                                escape = false; 
                                gestalt.push(b'"')
                            }
                        }
                        
                        //Matches for backslashes (escape or escaped backslash)
                        b'\\' => {
                            if escape {
                                gestalt.push(b'\\');
                            }
                            else {escape = true}
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

                on+=1;
                stack.push_front(Abstract::Var(Var::Gestalt(gestalt)));
            }

            //Set literal begin
            b'[' => {
                on+=1;
                stack.push_front(Abstract::Operator(b'['));
            }

            //Set literal continuation and alias beginning respectively.
            b',' | b'(' => {on+=1;}

            //Secondary argument beginning, checks if there is a conditional waiting, and skips code if there is and the latest value in the stack is false (<=0.0).
            //Also checks if there is a "baby" loop, and sets the relevant beginning on it, "maturing" the loop.
            b'{' => {
                match stack.get(1).unwrap() {
                    
                    Abstract::Conditional => {
                        match unpack_linear(stack.get(0).unwrap()) {
                            Some(b) => if b>0.0  {on+=1;} else {on = find_bracket_pair(program, on+1)}
                            None => {panic!("conditional was given nonlinear argument")}
                        }

                        //Removes the conditional operator and value
                        stack.pop_front();stack.pop_front()
                    }

                    Abstract::Operator(o) => {
                        if(o==b'~'){

                            //pops off baby loop
                            stack.pop_front();

                            //pushes on complete loop
                            stack.push_front(Abstract::Loop(on+1))
                        }
                    }

                    _ => {on+=1;}
                }
            }
            

            //Set literal end
            b']' => {
                let mut set: Vec<Var> = Vec::new();

                //Breaks if the first element in q is a opening bracket, signaling beginning of set
                while match stack.front() {Some(a) => match a {Abstract::Operator(o) => match o {b'[' => false, _ => true} _ => true} None => true} {

                    //Adds variables to set in reverse order of q, maintaining original order
                    match stack.pop_front().unwrap() {
                        Abstract::Var(v) => {set.insert(0, v);}

                        _ => {}
                    }
                }

                on+=1;
                stack.push_front(Abstract::Var(Var::Set(set)));
            }
            
            //Void literal
            b'_' => {on+=1;stack.push_front(Abstract::Var(Var::Void));}
            
            //Input
            b'$' => {on+=1;stack.push_front(Abstract::Var(input.clone()));}

            //Random literal begin
            b'%' => {on+=1;stack.push_front(Abstract::Var(Var::Linear(random::<f64>())));}

            //Alias end (variable referencing)
            b')' => {
                
                //Yeah look at this gross motherfucker. It matches the first item in the q to a Gestalt and looks for a var corresponding to that Gestalt in the map. Simple 'as
                let var = map.get(&String::from_utf8(match stack.pop_front().unwrap() {Abstract::Var(v) => match v {Var::Gestalt(g) => g, _ => Vec::from([b'_'])}, _ => Vec::from([b'_'])}).unwrap()).unwrap().clone();
                stack.push_front(Abstract::Var(var));
            }

            b'?' => {stack.push_front(Abstract::Conditional);on+=1;}

            //evaluates a ton of "normal" operators (artithmetic, boolean, comparison, etc.)
            //Should also handle loop recursion, and termination if secondary variable has 
            //evaluated to the relevant kill. Should contain recursive function evaluation
            //capabilities. Should handle environmental terminal calls ("unlimited functionality, apostrophe operator")
            //
            b'}' => {
                match unpack_operator(stack.get(2)) { Some(a) => { match a {

                    //CONTROL

                        //Alias assignment
                        b'#' => {
                            map.insert(
                                String::from_utf8(unpack_gestalt(stack.get(1)).unwrap()).to_string(),
                                match stack.get(0) {
                                    Abstract::Var(v) => v,
                                    _ => Var::Void,
                                }
                            );

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            on+=1;
                        }                

                    //ARTITHMETIC

                        //Addition
                        b'+' => {
                            let sum = unpack_linear(stack.get(0)).unwrap() + unpack_linear(stack.get(1)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            on+=1;

                            stack.push_front(Abstract::Var(Var::Linear(sum)));
                        }
                        //Subtraction
                        b'-' => {
                            let difference = unpack_linear(stack.get(0)).unwrap() - unpack_linear(stack.get(1)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            on+=1;

                            stack.push_front(Abstract::Var(Var::Linear(difference)));
                        }
                        //Multiplication
                        b'*' => {
                            let product = unpack_linear(stack.get(0)).unwrap() * unpack_linear(stack.get(1)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            on+=1;

                            stack.push_front(Abstract::Var(Var::Linear(product)));
                        }
                        //Division
                        b'/' => {
                            let quotient = unpack_linear(stack.get(0)).unwrap() / unpack_linear(stack.get(1)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            on+=1;

                            stack.push_front(Abstract::Var(Var::Linear(quotient)));
                        }
                        //Exponentiation
                        b'^' => {
                            let power = unpack_linear(stack.get(1)).unwrap().powf(unpack_linear(stack.get(0)).unwrap());

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            on+=1;

                            stack.push_front(Abstract::Var(Var::Linear(power)));
                        }
                        
                    //LOGICAL
                        
                        //And
                        b'&' => {
                            let truth = unpack_bool(stack.get(0)).unwrap() && unpack_bool(stack.get(1)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            stack.push_front(Abstract::Var(Var::Linear(truth?1.0:0.0)))
                        }

                        //Or
                        b'&' => {
                            let truth = unpack_bool(stack.get(0)).unwrap() || unpack_bool(stack.get(1)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            stack.push_front(Abstract::Var(Var::Linear(truth?1.0:0.0)));
                        }

                    //COMPARISON

                        //Greater than
                        b'>' => {
                            let truth = unpack_linear(stack.get(1)).unwrap() > unpack_linear(stack.get(0)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            stack.push_front(Abstract::Var(Var::Linear(truth?1.0:0.0)))
                        }

                        //Equal to 
                        b'=' => {
                            let truth = unpack_linear(stack.get(1)).unwrap() == unpack_linear(stack.get(0)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            stack.push_front(Abstract::Var(Var::Linear(truth?1.0:0.0)))
                        }

                        //Less than
                        b'<' => {
                            let truth = unpack_linear(stack.get(1)).unwrap() < unpack_linear(stack.get(0)).unwrap();

                            stack.pop_front;stack.pop_front;stack.pop_front;
                            stack.push_front(Abstract::Var(Var::Linear(truth?1.0:0.0)))
                        }

                    //MISC

                        //Evaluation
                        b'!' => {
                            let eva = evaluate(
                                unpack_gestalt(stack.get(1)).unwrap(),
                                match stack.get(2) {
                                    Abstract::Var(v) => v,
                                    _ => Var::Void,
                                }
                            );

                            stack.pop_front;stack.pop_front;stack.pop_front;

                            stack.push_front(eva);

                            on+=1;
                        }

                        //Reading/writing files
                        b'@' => {

                        }

                        //Set access
                        b'`' => {

                        }

                        //Wildcard/terminal access
                        b'\'' => {

                        }

                }}

                    //In this case, its not an operator, so either a conditional or loop
                    None => {
                        match stack.get(2).unwrap() {

                            Abstract::Conditional => {on+=1;}

                            Abstract::Loop(s) => {
                                //If the evaluation of the secondary argument is gestalt equal to the first,
                                //Then the loop is terminated.
                                if gestalt_equivilence(unpack_gestalt(stack.get(0)), unpack_gestalt(stack.get(1))) {
                                    //removes the whole of the loop code and moves forward
                                    stack.pop_front;stack.pop_front;stack.pop_front;

                                    on+=1;
                                } else {
                                    //removes the secondary argument and starts over at the loop's associated on value
                                    stack.pop_front;
                                    
                                    on = s;
                                }
                            }
                        }
                    }
                }
            }

            //Anything else (valid) should be a normal operator, so they just get appended.
            //Loops are included in here because they are initially appended as uninitialized.
            _ => {
                stack.push_front(Abstract::Operator(program[on]));on+=1;
            },
        }

        //Breaks if the queue has one literal element
        if match stack.front() {Some(v) => {match v {Abstract::Var(_) => true, _=> false}} None => false} & (stack.len() == 1){break}
    
    }

    return match stack.pop_front().unwrap() {
        Abstract::Var(v) => v,
    
        _ => Var::Void
    };
}

//Removes comments and whitespace
fn compile(program: &Vec<u8>) {

}

fn unpack_linear(packed: &Abstract) -> Option<f64> {
    return match packed {
        Abstract::Var(v) => {
            match v {
                Var::Linear(b) => Some(*b),

                _ => None
            }
        }

        _ => None
    }
}

fn unpack_gestalt(packed: &Abstract) -> Option<Vec<u8>> {
    return match packed {
        Abstract::Var(v) => {
            match v {
                Var::Gestalt(b) => Some(b.clone()),
                
                _ => None
            }
        }
       _ => None
    }
}

fn unpack_operator(packed: &Abstract) -> Option<u8> {
    return match packed {
        Abstract::Operator(o) => Some(o),
        _ => None
    }
}

fn unpack_bool(packed: &Abstract) -> Option<bool> {
    match packed {
        Abstract::Var(v) => {
            Some(v.bool())
        }

        _ => None
    }
}

fn gestalt_equivilence(a: &Vec<u8>, b: &Vec<u8>) -> boolean {
    String::from_utf8(a) == String::from_utf8(b)
}

//Helper function, used to find the end of secondary args. Expects to start the character directly after the first bracket.
//Returns the position directly after the pairing bracket.
fn find_bracket_pair(program: &Vec<u8>, mut on: usize) -> usize {
        let (mut bracket_number, mut gestalt, mut escape) = (1, false, false);

        while bracket_number!=0 {

            match program[on] {

                //Matches for opening brackets
                b'{' => {
                    if !gestalt {bracket_number+=1;}
                }

                //Matches for closing brackets
                b'}' => {
                    if !gestalt {bracket_number-=1;}
                }

                //Matches for quotes (gestalt initiation, termination, or escaped quote)
                b'"' => {
                    if !gestalt {gestalt=true;}
                    else {
                        if !escape {
                            gestalt=false;
                        }
                    }
                }
                
                //Matches for backslashes (escape initiation or escaped backslash)
                b'\\' => {
                    if !escape {
                        escape=true
                    }
                }

                //Matches for other characters and unescapes
                _ => {
                    if escape {
                        escape = false
                    }
                }
            }

            on+=1;
        }

        on
    }