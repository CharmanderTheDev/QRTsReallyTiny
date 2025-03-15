#[cfg(test)]
mod tests {
    use crate::qrt::{evaluate::evaluate, helpers::*, structs::Var};

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
        (comments, b"\\hello world", Var::Void),
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
        (void_literal, b"_", Var::Void),
        (input_literal, b"$", Var::Linear(42.0)),
        (random_literal, b">%{0}", Var::Linear(1.0)),

        //CONTROL

        //ARITHMETIC
        (addition, b"+2{2}", Var::Linear(4.0)),
        (subtraction, b"-3{2}", Var::Linear(1.0)),
        (multiplication, b"*3{2}", Var::Linear(6.0)),
        (division, b"/3{2}", Var::Linear(1.5))

        //LOGICAL

        //COMPARISON

        //MISCELLANEOUS


    }
}
