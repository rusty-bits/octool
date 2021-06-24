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
    let section_number = 1;

    //    let misc = oc_plist.and_then(|dict| dict.get_mut("Misc")).unwrap()
    //        .as_dictionary_mut().unwrap();
    print!("\x1B[2J\x1B[H");

    for i in 0..keys.len() {
        if i == section_number {
            print!("\x1B[7m");
        }
        let val = oc_plist.get_mut(&keys[i]);
        display_value(&keys[i], val);
    }
    let _e = list.to_file_xml("test1");
}

fn display_value(key: &String, val: Option<&mut Value>) {
    match val.expect("Failed to unwrap Value") {
        Value::Dictionary(v) => {
            println!("{}\x1B[0m >", key);
            process_dict(v);
        }
        Value::String(v) => println!("{}\x1B[0m: {}", key, v),
        Value::Boolean(v) => println!("\t{}\x1B[0m: {}", key, v),
        Value::Integer(v) => println!("\t{}\x1B[0m: {}", key, v),
        Value::Data(v) => println!("\t{}\x1B[0m: {:?}", key, v),
        Value::Array(v) => process_array(v),
        _ => panic!("Can't handle this type"),
    }
}

fn process_dict(dict: &mut Dictionary) {
    let section_keys: Vec<String> = dict.keys().map(|s| s.to_string()).collect();
    for i in 0..section_keys.len() {
        display_value(&section_keys[i], dict.get_mut(&section_keys[i]));
    }
}

fn process_array(arr: &mut Vec<Value>) {
    for i in 0..arr.len() {
        display_value(&i.to_string(), Some(&mut arr[i]));
    }
}
