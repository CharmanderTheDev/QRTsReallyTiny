use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug, PartialEq)]
pub enum Var {
    Void,             //Null type
    Linear(f64),      //Numbers
    Gestalt(Vec<u8>), //Strings
    Set(Vec<Var>),    //Lists
}
impl Var {
    //Custom representation schema for vars for debugging purposes
    pub fn represent(&self) -> String {
        match self {
            Var::Void => "Void".to_string(),

            Var::Linear(l) => f64::to_string(l),

            //Unwrap is fine here since getting the vec8 into the gestalt in the first place in QRT code should ensure validity
            Var::Gestalt(g) => "\"".to_string() + core::str::from_utf8(g).unwrap() + "\"",

            Var::Set(set) => {
                let mut string: String = "[".to_string();

                for var in set {
                    string.push_str(&var.represent());
                    string.push_str(", ");
                }

                string.push(']');

                string
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Abstract {
    Var(Var),     //Values
    Operator(u8), //Generic operators, also include "loops" that haven't been initialized yet.
    Loop(usize), //Loops are a special operator that require metadata pointing to their start location
}
impl Abstract {
    //Custom representation schema for abstract for debugging purposes
    pub fn represent(&self) -> String {
        match self {
            Abstract::Var(v) => "Var(".to_string() + &v.represent() + ")",

            Abstract::Operator(o) => {
                let mut buf = vec![0; 1];

                "Operator(".to_string() + (*o as char).encode_utf8(&mut buf) + ")"
            }

            Abstract::Loop(u) => "Loop(".to_string() + u.to_string().as_str() + ")",
        }
    }
}

pub type Evaluation = Result<
    Var,
    (
        String,
        usize,
        usize,
        VecDeque<Abstract>,
        HashMap<String, Var>,
    ),
>;