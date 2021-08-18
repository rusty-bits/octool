use console::{style, Term};
use plist::Value;
use std::{error::Error, io::Write};

use crate::res::has_parent;

#[derive(Debug)]
pub struct Position {
    pub file_name: String, // name of config.plist
    pub section_num: [usize; 5], // selected section for each depth
    pub depth: usize, // depth of plist we are looking at
    pub sec_key: [String; 5], // key of selected section
    pub item_clone: Value, // copy of highlighted item (can we get rid of this?)
    pub sec_length: [usize; 5], // number of items in current section
    pub resource_sections: Vec<String>, // concat name of sections that contain resources
    pub build_type: String, // building release or debug version
}

impl Position {
    pub fn up(&mut self) {
        if self.section_num[self.depth] > 0 {
            self.section_num[self.depth] -= 1;
            self.sec_length[self.depth + 1] = 0;
        }
    }
    pub fn down(&mut self) {
        if self.section_num[self.depth] < self.sec_length[self.depth] - 1 {
            self.section_num[self.depth] += 1;
            self.sec_length[self.depth + 1] = 0;
        }
    }
    pub fn left(&mut self) {
        if self.depth > 0 {
            self.sec_length[self.depth + 1] = 0;
            self.sec_key[self.depth].clear();
            self.depth -= 1;
        }
    }
    pub fn right(&mut self) {
        if self.sec_length[self.depth + 1] > 0 {
            self.depth += 1;
            self.section_num[self.depth] = 0;
        }
    }
    pub fn delete(&mut self) {
        if self.sec_length[self.depth] > 0 {
            self.sec_length[self.depth] -= 1;
        }
        if self.section_num[self.depth] == self.sec_length[self.depth] {
            self.up();
        }
        if self.sec_length[self.depth] == 0 {
            self.left();
        }
    }
    /// return true if current selected item is a resource
    pub fn is_resource(&self) -> bool {
        if self.depth != 2 {
            false
        } else {
            let mut sec_sub = self.sec_key[0].clone();
            sec_sub.push_str(&self.sec_key[1]);
            self.resource_sections.contains(&sec_sub)
        }
    }
}

pub fn update_screen(position: &mut Position, plist: &Value, term: &Term) {
    display_footer(term); // draw fooret first so it can be overwritten if needed

    write!(&*term, "\x1B[3H").unwrap();
    let rows = term.size().0 as i32;
    let mut row = 4;
    let list = plist.as_dictionary().unwrap();
    let keys: Vec<String> = list.keys().map(|s| s.to_string()).collect();
    for (i, k) in keys.iter().enumerate() {
        if row < rows {
            row += display_value(k, None, position, list.get(k).unwrap(), &term, i, 0).unwrap();
        }
    }

    let mut blanks = rows - row;
    if blanks < 0 {
        blanks = 0;
    }
    blanks += 1;

    write!(&*term, "{}", "\x1B[0K\r\n".repeat(blanks as usize)).unwrap();

    display_header(position, term); // draw header last so selected res is known
    write!(&*term, "\x1B8").unwrap();
}

fn display_footer(term: &Term) {
    write!(
        &*term,
        "\x1B[{}H {} save {} quit {} Go build EFI",
        term.size().0,
        style('s').reverse(),
        style('q').reverse(),
        style('G').reverse()
    )
    .unwrap();
    write!(
        &*term,
        "    {}{}boolean {}data {}integer {}string\x1B[0K",
        style(' ').green().reverse(),
        style(' ').red().reverse(),
        style(' ').magenta().reverse(),
        style(' ').blue().reverse(),
        style(' ').reverse(),
    )
    .unwrap();
}

