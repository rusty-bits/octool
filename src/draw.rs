extern crate hex;
extern crate plist;

//use plist::Dictionary;
use plist::Value;

use std::io::Write;

use console::Term;

#[derive(Debug)]
pub struct Position {
    pub file_name: String,
    pub section: [usize; 5],
    pub sec_length: [usize; 5],
    pub depth: usize,
    pub can_expand: bool,
}

impl Position {
    pub fn up(&mut self) {
        if self.section[self.depth] > 0 {
            self.section[self.depth] -= 1;
        }
    }

    pub fn down(&mut self) {
        if self.section[self.depth] < self.sec_length[self.depth] - 1 {
            self.section[self.depth] += 1;
        }
    }

    pub fn left(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    pub fn right(&mut self) {
        if self.can_expand {
            self.depth += 1;
            self.section[self.depth] = 0;
        }
    }
}


pub fn draw_screen(position: &mut Position, list: &Value, screen: &Term) {
    write!(
        &*screen,
        "\x1B[2J\x1B[H{}\r\n",
        position.file_name
    )
    .unwrap();
    let list = list.as_dictionary().unwrap();
    let keys: Vec<String> = list.keys().map(|s| s.to_string()).collect();
    for (i, k) in keys.iter().enumerate() {
        display_value(k, position, list.get(k).unwrap(), &screen, i, 0);
    }
    write!(&*screen, "\n\n\r{:?}{}", position.section, position.depth).unwrap();
    write!(&*screen, "\x1B[u").unwrap();
    screen.flush().unwrap();
}

pub fn display_value(
    key: &String,
    position: &mut Position,
    oc_plist: &Value,
    screen: &Term,
    item_num: usize,
    d: usize,
) {
    let mut live_item = false;
    let mut ls = String::new();
    write!(&*screen, "\n\r").unwrap();
    for _ in 0..d {
        write!(&*screen, "    ").unwrap();
    }
    if position.section[d] == item_num {
        write!(&*screen, "\x1B[7m").unwrap();
        if d == position.depth {
            // current live item
            live_item = true;
            position.can_expand = false;
            ls = "\x1B[s".to_string();
        }
    }
    match oc_plist {
        Value::Array(v) => {
            if live_item {
                position.can_expand = true;
                position.sec_length[d + 1] = v.len();
                if v.len() == 0 {
                    position.can_expand = false;
                }
            }
            write!(&*screen, "{}\x1B[0m >", key).unwrap();
            if position.depth > d && position.section[d] == item_num {
                for i in 0..v.len() {
                    display_value(&i.to_string(), position, &v[i], screen, i, d + 1);
                }
            }
        }
        Value::Boolean(v) => match v {
            true => write!(&*screen, "\x1B[32m{}\x1B[0m: {}", key, v).unwrap(),
            false => write!(&*screen, "\x1B[31m{}\x1B[0m: {}", key, v).unwrap(),
        },
        Value::Data(v) => {
            write!(
                &*screen,
                "\x1B[33m{}\x1B[0m: 0x{}{} | {}",
                key, ls, hex::encode_upper(&*v), String::from_utf8_lossy(v)
            )
            .unwrap();
        }
        Value::Dictionary(v) => {
            if live_item {
                position.can_expand = true;
                position.sec_length[d + 1] = v.keys().len();
                if v.keys().len() == 0 {
                    position.can_expand = false;
                }
            }
            write!(&*screen, "{}\x1B[0m > ", key).unwrap();
            let keys: Vec<String> = v.keys().map(|s| s.to_string()).collect();
            if position.depth > d && position.section[d] == item_num {
                for (i, k) in keys.iter().enumerate() {
                    display_value(&k, position, v.get(&k).unwrap(), screen, i, d + 1);
                }
            }
        }
        Value::Integer(v) => {
            write!(&*screen, "\x1B[34m{}\x1B[0m: {}{}", key, ls, v).unwrap();
        }
        Value::String(v) => {
            write!(&*screen, "{:>2}\x1B[0m: {}{}", key, ls, v).unwrap();
        }
        _ => panic!("Can't handle this type"),
    }
}
