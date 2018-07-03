use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::Parser;

#[derive(Parser)]
#[grammar = "parser/rosmsg.pest"]
struct IdentParser;

fn main() {
    let path = Path::new("hello.txt");
    let display = path.display();

    println!("Processing: {}", display);

    let mut file = match File::open(&path) {
        // The `description` method of `io::Error` returns a string that
        // describes the error
        Err(why) => panic!("couldn't open {}: {}", display,
                                                   why.description()),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read {}: {}", display,
                                                   why.description()),
        Ok(_) => (),
    }

    let files = IdentParser::parse(Rule::file, &s).unwrap_or_else(|e| panic!("{}", e));

    for file in files {

        // Because ident_list is silent, the iterator will contain idents
        for pair in file.into_inner() {
            let span = pair.clone().into_span();
            // A pair is a combination of the rule which matched and a span of input
            //println!("Rule:    {:?}", pair.as_rule());
            //println!("Span:    {:?}", span);
            println!("BEGIN {:?}", pair.as_rule());
            match pair.as_rule() {
                Rule::definition => {
                    // A pair can be converted to an iterator of the tokens which make it up:
                    for inner_pair in pair.clone().into_inner() {
                        let inner_span = inner_pair.clone().into_span();
                        match inner_pair.as_rule() {
                            Rule::variable | Rule::constant => {
                                for part_pair in inner_pair.into_inner() {
                                    let part = part_pair.clone().into_span();
                                    println!("  PART {:?}:   {}", part_pair.as_rule(), part_pair.as_str())
                                }
                            },
                            _ => panic!("ERR {:?}:   {}", inner_pair.as_rule(), inner_span.as_str())
                        };
                    }
                },
                Rule::comment => {
                    println!("{}", pair.as_str());
                },
                _ => panic!("ERR {:?}:   {}", pair.as_rule(), span.as_str())
            }


            println!("END {:?}", pair.as_rule());
        }
    }
}

#[test]
fn it_works() {
    let pairs = IdentParser::parse(Rule::file, "a1 b2").unwrap_or_else(|e| panic!("{}", e));

    // Because ident_list is silent, the iterator will contain idents
    for pair in pairs {

        let span = pair.clone().into_span();
        // A pair is a combination of the rule which matched and a span of input
        println!("Rule:    {:?}", pair.as_rule());
        println!("Span:    {:?}", span);
        println!("Text:    {}", span.as_str());

        // A pair can be converted to an iterator of the tokens which make it up:
        for inner_pair in pair.into_inner() {
            let inner_span = inner_pair.clone().into_span();
            match inner_pair.as_rule() {
                Rule::itype => println!("Letter:  {}", inner_span.as_str()),
                Rule::identifier => println!("Digit:   {}", inner_span.as_str()),
                _ => println!("UNK:   {}", inner_span.as_str())
            };
        }
    }
}