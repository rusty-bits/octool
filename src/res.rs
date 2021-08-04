use crate::draw::Position;
use serde_json::Value;
use std::path::Path;

pub struct Resources {
    pub acidanthera: Value,
    pub dortania: Value,
    pub octool_config: Value,
    pub config: plist::Value,
}

pub fn show_res_path(resources: &Resources, position: &Position) {
    let full_res: String;
    if position.sec_key[0].as_str() == "UEFI" && position.sec_key[1].as_str() == "Drivers" {
        full_res= resources.config.as_dictionary().unwrap()
            .get("UEFI").unwrap().as_dictionary().unwrap()
            .get("Drivers").unwrap().as_array().unwrap()
            .get(position.sec_key[2].parse::<usize>().unwrap()).unwrap().as_string().unwrap().to_string();
    } else {
        full_res = position.sec_key[position.depth].clone();
    }
    let ind_res: &str = full_res.split('/').collect::<Vec<&str>>().last().unwrap();
    let stem: Vec<&str> = ind_res.split('.').collect();
    println!("\n {} - {}\x1B[0K", stem[0], ind_res);
    println!(
        "inside INPUT dir?\x1B[0K\n {:?}\x1B[0K\n\x1B[2K",
        Path::new("INPUT").join(ind_res).exists()
    );
    println!("{} in dortania_config\x1B[0K", stem[0]);
    println!(
        "{:?}\x1B[0K\n\x1B[2K",
        resources.dortania[stem[0]]["versions"][0]["links"]["release"]
    );
    let acid_child = resources.acidanthera[ind_res].clone();
    println!("{} in acidanthera_config\x1B[0K", ind_res);
    print!("{:?}\x1B[0K\n\x1B[2K", acid_child);
    match acid_child["parent"].to_owned() {
        serde_json::Value::String(s) => {
            println!("parent {} in acidanthera_config\x1B[0K", s);
            println!(
                "{:?}\x1B[0K",
                resources.acidanthera[s]["versions"][0]["links"]["release"]
            );
        }
        _ => (),
    }
}
