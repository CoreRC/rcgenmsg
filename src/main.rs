use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
//use std::env;

extern crate argparse;

use argparse::{ArgumentParser, Store, StoreTrue};

extern crate handlebars;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::Parser;

#[derive(Parser)]
#[grammar = "parser/rosmsg.pest"]
struct IdentParser;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConstDef {
    typ: String,
    name: String,
    val: String,
    comment: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FieldDef {
    typ: String,
    name: String,
    comment: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ASTDef {
    consts: Vec<ConstDef>,
    fields: Vec<FieldDef>,
}

fn pest_to_ast(pes: &pest::iterators::Pair<Rule>) -> Option<ASTDef> {
    let mut ast = ASTDef {
        consts: Vec::new(),
        fields: Vec::new(),
    };

    // Because ident_list is silent, the iterator will contain idents
    for pair in pes.clone().into_inner() {
        let span = pair.clone().into_span();
        match pair.as_rule() {
            Rule::constant => {
                let mut typ: Option<String> = None;
                let mut name: Option<String> = None;
                let mut val: Option<String> = None;
                let mut comment: Option<String> = None;

                for inner_pair in pair.clone().into_inner() {
                    let inner_span = inner_pair.clone().into_span();
                    match inner_pair.as_rule() {
                        Rule::itype => {
                            typ = Some(inner_pair.as_str().to_string());
                        }
                        Rule::identifier => {
                            name = Some(inner_pair.as_str().to_string());
                        }
                        Rule::value => {
                            val = Some(inner_pair.as_str().to_string());
                        }
                        Rule::comment => {
                            comment = Some(inner_pair.as_str().to_string());
                        }
                        _ => panic!("ERR {:?}:   {}", inner_pair.as_rule(), inner_span.as_str()),
                    };
                }

                ast.consts.push(ConstDef {
                    typ: typ.expect("No type detected"),
                    name: name.expect("No name detected"),
                    val: val.expect("No val detected"),
                    comment: comment.unwrap_or("".to_string()),
                })
            }
            Rule::variable => {
                let mut typ: Option<String> = None;
                let mut name: Option<String> = None;
                let mut comment: Option<String> = None;

                for inner_pair in pair.clone().into_inner() {
                    let inner_span = inner_pair.clone().into_span();
                    match inner_pair.as_rule() {
                        Rule::itype => {
                            typ = Some(inner_pair.as_str().to_string());
                        }
                        Rule::identifier => {
                            name = Some(inner_pair.as_str().to_string());
                        }
                        Rule::comment => {
                            comment = Some(inner_pair.as_str().to_string());
                        }
                        _ => panic!("ERR {:?}:   {}", inner_pair.as_rule(), inner_span.as_str()),
                    };
                }

                ast.fields.push(FieldDef {
                    typ: typ.expect("No type detected"),
                    name: name.expect("No name detected"),
                    comment: comment.unwrap_or("".to_string()),
                })
            }
            Rule::comment => {
                // println!("{}", pair.as_str());
            }
            _ => panic!("ERR {:?}:   {}", pair.as_rule(), span.as_str()),
        }
    }

    return Some(ast);
}

fn compile_file(ast: ASTDef) {
    use handlebars::Handlebars;

    let mut reg = Handlebars::new();

    let template = r###"struct BatteryState {
{{#each consts as |c| ~}}
  const {{c.name}} @0 : {{c.typ}} = {{ c.val }};{{ c.comment }}
{{/each ~}}

{{#each fields as |f| ~}}
  {{f.name}} @0 : {{f.typ}};{{ f.comment }}
{{/each ~}}
}"###;
    // render without register
    println!("{}", reg.render_template( template, &serde_json::to_value(&ast).unwrap()).unwrap());
    //println!("{}", serde_json::to_string(&ast).unwrap());
    // register template using given name
//    reg.register_template_string("tpl_1", "Good afternoon, {{name}}")
//        .unwrap();
//    println!("{}", reg.render("tpl_1", &json!({"name": "foo"})).unwrap());
}

fn main() {
    let mut verbose = false;
    let mut filename = "".to_string();
    {
        // this block limits scope of borrows by ap.refer() method
        let mut ap = ArgumentParser::new();
        ap.set_description("ROS Message Transpiler.");
        ap.refer(&mut verbose)
            .add_option(&["-v", "--verbose"], StoreTrue, "Be verbose");
        ap.refer(&mut filename)
            .add_argument(&"--name", Store, "Name for the msg file");
        ap.parse_args_or_exit();
    }

    let path = Path::new(&filename);
    let display = path.display();

    println!("Processing: {}", display);

    let mut file = match File::open(&path) {
        // The `description` method of `io::Error` returns a string that
        // describes the error
        Err(why) => panic!("couldn't open file {}: {}", display, why.description()),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read {}: {}", display, why.description()),
        Ok(_) => (),
    }

    let files = IdentParser::parse(Rule::file, &s).unwrap_or_else(|e| panic!("{}", e));

    match files.peekable().peek() {
        None => panic!("File is empty!"),
        Some(pes) => match pest_to_ast(pes) {
            Some(ast) => {
                if verbose {
                    println!("{:?}", ast);
                }
                compile_file(ast);
            }
            None => panic!("Failed to parse AST from Pest tree"),
        },
    }
}

#[test]
fn it_works() {
    let path = Path::new("hello.txt");
    let display = path.display();

    println!("Processing: {}", display);

    let mut file = match File::open(&path) {
        // The `description` method of `io::Error` returns a string that
        // describes the error
        Err(why) => panic!("couldn't open {}: {}", display, why.description()),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read {}: {}", display, why.description()),
        Ok(_) => (),
    }

    let files = IdentParser::parse(Rule::file, &s).unwrap_or_else(|e| panic!("{}", e));

    match files.peekable().peek() {
        None => panic!("File is empty!"),
        Some(pes) => match pest_to_ast(pes) {
            Some(ast) => {
                println!("{:?}", ast);

                compile_file(ast);
            }
            None => panic!("Failed to parse AST from Pest tree"),
        },
    }
}
