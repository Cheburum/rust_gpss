#[macro_use]
extern crate array_macro;
#[macro_use]
extern crate log;
extern crate env_logger;

mod interpreter;
use interpreter::Interpreter;

mod lexer;
use lexer::lexer;

fn main() {
    env_logger::init();
    let mut interpreter = Interpreter::build_test_interpreter();
    interpreter.process();
}
