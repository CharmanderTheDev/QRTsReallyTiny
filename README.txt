THIS README WILL FUNCTION AS THE OFFICIAL HANDBOOK FOR THE QRT LANGUAGE

QRT: QRT's Really Tiny!

QRT is a minimalist interpreted hobby language created by charmander_the_dev on GitHub.
It is built upon prefix notation and binary functions, and is interpreted here with rust.
It is not intended for practical application.

repository link: https://github.com/CharmanderTheDev/QRTsReallyTiny

BASIC SYNTAX:
QRT programs are a sequence of "expressions". An expression can either be
a literal, a reference, or an operation. Literals can take 1 of 6 forms:

    - Linear (written as a sequence of digits with 1 or 0 decimal points)
    - Gestalt (written as a sequence of ASCII characters within double quotes, allowing escapes)
    - Set (written as a sequence of expressions seperated by commas within square brackets)
        !PLEASE NOTE that due to rust's borrow checker, prohibiting multiple mutable references,
        Sets are currently .clone()'d for practically every operation they take part in. 
        This brutally unperformant policy is probably the #1 problem with this interpreter!
    - Void (written as a single underscore)
    - Input (written as a single dollar sign)
        When programs are called from the terminal, this is always equal to Void.
    - Random (written as a single percent sign)
        The random expression evaluates to a random Linear between 0 and 1 of rust's f64 type

References reference variables, and take the form of a previously assigned alias surrounded
by parentheses.

Operations take the form of ab{c}, where a is an operator, b is the primary argument, and c
is the secondary argument. Both b and c are (usually) expressions.

When the end of execution is reached, or a semicolon (;) is reached, the program will halt and
return the last evaluated value.

Comments are denoted with backslashes. A comment will not end until the next backslash is reached.

IMPORTANT OPERATIONS:

