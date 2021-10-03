use plist::Value;
use termion::{color, terminal_size};
use termion::{raw::RawTerminal, style};

use std::error::Error;
use std::io::{Stdout, Write};

#[derive(Debug)]
pub struct Position {
    pub config_file_name: String,       // name of config.plist
    pub sec_num: [usize; 5],            // selected section for each depth
    pub depth: usize,                   // depth of plist section we are looking at
    pub sec_key: [String; 5],           // key of selected section
    pub item_clone: Value,              // copy of highlighted item (can we get rid of this?)
    pub held_item: Option<Value>,       // last deleted or placed item value
    pub held_key: String,               // last deleted or placed key
    pub sec_length: [usize; 5],         // number of items in current section
    pub resource_sections: Vec<String>, // concat name of sections that contain resources
    pub build_type: String,             // building release or debug version
    pub can_expand: bool,               // true if highlighted field can have children
    pub find_string: String,            // last entered search string
    pub modified: bool,                 // true if plist changed and not saved
}

impl Position {
    pub fn up(&mut self) {
        if self.sec_num[self.depth] > 0 {
            self.sec_num[self.depth] -= 1;
            //            self.sec_length[self.depth + 1] = 0;
        }
    }
    pub fn down(&mut self) {
        if self.sec_length[self.depth] > 0 {
            if self.sec_num[self.depth] < self.sec_length[self.depth] - 1 {
                self.sec_num[self.depth] += 1;
                //                self.sec_length[self.depth + 1] = 0;
            }
        }
    }
    pub fn left(&mut self) {
        if self.depth > 0 {
            //            self.sec_length[self.depth + 1] = 0;
            self.sec_key[self.depth].clear();
            self.depth -= 1;
        }
    }
    pub fn right(&mut self) {
        if self.depth < 3 && self.can_expand {
            self.depth += 1;
            self.sec_num[self.depth] = 0;
        }
    }
    pub fn add(&mut self) {
        self.sec_length[self.depth] += 1;
        self.modified = true;
    }
    pub fn delete(&mut self) {
        if self.sec_length[self.depth] > 0 {
            self.sec_length[self.depth] -= 1;
        }
        if self.sec_num[self.depth] == self.sec_length[self.depth] {
            self.up();
        }
        self.modified = true;
        //        if self.sec_length[self.depth] == 0 {
        //            self.left();
        //        }
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
    pub fn res_name(&self, name: &mut String) {
        *name = self.sec_key[self.depth]
            .to_owned()
            .split('/')
            .last()
            .unwrap()
            .to_string();
    }
}

/// Redraws the plist on the screen
/// Draws the Footer first, in case it needs to be overwritten
/// Draws the plist next with current selection expanded
/// This allows the currently highlighted item info to be obtained
/// so any special comments can be included in the Header
/// which is drawn last
pub fn update_screen(
    position: &mut Position,
    plist: &Value,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    let rows: i32 = terminal_size().unwrap().1.into();
    position.can_expand = false;

    write!(stdout, "\x1B[{}H", rows - 1)?; // show footer first, in case we need to write over it
    write!(
        stdout,
        " {inv}x{res}cut {inv}c{res}opy {inv}p{res}aste   {inv}f{res}ind {inv}n{res}ext   {inv}a{res}dd {inv}d{res}el  {inv}m{res}erge {inv}r{res}eset\x1B[0K\r\n {inv}s{res}ave {inv}q{res}uit   {inv}G{res}o build EFI  {inv}{red} {grn} {res}boolean {inv}{mag} {res}data {inv}{blu} {res}integer {inv} {res}string\x1B[0K",
        inv = style::Invert,
        res = style::Reset,
        grn = color::Fg(color::Green),
        red = color::Fg(color::Red),
        mag = color::Fg(color::Magenta),
        blu = color::Fg(color::Blue),
    )?;

    write!(stdout, "\x1B[3H")?; // jump under header
    let mut row = 4;
    let list = plist.as_dictionary().unwrap();
    let keys: Vec<String> = list.keys().map(|s| s.to_string()).collect();
    for (i, k) in keys.iter().enumerate() {
        if row < rows {
            row += display_value(k, None, position, list.get(k).unwrap(), stdout, i, 0).unwrap();
        }
    }
    #[cfg(debug_assertions)]
    write!(
        stdout,
        "{:?} {} {:?}",
        position.sec_length, position.depth, position.sec_num
    )?;

    let mut blanks = rows - row - 1;
    if blanks < 0 {
        blanks = 0;
    }

    write!(stdout, "{}", "\r\n\x1B[0K".repeat(blanks as usize))?; // clear rows up to footer
                                                                  // lastly draw the header
    let mut info = String::new();
    position.res_name(&mut info);
    if info.len() > 20 {
        info = info[0..17].to_string();
        info.push_str("...");
    }
    write!(
        stdout,
        "\x1B[H\x1B[0K{}{}   \x1B[0;7mi\x1B[0mnfo for {}{}{} if available\r\n\x1B[0K",
        color::Fg(color::Green),
        &position.config_file_name,
        style::Underline,
        &info,
        style::Reset,
    )
    .unwrap();
    if position.depth > 0 {
        write!(stdout, "  \x1B[7mleft\x1B[0m collapse").unwrap();
    }
    write!(
        stdout,
        "{}",
        match position.item_clone {
            Value::Array(_) | Value::Dictionary(_) => "  \x1B[7mright\x1B[0m expand",
            Value::Integer(_) | Value::String(_) => "  \x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m edit",
            Value::Boolean(_) => "  \x1B[7mspace\x1B[0m toggle",
            Value::Data(_) =>
                "  \x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m edit  \x1B[7mtab\x1B[0m switch hex/string",
            _ => "  XXXunknownXXX",
        }
    )
    .unwrap();
    if position.depth == 2 {
        if position.is_resource() {
            write!(stdout, "  \x1B[7mspace\x1B[0m toggle").unwrap();
        }
    }
    //    if position.is_resource() {
    //        write!(stdout, "  \x1B[7ma\x1B[0m add resource").unwrap();
    //    }
    if position.find_string.len() > 0 {
        write!(
            stdout,
            "  \x1B[7mn\x1B[0m jump to next {}{}\x1B[0m",
            style::Underline,
            position.find_string
        )
        .unwrap();
    }
    if position.held_key.len() > 0 {
        write!(
            stdout,
            "  \x1B[7mp\x1B[0m paste {}{}\x1B[0m",
            style::Underline,
            position.held_key
        )
        .unwrap();
    }
    write!(stdout, "\r\n\x1B[2K\x1B8",).unwrap();
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
    let mut selected_item = false;
    let mut save_curs_pos = String::new();
    let mut key_style = String::new();
    let mut pre_key = '>';
    let mut row = 1;
    write!(stdout, "\r\n{}\x1B[0K", "    ".repeat(d))?; // indent to section and clear rest of line
    if position.sec_num[d] == item_num {
        selected_item = true;
        position.sec_key[d] = key.to_string();
        key_style.push_str("\x1B[7m");
        // is current live item
        if d == position.depth {
            live_item = true;
            position.item_clone = plist_value.clone();
            save_curs_pos = "\x1B7".to_string(); // save cursor position for editing and info display
        }
    }
    match plist_value {
        Value::Array(v) => {
            if selected_item {
                position.sec_length[d + 1] = v.len();
            }
            if live_item {
                position.can_expand = true;
            }
            if position.depth > d && position.sec_num[d] == item_num {
                pre_key = 'v';
            }
            write!(
                stdout,
                "{} {}{}\x1B[0m  [{}]{} ",
                pre_key,
                key_style,
                key,
                v.len(),
                save_curs_pos
            )?;
            if position.depth > d && position.sec_num[d] == item_num {
                if v.len() == 0 {
                    write!(
                        stdout,
                        "\r\n\x1B[0K{}\x1B[7mempty\x1B[0m{}",
                        "    ".repeat(d + 1),
                        save_curs_pos
                    )
                    .unwrap();
                    row += 1;
                } else {
                    let mut key = String::new();
                    for i in 0..v.len() {
                        let color = get_array_key(&mut key, &v[i], i);
                        row += display_value(&key, color, position, &v[i], stdout, i, d + 1)?;
                    }
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
            if selected_item {
                position.sec_length[d + 1] = v.keys().len();
            }
            if live_item {
                position.can_expand = true;
            }
            if position.depth > d && position.sec_num[d] == item_num {
                pre_key = 'v';
            }
            write!(
                stdout,
                "{} {}{}{}\x1B[0m  [{}]{} ",
                pre_key,
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
            if position.depth > d && position.sec_num[d] == item_num {
                if v.keys().len() == 0 {
                    write!(
                        stdout,
                        "\r\n\x1B[0K{}\x1B[7mempty\x1B[0m{}",
                        "    ".repeat(d + 1),
                        save_curs_pos
                    )
                    .unwrap();
                    row += 1;
                } else {
                    let keys: Vec<String> = v.keys().map(|s| s.to_string()).collect();
                    for (i, k) in keys.iter().enumerate() {
                        row += display_value(
                            &k,
                            None,
                            position,
                            v.get(&k).unwrap(),
                            stdout,
                            i,
                            d + 1,
                        )?;
                    }
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
            write!(
                stdout,
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
            hex_u.push_str(&style::Reset.to_string());
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
