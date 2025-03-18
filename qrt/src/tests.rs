#[cfg(test)]
mod tests {
    use crate::qrt::{evaluate::evaluate, structs::Var};

    macro_rules! test {
        ( $( ($funcname:ident, $qrtcode:expr, $result:expr) ),*) => {
            $(
                #[test]
                fn $funcname() {
                    assert_eq!(
                        evaluate($qrtcode, &Var::Linear(42.0)),
                        Ok($result)
                    )
                }
            )*
        };
    }

    test! {
        //RUDIMENTARIES
        (comments, b"\\hello world\\2", Var::Linear(2.0)),
        (linear_literal, b"3141.5926", Var::Linear(3141.5926)),
        (gestalt_literal, b"\"hello world\"", Var::Gestalt("hello world".into())),
        (set_literal, b"[3141.5926, \"hello world\", [42, \"42\"]]", Var::Set([
            Var::Linear(3141.5926),
            Var::Gestalt("hello world".into()),
            Var::Set([
                Var::Linear(42.0),
                Var::Gestalt("42".into())
            ].to_vec())
        ].to_vec())),
        (void_literal, b"_", Var::Void(())),
        (input_literal, b"$", Var::Linear(42.0)),
        (random_literal, b"=%{%}", Var::Linear(0.0)),

        //CONTROL

        //ARITHMETIC
        (linear_linear_addition, b"+2{2}", Var::Linear(4.0)),
        (gestalt_to_linear_coercion, b"+0{\"2\"}", Var::Linear(2.0)),
        (linear_to_gestalt_concatenation, b"+\"\"{2}", Var::Gestalt(b"2".to_vec())),
        (gestalt_concatenation, b"+\"2\"{\"2\"}", Var::Gestalt(b"22".to_vec())),
        (set_linear_appending, b"+[3]{2}", Var::Set([Var::Linear(3.0), Var::Linear(2.0)].to_vec())),
        (set_geslalt_appending, b"+[3]{\"2\"}", Var::Set([Var::Linear(3.0), Var::Gestalt(b"2".to_vec())].to_vec())),
        (set_concatenation, b"+[1,2]{[3,4]}", Var::Set([
            Var::Linear(1.0),
            Var::Linear(2.0),
            Var::Linear(3.0),
            Var::Linear(4.0)
        ].to_vec())),

        (subtraction, b"-3{2}", Var::Linear(1.0)),
        (gestalt_removal, b"-\"123\"{2}", Var::Gestalt(b"12".to_vec())),
        (set_removal, b"-[1,2,3]{2}", Var::Set([Var::Linear(1.0), Var::Linear(2.0)].to_vec())),

        (multiplication, b"*3{2}", Var::Linear(6.0)),
        (division, b"/3{2}", Var::Linear(1.5)),
        (exponentiation, b"^3{2}", Var::Linear(9.0)),
        (set_length, b"^[1,2,3]{_}", Var::Linear(3.0)),

        //LOGICAL
        (and, b"[&0.0{0.0}, &1.0{0.0}, &1.0{1.0}]", Var::Set([
            Var::Linear(0.0),
            Var::Linear(0.0),
            Var::Linear(1.0)
        ].to_vec())),

        (or, b"[|0.0{0.0}, |1.0{0.0}, |1.0{1.0}]", Var::Set([
            Var::Linear(0.0),
            Var::Linear(1.0),
            Var::Linear(1.0)
        ].to_vec())),

        //COMPARISON
        (void_equality, b"[=_{_}, =1{_}]", Var::Set([
            Var::Linear(1.0),
            Var::Void(())
        ].to_vec())),
        (linear_equality, b"[=1{1}, =0{1}]", Var::Set([
            Var::Linear(1.0),
            Var::Linear(0.0)
        ].to_vec())),
        (gestalt_equality, b"[=\"a\"{\"a\"}, =\"a\"{\"b\"}]", Var::Set([
            Var::Linear(1.0),
            Var::Linear(0.0)
        ].to_vec())),
        (set_equality, b"[=[1,2,3]{[1,2,3]}, =[1,2,3]{[4,5,6]}, =[1,2,3]{[1,2]}]", Var::Set([
            Var::Linear(1.0),
            Var::Linear(0.0),
            Var::Linear(0.0)
        ].to_vec())),

        (greater_than, b"[>1{0}, >0{1}]", Var::Set([
            Var::Linear(1.0),
            Var::Linear(0.0)
        ].to_vec())),
        (gestalt_front_trim, b">\"hello\"{1}", Var::Gestalt(b"hell".to_vec())),
        (set_front_trim, b">[1,2,3]{1}", Var::Set([Var::Linear(1.0), Var::Linear(2.0)].to_vec())),


        (less_than, b"[<1{0}, <0{1}]", Var::Set([
            Var::Linear(0.0),
            Var::Linear(1.0)
        ].to_vec())),

        (gestalt_back_trim, b"<\"hello\"{1}", Var::Gestalt(b"ello".to_vec())),
        (set_back_trim, b"<[1,2,3]{1}", Var::Set([Var::Linear(2.0), Var::Linear(3.0)].to_vec())),

        //MISCELLANEOUS
        (assignment_and_aliases, b"#me{2}(me)", Var::Linear(2.0)),

        (evaluate_jump, b":plusone{+${1};}!(plusone!){1}", Var::Linear(2.0)),
        (evaluate_macro, b"!\"+${1}\"{1}", Var::Linear(2.0)),
        (evaluate_recursion, b"
            :unit{
                ?=${0}{0;}
                +!0{-${1}}{1};
            }
            !(unit!){16};",

        Var::Linear(16.0)),

        (looping, b"
            #a{0}
            ~kill{
                ?=(a){16}{
                    (kill)
                }
                #a{+(a){1}}
            }
            (a)", Var::Linear(16.0)),

        (modulus, b"`9{2}", Var::Linear(1.0)),
        (gestalt_access, b"`\"hello\"{3}", Var::Gestalt(b"l".to_vec())),
        (set_access, b"`[1,2,3]{1}", Var::Linear(2.0))

        //ADVANCED COMPOSITE PROGRAMS

        //sieve of eratosthenes
        
    }
}
