use plist::Value;
use termion::{color, cursor, terminal_size};
use termion::{raw::RawTerminal, style};

use std::error::Error;
use std::io::{Stdout, Write};

//use crate::res::has_parent;

#[derive(Debug)]
pub struct Position<'a> {
    pub file_name: String,              // name of config.plist
    pub section_num: [usize; 5],        // selected section for each depth
    pub depth: usize,                   // depth of plist we are looking at
    pub sec_key: [String; 5],           // key of selected section
    pub item_clone: Value,              // copy of highlighted item (can we get rid of this?)
    pub sec_length: [usize; 5],         // number of items in current section
    pub resource_sections: Vec<String>, // concat name of sections that contain resources
    pub build_type: String,             // building release or debug version
    pub parents: &'a serde_json::Value,
}

impl<'a> Position<'a> {
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
    /*    pub fn has_parent(&self) -> bool {
        let mut r = String::new();
        self.res_name(&mut r);
        match self.parents[r]["parent"].as_str() {
            Some(_) => true,
            None => false,
        }
    } */
    pub fn parent(&self) -> Option<&str> {
        let mut r = String::new();
        self.res_name(&mut r);
        self.parents[r]["parent"].as_str()
    }
    pub fn res_name(&self, name: &mut String) {
        if self.sec_key[0] == "UEFI" && self.sec_key[1] == "Drivers" && self.depth == 2 {
            *name = self.item_clone.as_string().unwrap_or("").to_owned();
        } else {
            *name = self.sec_key[self.depth]
                .to_owned()
                .split('/')
                .last()
                .unwrap()
                .to_string();
        }
        if name.starts_with('#') {
            name.remove(0);
        }
    }
}

pub fn update_screen(
    position: &mut Position,
    plist: &Value,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    let rows: i32 = terminal_size().unwrap().1.into();
    write!(stdout, "\x1B[{}H", rows)?;
    write!(
        stdout,
        " {inv}s{res} save {inv}q{res} quit {inv}G{res} Go build EFI  {inv}{red} {grn} {res}boolean {inv}{mag} {res}data {inv}{blu} {res}integer {inv} {res}string",
        inv = style::Invert,
        res = style::Reset,
        grn = color::Fg(color::Green),
        red = color::Fg(color::Red),
        mag = color::Fg(color::Magenta),
        blu = color::Fg(color::Blue),
    )?;

    write!(stdout, "\x1B[3H")?;
    let mut row = 4;
    let list = plist.as_dictionary().unwrap();
    let keys: Vec<String> = list.keys().map(|s| s.to_string()).collect();
    for (i, k) in keys.iter().enumerate() {
        if row < rows {
            row += display_value(k, None, position, list.get(k).unwrap(), stdout, i, 0).unwrap();
        }
    }

    let mut blanks = rows - row;
    if blanks < 0 {
        blanks = 0;
    }

    write!(stdout, "{}", "\r\n\x1B[0K".repeat(blanks as usize))?;

    let mut tmp = String::new();
    let mut info = String::new();
    position.res_name(&mut info);
    if info.len() > 20 {
        info = info[0..17].to_string();
        info.push_str("...");
    }
    write!(
        stdout,
        "\x1B[H\x1B[0K{}{}   \x1B[0;7mi\x1B[0m {}{}{} info if available\r\n\x1B[0K  {}",
        color::Fg(color::Green),
        &position.file_name,
        style::Underline,
        &info,
        style::Reset,
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
        if position.is_resource() {
            write!(stdout, " \x1B[7mspace\x1B[0m to toggle").unwrap();
        }
    }
    if position.depth > 0 {
        write!(stdout, "  {}", "\x1B[7mleft\x1B[0m to collapse").unwrap();
    }
    write!(stdout, "\x1B8").unwrap();
    Ok(())
}

