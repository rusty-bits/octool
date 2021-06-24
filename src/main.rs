extern crate plist;

use plist::{Dictionary, Value};
use std::env;

fn main() {
    let file = env::args()
        .nth(1)
        .unwrap_or("INPUT/Sample.plist".to_string());

    let mut list =
        Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());

    let oc_plist = list.as_dictionary_mut().unwrap();

    let keys: Vec<String> = oc_plist.keys().map(|s| s.to_string()).collect();
    let section_number = 0;

    //    let misc = oc_plist.and_then(|dict| dict.get_mut("Misc")).unwrap()
    //        .as_dictionary_mut().unwrap();
    print!("\x1B[2J\x1B[H");

    for i in 0..keys.len() {
        if i == section_number {
            println!("\x1B[7m{}\x1B[0m >", keys[i]);
            match oc_plist.get_mut(&keys[i]).expect("Failed to unwrap Value") {
                Value::Dictionary(v) => process_dict(v),
                Value::String(v) => println!("\t{}", v),
                Value::Boolean(v) => println!("\t{}", v),
                Value::Integer(v) => println!("\t{}", v),
                Value::Data(v) => println!("\t{:?}", v),
                _ => panic!("Value type not handled yet"),
            }
        } else {
            println!("{} >", keys[i]);
        }
    }
    //    let _e = list.to_file_xml("test1");
}

fn process_dict(dict: &mut Dictionary) {
    let section_keys: Vec<String> = dict.keys().map(|s| s.to_string()).collect();
    println!("\t{:?}", section_keys);
}
