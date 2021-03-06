use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
//use std::env;

extern crate argparse;
extern crate crypto;
extern crate regex;

use argparse::{ArgumentParser, Store, StoreTrue};

extern crate handlebars;

extern crate inflector;

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

#[derive(Serialize, Deserialize, Debug)]
pub struct TypeTransformRule {
    from: String,
    to: String,
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

fn type_mapping(mut typ: String) -> String {
    let arr_regex = regex::Regex::new(r"(.+)[\[\]][\]]").unwrap();

    let mut is_array = false;

    match arr_regex.captures(typ.clone().as_ref()) {
        Some(t) => {
            typ = t[1].to_string();
            is_array = true;
        }
        None => {}
    }

    match typ.as_ref() {
        "bool" => {
            typ = "Bool".to_string();
        }
        "int8" | "int16" | "int32" | "int64" => {
            typ = str::replace(typ.as_ref(), "int", "Int").into();
        }
        "uint8" | "uint16" | "uint32" | "uint64" => {
            typ = str::replace(typ.as_ref(), "uint", "UInt").into();
        }
        "float32" | "float64" => {
            typ = str::replace(typ.as_ref(), "float", "Float").into();
        }
        "time" => {
            typ = "import \"/Time.capnp\".Time".to_string();
        }
        "duration" => {
            typ = "import \"/Duration.capnp\".Time".to_string();
        }
        "string" => {
            typ = "Text".to_string();
        }
        "Header" => {
            typ = "import \"/std_msgs/Header.capnp\".Header".to_string();
        }
        _ => {
            typ = format!(
                r#"import "/{}.capnp".{}"#,
                typ,
                typ.split("/").collect::<Vec<&str>>()[1]
            );
        }
    }

    if is_array {
        typ = format!("List({})", typ);
    }

    return typ;
}

fn compile_file(filename: &Path, namespace: &str, ast: ASTDef) {
    use handlebars::Handlebars;

    let reg = Handlebars::new();

    let template = r###"# Automatically generated by rcgenmsg for {{filename}}
@{{id}};

{{#if namespace }}
using Cxx = import "/capnp/c++.capnp";
$Cxx.namespace("{{ namespace }}");
{{/if}}

struct {{msg_name}} {
{{#each consts as |c| ~}}
  const {{c.name}} : {{{c.typ}}} = {{ c.val }};{{ c.comment }}
{{/each ~}}

{{#each fields as |f| ~}}
  {{f.name}} @{{f.id}} : {{{f.typ}}};{{ f.comment }}
{{/each ~}}
}"###;

    let mut json_data = serde_json::to_value(&ast).unwrap();

    let msg_name = filename.file_stem().unwrap().to_str().unwrap();
    let msg_id: String;

    {
        use crypto::digest::Digest;
        let mut md5 = crypto::md5::Md5::new();
        let mut buf = "".to_string();
        buf.push_str(namespace);
        buf.push_str("::");
        buf.push_str(msg_name);
        md5.input(&buf.as_bytes());

        let mut result = vec![0u8; 16];

        md5.result(&mut result);

        result[0] |= 1u8 << 7;

        let result_str: Vec<String> = result[0..8].iter().map(|b| format!("{:02x}", b)).collect();

        msg_id = "0x".to_string() + &result_str.join("");
    }

    json_data
        .as_object_mut()
        .unwrap()
        .insert("msg_name".to_string(), msg_name.into());
    json_data
        .as_object_mut()
        .unwrap()
        .insert("namespace".to_string(), namespace.into());
    json_data
        .as_object_mut()
        .unwrap()
        .insert("id".to_string(), msg_id.into());
    json_data.as_object_mut().unwrap().insert(
        "filename".to_string(),
        filename.display().to_string().into(),
    );

    if json_data.get("fields").is_some() {
        let mut i: i64 = 0;
        for mut f in json_data["fields"].as_array_mut().unwrap() {
            {
                let fname = f.get("name").unwrap().as_str().unwrap().to_string();
                //let newname = serde_json::Value::String());
                *f.pointer_mut("/name").unwrap() =
                    inflector::cases::camelcase::to_camel_case(&fname).into();
            }

            {
                let ftype = f.get("typ").unwrap().as_str().unwrap().to_string();

                *f.pointer_mut("/typ").unwrap() = type_mapping(ftype).into();
            }

            f.as_object_mut()
                .unwrap()
                .insert("id".to_string(), i.into());
            i += 1;
        }
    }

    if json_data.get("consts").is_some() {
        let mut i: i64 = 0;
        for mut f in json_data["consts"].as_array_mut().unwrap() {
            {
                let fname = f.get("name").unwrap().as_str().unwrap().to_string();
                //let newname = serde_json::Value::String());
                *f.pointer_mut("/name").unwrap() =
                    inflector::cases::camelcase::to_camel_case(&fname).into();
            }

            {
                let ftype = f.get("typ").unwrap().as_str().unwrap().to_string();

                *f.pointer_mut("/typ").unwrap() = type_mapping(ftype).into();
            }

            f.as_object_mut()
                .unwrap()
                .insert("id".to_string(), i.into());
            i += 1;
        }
    }

    // render without register
    println!("{}", reg.render_template(template, &json_data).unwrap());
    //println!("{}", serde_json::to_string(&ast).unwrap());
    // register template using given name
    //    reg.register_template_string("tpl_1", "Good afternoon, {{name}}")
    //        .unwrap();
    //    println!("{}", reg.render("tpl_1", &json!({"name": "foo"})).unwrap());
}

fn main() {
    let mut verbose = false;
    let mut filename = "".to_string();
    let mut namespace = "".to_string();
    {
        // this block limits scope of borrows by ap.refer() method
        let mut ap = ArgumentParser::new();
        ap.set_description("ROS Message Transpiler.");
        ap.refer(&mut verbose)
            .add_option(&["-v", "--verbose"], StoreTrue, "Be verbose");
        ap.refer(&mut namespace)
            .add_option(&["-n", "--namespace"], Store, "Namespace of the file");
        ap.refer(&mut filename)
            .add_argument(&"name", Store, "Name for the msg file");
        ap.parse_args_or_exit();
    }

    let path = Path::new(&filename);
    let display = path.display();

    eprintln!("Processing: {}", display);

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
                compile_file(&path, &namespace, ast);
            }
            None => panic!("Failed to parse AST from Pest tree"),
        },
    }
}

#[test]
fn it_works() {
    let path = Path::new("test_messages/BatteryInfo.msg");
    let display = path.display();

    eprintln!("Processing: {}", display);

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

                compile_file(&path, "", ast);
            }
            None => panic!("Failed to parse AST from Pest tree"),
        },
    }
}
