use std::{env, fs, vec::Vec};

fn main() {
    let args: Vec<String> = env::args().collect();

    let file = fs::read_to_string(&args[1]).expect("No such file found");
    let evaluation = evaluate(file.into_bytes(), 0);

    println!("{}",
        match evaluation {
            Var::Gestalt(g) => String::from_utf8(g).unwrap(),
            _=> String::from("non-gestalt")
        }
    );

}

enum Var {
    Void,
    Continue,
    Linear(f64),
    Gestalt(Vec<u8>),
    Set(Vec<Var>),
}

fn evaluate(program: Vec<u8>, mut on: usize) -> Var {
    match program[on] {

        //Linear literal
        

        //Gestalt literal
        b'"' => {
            let mut gestalt: Vec<u8> = Vec::new();
            let mut escape = false;
            loop {
                on+=1;
                match program[on] {
                    //Matches for quotes (gestalt termination or escaped quote)
                    b'"' => {
                        if !escape {break;} 
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
            Var::Gestalt(gestalt)
        }
        

        
        _ => {
            Var::Void
        }
    }
}