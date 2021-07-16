use console::{style, Term};
use plist::Value;
use std::{error::Error, io::Write};

#[derive(Debug)]
pub struct Position {
    pub file_name: String,
    pub section: [usize; 5],
    pub depth: usize,
    pub sec_key: [String; 5],
    pub item_clone: Value,
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
    write!(&*term, "\x1B[3H").unwrap();
    let list = list.as_dictionary().unwrap();
    let keys: Vec<String> = list.keys().map(|s| s.to_string()).collect();
    for (i, k) in keys.iter().enumerate() {
        display_value(k, position, list.get(k).unwrap(), &term, i, 0).unwrap();
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
    let mut tmp = String::new();
    write!(
        &*term,
        "\x1B[H\x1B[0K{}  <{}>\r\n\x1B[0K  {}",
        position.file_name,
        position.sec_key[position.depth],
        match position.item_clone {
            Value::Array(_) | Value::Dictionary(_) => {
                tmp.push_str(&style("right").reverse().to_string());
                tmp.push_str(" or ");
                tmp.push_str(&style("l").reverse().to_string());
                tmp.push_str(" to expand");
                &tmp
            }
            Value::Integer(_) => "enter/tab to edit",
            Value::String(_) => "enter/tab to edit",
            Value::Boolean(_) => "space/tab/enter to toggle",
            Value::Data(_) => "enter/tab to edit,  tab to switch between hex and string",
            _ => "XXX",
        }
    )
    .unwrap();
}

pub fn display_value(
    key: &String,
    position: &mut Position,
    plist_value: &Value,
    term: &Term,
    item_num: usize,
    d: usize,
) -> Result<(), Box<dyn Error>> {
    let mut live_item = false;
    let mut save_curs_pos = String::new();
    let mut key_style = String::new();
    let mut pre_key = '>';
    write!(&*term, "\x1B[0K\n\r{}", "    ".repeat(d))?;
    if position.section[d] == item_num {
        position.sec_key[d] = key.to_string();
        key_style.push_str("\x1B[7m");
        // current live item
        if d == position.depth {
            live_item = true;
            position.item_clone = plist_value.clone();
            save_curs_pos = "\x1B[s".to_string(); // save cursor position
        }
    }
    match plist_value {
        Value::Array(v) => {
            if live_item {
                position.sec_length[d + 1] = v.len();
            }
            if position.depth > d && position.section[d] == item_num {
                pre_key = 'v';
            }
            write!(
                &*term,
                "{} {}{}\x1B[0m  [{}] ",
                pre_key,
                key_style,
                key,
                v.len()
            )
            .unwrap();
            if position.depth > d && position.section[d] == item_num {
                let mut key = String::new();
                for i in 0..v.len() {
                    get_array_key(&mut key, &v[i], i);
                    display_value(&key, position, &v[i], term, i, d + 1)?;
                }
            }
        }
        Value::Boolean(v) => {
            match v {
                true => write!(&*term, "{}{}: {}{}", key_style, style(key).green(), save_curs_pos, v).unwrap(),
                false => write!(&*term, "{}{}: {}{}", key_style, style(key).red(), save_curs_pos, v).unwrap(),
            };
        }
        Value::Data(v) => {
            write!(
                &*term,
                "{}{}: <{}{}> | \"{}\"\x1B[0K",
                key_style,
                style(key).magenta(),
                save_curs_pos,
                hex_str_with_style(hex::encode(&*v)),
                get_lossy_string(v)
            )?;
        }
        Value::Dictionary(v) => {
            if live_item {
                position.sec_length[d + 1] = v.keys().len();
            }
            if position.depth > d && position.section[d] == item_num {
                pre_key = 'v';
            }
            write!(
                &*term,
                "{} {}{}\x1B[0m  [{}]{} ",
                pre_key,
                key_style,
                key,
                v.len(),
                save_curs_pos
            )
            .unwrap();
            if position.depth > d && position.section[d] == item_num {
                let keys: Vec<String> = v.keys().map(|s| s.to_string()).collect();
                for (i, k) in keys.iter().enumerate() {
                    display_value(&k, position, v.get(&k).unwrap(), term, i, d + 1)?;
                }
            }
        }
        Value::Integer(v) => {
            write!(
                &*term,
                "{}{}: {}{}",
                key_style,
                style(key).blue(),
                save_curs_pos,
                v
            )?;
        }
        Value::String(v) => {
            write!(
                &*term,
                "{}{:>2}\x1B[0m: {}{}",
                key_style, key, save_curs_pos, v
            )?;
        }
        _ => panic!("Can't handle this type"),
    }
    Ok(())
}

pub fn get_lossy_string(v: &Vec<u8>) -> String {
    let mut tmp = String::new();
    for c in v {
        /*        if c.is_ascii() {
            tmp.push(*c as char);
        } else {
            tmp.push('\u{fffd}');
        }*/
        if c < &32 || c > &126 {
            tmp.push('\u{fffd}');
        } else {
            tmp.push(*c as char);
        }
    }
    tmp
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

pub fn hex_str_with_style(v: String) -> String {
    let mut hex_u = String::new();
    let mut col = v.len() % 2;
    for c in v.chars() {
        if col > 1 {
            hex_u.push_str(&style(c).magenta().to_string());
        } else {
            hex_u.push(c);
        }
        col += 1;
        if col > 3 {
            col = 0;
        };
    }
    hex_u
}
