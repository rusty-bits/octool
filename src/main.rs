extern crate plist;

use std::env;
use plist::Value;

fn main() {
    let file = env::args().nth(1).unwrap_or("/Users/rbits/OC-tool/INPUT/config.plist".to_string());

    let list = Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());

    let oc_plist = match list {
        Value::Dictionary(d) => d,
        _ => panic!("That's not a plist"),
    };

    println!("{:?}", oc_plist);
/*
    match list {
        Value::Dictionary(dic) => {
            for (k, v) in dic {
                println!("{}", k);
                match v {
                    Value::Array(a) => {
                        for v in a {
                            println!("\t{:?}", v);
                        };

                    },
                    _ => {
                        println!("\tno array");
                    },
                };
            };
        },
        Value::Array(_) => {},
        Value::Boolean(_) => {},
        Value::Data(_) => {},
        Value::Date(_) => {},
        Value::Integer(_) => {},
        Value::Real(_) => {},
        Value::String(_) => {},
        Value::Uid(_) => {},
        Value::__Nonexhaustive => {},
    };
*/

}
