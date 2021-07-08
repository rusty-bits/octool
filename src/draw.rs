use console::{style, Term};
use plist::Value;
use std::io::Write;

#[derive(Debug)]
pub struct Position {
    pub file_name: String,
    pub section: [usize; 5],
    pub depth: usize,
    pub sec_key: [String; 5],
    pub sec_length: [usize; 5],
}

impl Position {
    pub fn up(&mut self) {
        if self.section[self.depth] > 0 {
            self.section[self.depth] -= 1;
            self.sec_length[self.depth + 1] = 0;
        }
    }

    pub fn down(&mut self) {
        if self.section[self.depth] < self.sec_length[self.depth] - 1 {
            self.section[self.depth] += 1;
            self.sec_length[self.depth + 1] = 0;
        }
    }

    pub fn left(&mut self) {
        if self.depth > 0 {
            self.sec_length[self.depth + 1] = 0;
            self.sec_key[self.depth] = "".to_string();
            self.depth -= 1;
        }
    }

    pub fn right(&mut self) {
        if self.sec_length[self.depth + 1] > 0 {
            self.depth += 1;
            self.section[self.depth] = 0;
        }
    }
}

pub fn draw_screen(position: &mut Position, list: &Value, term: &Term) {
    write!(&*term, "\x1B[2H").unwrap();
    let list = list.as_dictionary().unwrap();
    let keys: Vec<String> = list.keys().map(|s| s.to_string()).collect();
    for (i, k) in keys.iter().enumerate() {
        display_value(k, position, list.get(k).unwrap(), &term, i, 0);
    }
    display_footer(position, term);
    display_header(position, term);
    write!(&*term, "\x1B[u").unwrap();
}

pub fn display_footer(position: &mut Position, term: &Term) {
    write!(
        &*term,
        "\x1B[0J\r\n\nDebug stuff:\r\n{:?} {:?} {} {:?}",
        position.section, position.sec_length, position.depth, position.sec_key
    )
    .unwrap();
}

pub fn display_header(position: &mut Position, term: &Term) {
    write!(&*term, "\x1B[H{} \x1B[0K\r\n", position.file_name).unwrap();
}

pub fn display_value(
    key: &String,
    position: &mut Position,
    oc_plist: &Value,
    term: &Term,
    item_num: usize,
    d: usize,
) {
    let mut live_item = false;
    let mut ls = String::new();
    write!(&*term, "\x1B[0K\n\r{}", "    ".repeat(d)).unwrap();
    if position.section[d] == item_num {
        write!(&*term, "\x1B[7m").unwrap();
        position.sec_key[d] = key.to_string();
        // current live item
        if d == position.depth {
            live_item = true;
            ls = "\x1B[s".to_string(); // save cursor position
        }
    }
    match oc_plist {
        Value::Array(v) => {
            if live_item {
                position.sec_length[d + 1] = v.len();
            }
            write!(&*term, "{}\x1B[0m >", key).unwrap();
            if position.depth > d && position.section[d] == item_num {
                let mut key = String::new();
                for i in 0..v.len() {
                    get_array_key(&mut key, &v[i], i);
                    display_value(&key, position, &v[i], term, i, d + 1);
                }
            }
        }
        Value::Boolean(v) => {
            match v {
                true => write!(&*term, "{}: {}", style(key).green(), v).unwrap(),
                false => write!(&*term, "{}: {}", style(key).red(), v).unwrap(),
            };
        }
        Value::Data(v) => {
            write!(
                &*term,
                "{}: 0x{}{} | ",
                style(key).yellow(),
                ls,
                hex::encode_upper(&*v) //                String::from_utf8_lossy(v)
            )
            .unwrap();
            display_lossy_string(v, term);
        }
        Value::Dictionary(v) => {
            if live_item {
                position.sec_length[d + 1] = v.keys().len();
            }
            write!(&*term, "{}\x1B[0m > ", key).unwrap();
            let keys: Vec<String> = v.keys().map(|s| s.to_string()).collect();
            if position.depth > d && position.section[d] == item_num {
                for (i, k) in keys.iter().enumerate() {
                    display_value(&k, position, v.get(&k).unwrap(), term, i, d + 1);
                }
            }
        }
        Value::Integer(v) => {
            write!(&*term, "{}: {}{}", style(key).blue(), ls, v).unwrap();
        }
        Value::String(v) => {
            write!(&*term, "{:>2}\x1B[0m: {}{}", key, ls, v).unwrap();
        }
        _ => panic!("Can't handle this type"),
    }
}

pub fn display_lossy_string(v: &Vec<u8>, term: &Term) {
    let mut tmp = String::new();
    for c in v {
        if c < &32 || c > &126 {
            tmp.push('\u{fffd}');
        } else {
            tmp.push(*c as char);
        }
    }
    write!(&*term, "{}", tmp).unwrap();
}

fn get_array_key(key: &mut String, v: &plist::Value, i: usize) {
    match v {
        Value::Dictionary(d) => {
            for k in ["Name", "Path", "BundlePath", "Comment"] {
                if d.contains_key(k) {
                    *key = d.get(k).unwrap().clone().into_string().unwrap();
                    break; // stop after first match
                }
            }

            if key.len() == 0 {
                *key = i.to_string();
            }

            match d.get("Enabled") {
                Some(Value::Boolean(b)) => {
                    if *b {
                        *key = style(&*key).green().to_string();
                    } else {
                        *key = style(&*key).red().to_string();
                    }
                }
                _ => (),
            }
        }
        _ => *key = i.to_string(),
    }
}