Assignment:
using the hashtag (#) as it's operator, takes in a plainly written alias
as its first argument, and an expression as its second. The variable will be assigned to 
the alias. If given a void literal (_) as an alias, the value is simply discarded.
This operation returns nothing. Example program below.
#foo{42}(42); \returns 42\

Conditional:
using the question mark (?) as it's operator, takes in a Linear as its first argument,
and any amount of QRT code as its "second argument". If the given Linear is above 0, the
code is evaluated. If it is 0 or below, it is skipped. This operator returns nothing.
Example code below.
?0{1;}?1{0;} \This code will return 0, since the 1 and its corresponding semicolon were skipped.\

Jump Definition:
using the  colon (:) as it's operator, takes in a plainly written alias as its first
argument, and any length of code as its "second argument". When this operations is called,
the place in your file where the secondary argument begins is saved as a Linear with your given
alias plus a bang (!). This operator returns nothing. Please see the following "jumps/macros" 
section to make sense of this.

Jumps/Macros:
using the bang (!) as it's operator, takes in either a linear or a gestalt as its primary argument,
and any expression as its second. If the primary argument is a Linear, it is classified as a jump.
If the primary argument is a Gestalt, it is classified as a macro.

Jumps:
"Jumps" to the place in the QRT file as given by the inputted linear, starting a new "sub-evaluation"
at that point, with the input ($) being set to the secondary argument, and returning whatever that 
sub-evaluation returns. This essentially works as a function call. Example program below.

:plusone{+${1};} \"function" defined here\
!(plusone!){2}; \"function" "called" here, will return 3\

!PLEASE NOTE that calling a jump with a literal linear, such as !0{_}, can be used for recursive purposes, as the jump
is relative to the "start" of the current sub-evaluation.!

Macros:
evaluates the given gestalt as if it were a program, setting its input to the secondary argument,
and returning whatever that evaluation returns. See example program below.
#plussone{"+${1};"} \"macro" defined here\
!(plusone){2}; \"macro" called here, will return 3\

!PLEASE NOTE that within jumps and macros, the sub-evaluation has a completely reset scope,
so variables defined outside of the functions will not be avaliable, including other functions.
One possible solution is passing a set into your function as input that includes desired/needed macros!

Looping:
using the tilde (~) as its operator, takes in a plainly written alias as its primary argument, and
any amount of QRT code as it's "second argument". The "kill id" of the loop is assigned to the given alias,
and if at any point that alias is referenced, that loop is terminated and the program moves on. 
The loop itself will return nothing. Example code below.

#a{0} \counter defined\
~kill{ \loop beginning, sets kill id to the alias "kill"\
    ?=(a){10}{ \checks if a is equal to 10\
        (kill) \references the kill variable, terminating the loop\
    }
    #a{+(a){10}} \reassigns a to a + 1, essentially incrementing it\
}
a; \returns a, or 10\

!PLEASE NOTE that by necessity, loop bodies can return NOTHING. If a tailing expression exists in a loop body
that returns a value, the loop will not function correctly!

File Access:
using the at symbol (@) as its operator, takes in a Gestalt as its primary argument, and either a Gestalt
or a Void as its second. If the secondary argument is a Gestalt, it will write that Gestalt to the given
path, creating a new file if it exists. If the secondary argument is a Void, it will not. Either way, the
initial contents of that file will be returned, throwing an error if the file did not exist and a Void was
inputted. Some examples are written below. For examples 1 and 2, he file "hello_world.txt" exists in context,
and contains the phrase "hello world". For examples 3 and 4, it does not exist.

example 1:
@"hello_world.txt"{_} \returns a Gestalt containing "hello world", with no write to hello_world.txt\

example 2:
@"hello_world.txt"{"goodbye world"} \returns a Gestalt containing "hello world", while replacing the text within the given file with "goodbye world"\

example 3:
@"hello_world.txt"{_} \throws an error stating the given file could not be found\

example 4:
@"hello_world.txt"{"goodbye world"} \creates a new file named "hello_world.txt", containing the phrase "goodbye world"\

!PLEASE NOTE that returned values for file writing, function calling, etc. can be discarded by assigning them
to a void literal, with #_(VALUE). This is useful for loops, as nothing can return values within them.!

OTHER OPERATIONS:
this section will be structured as follows: a category will be named, with a list of operators.
Each operator will have a sublist of type combinations, detailing the operation specifics for each.
In those operation specifics, "a" will refer to the primary argument, while "b" will refer to the secondary.

arithmetic:
    +
        Linear-Linear (addition): returns a + b
        Linear-Gestalt (linear coercion): coerces b to a number, then returns a + b
        Gestalt-Linear (gestalt coercion): coerces b to a string, then returns b concatenated to a
        Gestalt-Gestalt (gestalt concatenation): returns b concatentated to a
        Set-Linear (linear appending): returns a with b added to the end
        Set-Gestalt (gestalt appending): returns a with b added to the end
        Set-Set (set appending): returns a with b added to the end as a subset
    -
        Linear-Linear (subtraction) returns a - b
        Gestalt-Linear (gestalt removal) returns a with character b removed
        Set-Linear (set removal) returs a with element b removed
    *
        Linear-Linear (multiplication) returns a * b
        Set-Set (set concatenation) returns b concatenated to a
    /
        Linear-Linear (division) returns a / b
    ^
        Linear-Linear (exponentiation) returns a ^ b
        Gestalt-Void (gestalt sizing) returns the length of a
        Set-Void (sizing) returns the length of b

logic:
    &
        Linear-Linear (and) returns 1 if both a and b are >0, 0 otherwise
    |
        Linear-Linear (or) returns 1 if either a or b are >0, 0 otherwise

comparison:
    =
        Void-Void (void equality) returns 1
        Linear-Linear (linear equality) returns 1 if a is equal to b, 0 otherwise
        Gestalt-Gestalt (gestalt equality) returns 1 if a is equal to b, 0 otherwise
        Set-Set (set equality) returns 1 if a is equal to b, 0 otherwise
    >
        Linear-Linear(greater than) returns 1 if a is greater than b, 0 otherwise
        Gestalt-Linear(gestalt end removal) returns a with b characters removed from the end
        Set-Linear(set end removal) returns a with b elements removed from the end
    <
        Linear-Linear(less than) returns 1 if a is less than b, 0 otherwise
        Gestalt-Linear(gestalt start removal) returns a with b characters removed from the start
        Set-Linear(set start removal) returns a with b elements removed from the start

miscellaneous:
    `
        Linear-Linear (modulus) returns the remainder of a / b
        Gestalt-Linear (gestalt access) returns the character of a at index b (floor function is used for non-integers)
        Set-Linear (set access) returns the element of a at index b (floor function is used for non-integers)

For a good reference, take a look in the ./qrt/src/tests.rs file for a bunch of little QRT programs, 
including the sieve of eratosthenes.

THE END
happy QRT'ing!