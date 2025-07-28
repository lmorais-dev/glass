use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/glass.pest"]
pub struct Parser;