fn display_value(
    key: &String,
    key_color: Option<bool>,
    position: &mut Position,
    plist_value: &Value,
    stdout: &mut RawTerminal<Stdout>,
    item_num: usize,
    d: usize,
) -> Result<i32, Box<dyn Error>> {
    let mut live_item = false;
    let mut save_curs_pos = String::new();
    let mut key_style = String::new();
    let mut pre_key = '>';
    let mut row = 1;
    write!(stdout, "\x1B[0K\r\n{}", "    ".repeat(d))?;
    if position.section_num[d] == item_num {
        position.sec_key[d] = key.to_string();
        key_style.push_str("\x1B[7m");
        // is current live item
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
            write!(stdout, "{} ", pre_key)?;
            write!(
                stdout,
                "{}{}\x1B[0m  [{}]{} ",
                key_style,
                key,
                v.len(),
                cursor::Save
            )?;
            if position.depth > d && position.section_num[d] == item_num {
                let mut key = String::new();
                for i in 0..v.len() {
                    let color = get_array_key(&mut key, &v[i], i);
                    row += display_value(&key, color, position, &v[i], stdout, i, d + 1)?;
                }
            }
        }
        Value::Boolean(v) => {
            match v {
                true => write!(
                    stdout,
                    "{}{}{}{}: {}{}",
                    key_style,
                    color::Fg(color::Green),
                    key,
                    style::Reset,
                    save_curs_pos,
                    v
                )
                .unwrap(),
                false => write!(
                    stdout,
                    "{}{}{}{}: {}{}",
                    key_style,
                    color::Fg(color::Red),
                    key,
                    style::Reset,
                    save_curs_pos,
                    v
                )
                .unwrap(),
            };
        }
        Value::Data(v) => {
            write!(
                stdout,
                "{}{}{}{}: <{}{}> | {}{}{}\x1B[0K",
                key_style,
                color::Fg(color::Magenta),
                key,
                style::Reset,
                save_curs_pos,
                hex_str_with_style(hex::encode(&*v)),
                '\"',
                get_lossy_string(v),
                '\"'
            )?;
        }
        Value::Dictionary(v) => {
            if live_item {
                position.sec_length[d + 1] = v.keys().len();
            }
            if position.depth > d && position.section_num[d] == item_num {
                pre_key = 'v';
            }
            write!(stdout, "{} ", pre_key)?;
            /*            if d == 2 && position.is_resource() {
                if position.has_parent() {
                    write!(&*term, "{} ", style("•").green())?;
                } else {
                    write!(&*term, "{} ", style("•").red())?;
                }
            }; */
            write!(
                stdout,
                "{}{}{}\x1B[0m  [{}]{} ",
                key_style,
                match key_color {
                    Some(true) => color::Fg(color::Green).to_string(),
                    Some(false) => color::Fg(color::Red).to_string(),
                    None => color::Fg(color::Reset).to_string(),
                },
                key,
                v.len(),
                save_curs_pos
            )
            .unwrap();
            if position.depth > d && position.section_num[d] == item_num {
                let keys: Vec<String> = v.keys().map(|s| s.to_string()).collect();
                for (i, k) in keys.iter().enumerate() {
                    row += display_value(&k, None, position, v.get(&k).unwrap(), stdout, i, d + 1)?;
                }
            }
        }
        Value::Integer(v) => {
            write!(
                stdout,
                "{}{}{}{}: {}{}",
                key_style,
                color::Fg(color::Blue),
                key,
                style::Reset,
                save_curs_pos,
                v
            )?;
        }
        Value::String(v) => {
            write!(stdout, "{}{:>2}\x1B[0m: ", key_style, key)?;
            /*            if position.is_resource() {
                if position.has_parent(&v) {
                    write!(&*term, "{}", style("•").green())?;
                } else {
                    write!(&*term, "{}", style("•").red())?;
                }
            } */
            write!(stdout, "{}{}", save_curs_pos, v)?;
        }
        _ => panic!("Can't handle this type"),
    }
    Ok(row)
}

pub fn get_lossy_string(v: &Vec<u8>) -> String {
    let mut tmp = String::new();
    for c in v {
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
            hex_u.push_str(&color::Fg(color::Magenta).to_string());
            hex_u.push(c);
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