fn display_header(position: &mut Position, term: &Term) {
    let mut tmp = String::new();
    let mut info = position.sec_key[position.depth].clone();
    if info.len() > 20 {
        info = info[0..17].to_string();
        info.push_str("...");
    }
    write!(
        &*term,
        "\x1B[H\x1B[0K{}   \x1B[7mi\x1B[0m {} info if available\r\n\x1B[0K  {}",
        style(&position.file_name).green(),
        style(&info).underlined(),
        match position.item_clone {
            Value::Array(_) | Value::Dictionary(_) => {
                tmp.push_str("\x1B[7mright\x1B[0m");
                tmp.push_str(" to expand");
                &tmp
            }
            Value::Integer(_) => "\x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m to edit",
            Value::String(_) => "\x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m to edit",
            Value::Boolean(_) => "\x1B[7mspace\x1B[0m to toggle",
            Value::Data(_) =>
                "\x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m to edit,  \x1B[7mtab\x1B[0m to switch hex/string",
            _ => "XXXunknownXXX",
        }
    )
    .unwrap();
    if position.depth == 2 {
        let mut sec = position.sec_key[0].clone();
        sec.push_str(&position.sec_key[1]);
        if position.resource_sections.contains(&sec) {
            write!(&*term, " \x1B[7mspace\x1B[0m to toggle").unwrap();
        }
    }
    if position.depth > 0 {
        write!(&*term, "  {}", "\x1B[7mleft\x1B[0m to collapse").unwrap();
    }
}

fn display_value(
    key: &String,
    key_color: Option<bool>,
    position: &mut Position,
    plist_value: &Value,
    term: &Term,
    item_num: usize,
    d: usize,
) -> Result<i32, Box<dyn Error>> {
    let mut live_item = false;
    let mut save_curs_pos = String::new();
    let mut key_style = String::new();
    let mut pre_key = '>';
    let mut row = 1;
    write!(&*term, "\x1B[0K\n\r{}", "    ".repeat(d))?;
    if position.section_num[d] == item_num {
        position.sec_key[d] = key.to_string();
        //        position.sec_key[d] = String::from_utf8(strip_ansi_escapes::strip(&key)?)?;
        //            key.to_string();
        key_style.push_str("\x1B[7m");
        // current live item
        if d == position.depth {
            live_item = true;
            position.item_clone = plist_value.clone();
            save_curs_pos = "\x1B7".to_string(); // save cursor position
        }
    }
    match plist_value {
        Value::Array(v) => {
            if live_item {
                position.sec_length[d + 1] = v.len();
            }
            if position.depth > d && position.section_num[d] == item_num {
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
            if position.depth > d && position.section_num[d] == item_num {
                let mut key = String::new();
                for i in 0..v.len() {
                    let color = get_array_key(&mut key, &v[i], i);
                    row += display_value(&key, color, position, &v[i], term, i, d + 1)?;
                }
            }
        }
        Value::Boolean(v) => {
            match v {
                true => write!(
                    &*term,
                    "{}{}: {}{}",
                    key_style,
                    style(key).green(),
                    save_curs_pos,
                    v
                )
                .unwrap(),
                false => write!(
                    &*term,
                    "{}{}: {}{}",
                    key_style,
                    style(key).red(),
                    save_curs_pos,
                    v
                )
                .unwrap(),
            };
        }
        Value::Data(v) => {
            write!(
                &*term,
                "{}{}: <{}{}> | {}{}{}\x1B[0K",
                key_style,
                style(key).magenta(),
                save_curs_pos,
                hex_str_with_style(hex::encode(&*v)),
                style('\"').magenta(),
                get_lossy_string(v),
                style('\"').magenta()
            )?;
        }
        Value::Dictionary(v) => {
            if live_item {
                position.sec_length[d + 1] = v.keys().len();
            }
            if position.depth > d && position.section_num[d] == item_num {
                pre_key = 'v';
            }
            write!(
                &*term,
                "{} {}{}\x1B[0m  [{}]{} ",
                pre_key,
                key_style,
                match key_color {
                    Some(true) => style(key).green(),
                    Some(false) => style(key).red(),
                    None => style(key).white(),
                },
                v.len(),
                save_curs_pos
            )
            .unwrap();
            if position.depth > d && position.section_num[d] == item_num {
                let keys: Vec<String> = v.keys().map(|s| s.to_string()).collect();
                for (i, k) in keys.iter().enumerate() {
                    row += display_value(&k, None, position, v.get(&k).unwrap(), term, i, d + 1)?;
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
    Ok(row)
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

fn get_array_key(key: &mut String, v: &plist::Value, i: usize) -> Option<bool> {
    match v {
        Value::Dictionary(d) => {
            for k in ["Path", "BundlePath", "Name", "Comment"] {
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
                        return Some(true);
                    } else {
                        return Some(false);
                    }
                }
                _ => (),
            }
        }
        _ => *key = i.to_string(),
    }
    None
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
