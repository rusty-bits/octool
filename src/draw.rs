use plist::Value;
use std::collections::HashMap;
use termion::{color, terminal_size};
use termion::{raw::RawTerminal, style};

use std::error::Error;
use std::io::{Stdout, Write};

use crate::res::{self, Resources};

#[derive(Debug)]
pub struct Settings {
    pub config_file_name: String,       // name of config.plist
    pub sec_num: [usize; 5],            // selected section for each depth
    pub depth: usize,                   // depth of plist section we are looking at
    pub sec_key: [String; 5],           // key of selected section
    pub item_instructions: String,      // item instructions for display in header
    pub held_item: Option<Value>,       // last deleted or placed item value
    pub held_key: String,               // last deleted or placed key
    pub sec_length: [usize; 5],         // number of items in current section
    pub resource_sections: Vec<String>, // concat name of sections that contain resources
    pub build_type: String,             // building release or debug version
    pub oc_build_version: String,       // version of OpenCorePkg to use
    pub oc_build_date: String,
    pub oc_build_version_res_index: usize, // index of OpenCorePkg we are using
    pub resource_ver_indexes: HashMap<String, usize>,
    pub can_expand: bool,    // true if highlighted field can have children
    pub find_string: String, // last entered search string
    pub modified: bool,      // true if plist changed and not saved
}

impl Settings {
    pub fn up(&mut self) {
        if self.sec_num[self.depth] > 0 {
            self.sec_num[self.depth] -= 1;
        }
    }
    pub fn down(&mut self) {
        if self.sec_length[self.depth] > 0 {
            if self.sec_num[self.depth] < self.sec_length[self.depth] - 1 {
                self.sec_num[self.depth] += 1;
            }
        }
    }
    pub fn left(&mut self) {
        if self.depth > 0 {
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
    settings: &mut Settings,
    resources: &Resources,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    let plist = &resources.config_plist;
    let rows: i32 = terminal_size().unwrap().1.into();
    settings.can_expand = false;

    write!(stdout, "\x1B[{}H", rows - 1)?; // show footer first, in case we need to write over it
    write!(
        stdout,
        " {inv}x{res}cut {inv}c{res}opy {inv}p{res}aste   {inv}f{res}ind {inv}n{res}ext   {inv}a{res}dd {inv}d{res}el  {inv}m{res}erge {inv}r{res}eset   {inv}K{res}ey\x1B[0K\r\n {inv}s{res}ave {inv}q{res}uit   {inv}G{res}o build EFI  {inv}{red} {grn} {res}boolean {inv}{mag} {res}data {inv}{blu} {res}integer {inv} {res}string\x1B[0K",
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
            row += display_value(
                k,
                None,
                settings,
                resources,
                list.get(k).unwrap(),
                stdout,
                i,
                0,
            )
            .unwrap();
        }
    }
    #[cfg(debug_assertions)]
    write!(
        stdout,
        "debug-> {:?} {} {:?}",
        settings.sec_length, settings.depth, settings.sec_num
    )?;

    let mut blanks = rows - row - 1;
    if blanks < 0 {
        blanks = 0;
    }

    write!(stdout, "{}", "\r\n\x1B[0K".repeat(blanks as usize))?; // clear rows up to footer
                                                                  // lastly draw the header
    let mut info = String::new();
    settings.res_name(&mut info);
    if info.len() > 20 {
        info = info[0..17].to_string();
        info.push_str("...");
    }
    write!(
        stdout,
        "\x1b[1;{}H\x1b[2Kv{}",
        (terminal_size().unwrap().0 - settings.oc_build_version.len() as u16).to_string(),
        settings.oc_build_version,
    )
    .unwrap();
    write!(
        stdout,
        "\x1B[H{}{}   \x1B[0;7mi\x1B[0mnfo for {}{}{} if available\r\n\x1B[0K",
        color::Fg(color::Green),
        &settings.config_file_name,
        style::Underline,
        &info,
        style::Reset,
    )
    .unwrap();
    if settings.depth > 0 {
        write!(stdout, "  \x1B[7mleft\x1B[0m collapse").unwrap();
    }
    write!(stdout, "{}", settings.item_instructions,).unwrap();
    if settings.depth == 2 {
        if settings.is_resource() {
            write!(stdout, "  \x1B[7mspace\x1B[0m toggle").unwrap();
        }
    }
    if settings.find_string.len() > 0 {
        write!(
            stdout,
            "  \x1B[7mn\x1B[0m jump to next {}{}\x1B[0m",
            style::Underline,
            settings.find_string
        )
        .unwrap();
    }
    if settings.held_key.len() > 0 {
        write!(
            stdout,
            "  \x1B[7mp\x1B[0m paste {}{}\x1B[0m",
            style::Underline,
            settings.held_key
        )
        .unwrap();
    }
    write!(stdout, "\r\n\x1B[2K\x1B8",).unwrap();
    Ok(())
}

fn display_value(
    key: &String,
    key_color: Option<bool>,
    settings: &mut Settings,
    resources: &Resources,
    plist_value: &Value,
    stdout: &mut RawTerminal<Stdout>,
    item_num: usize,
    display_depth: usize,
) -> Result<i32, Box<dyn Error>> {
    let mut live_item = false;
    let mut selected_item = false;
    let mut save_curs_pos = String::new();
    let mut key_style = String::new();
    let mut pre_key = '>';
    let mut row = 1;
    write!(stdout, "\r\n{}\x1B[0K", "    ".repeat(display_depth))?; // indent to section and clear rest of line
    if settings.sec_num[display_depth] == item_num {
        selected_item = true;
        settings.sec_key[display_depth] = key.to_string();
        key_style.push_str("\x1B[7m");
        // is current live item
        if display_depth == settings.depth {
            live_item = true;
            settings.item_instructions = match plist_value {
            Value::Array(_) | Value::Dictionary(_) => "  \x1B[7mright\x1B[0m expand",
            Value::Integer(_) | Value::String(_) => "  \x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m edit",
            Value::Boolean(_) => "  \x1B[7mspace\x1B[0m toggle",
            Value::Data(_) =>
                "  \x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m edit  \x1B[7mtab\x1B[0m switch hex/string",
            _ => "  XXXunknownXXX",
        }.to_string();
            save_curs_pos = "\x1B7".to_string(); // save cursor position for editing and info display
        }
    }
    match plist_value {
        Value::Array(v) => {
            if selected_item {
                settings.sec_length[display_depth + 1] = v.len();
            }
            if live_item {
                settings.can_expand = true;
            }
            if settings.depth > display_depth && settings.sec_num[display_depth] == item_num {
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
            if settings.depth > display_depth && settings.sec_num[display_depth] == item_num {
                if v.len() == 0 {
                    write!(
                        stdout,
                        "\r\n\x1B[0K{}\x1B[7mempty\x1B[0m{}",
                        "    ".repeat(display_depth + 1),
                        save_curs_pos
                    )
                    .unwrap();
                    row += 1;
                } else {
                    let mut key = String::new();
                    for i in 0..v.len() {
                        let color = get_array_key(&mut key, &v[i], i);
                        row += display_value(
                            &key,
                            color,
                            settings,
                            resources,
                            &v[i],
                            stdout,
                            i,
                            display_depth + 1,
                        )?;
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
                settings.sec_length[display_depth + 1] = v.keys().len();
            }
            if live_item {
                settings.can_expand = true;
            }
            if settings.depth > display_depth && settings.sec_num[display_depth] == item_num {
                pre_key = 'v';
            }
            write!(
                stdout,
                "{} {}{}{}\x1B[0m {} [{}]{} ",
                pre_key,
                key_style,
                match key_color {
                    Some(true) => color::Fg(color::Green).to_string(),
                    Some(false) => color::Fg(color::Red).to_string(),
                    None => color::Fg(color::Reset).to_string(),
                },
                key,
                //                settings.resource_ver_indexes.get(key.as_str()),
                res::res_version(settings, &resources, &key),
/*                if let Some(parent_res) = resources.resource_list[key]["parent"].as_str() {
                    match settings.resource_ver_indexes.get(parent_res) {
                        Some(p_index) => {
                            if let Some(v) =
                                resources.dortania[parent_res]["versions"][p_index]["version"].as_str()
                            {
                                ver = v.to_owned();
                            } else {
                                ver = "".to_owned();
                            }
                        }
                        None => {
                            let mut p_index = 0;
                            loop {
                                if let Some(date) = resources.dortania[parent_res]["versions"][p_index]
                                    ["date_built"]
                                    .as_str()
                                {
                                    if date[..11] <= settings.oc_build_date[..11] {
                                        settings.resource_ver_indexes.insert(parent_res.to_owned(), p_index);
                                        if let Some(s) = resources.dortania[parent_res]["versions"][p_index]
                                            ["version"]
                                            .as_str()
                                        {
                                            ver = s.to_owned();
                                        } else {
                                            ver = "".to_owned();
                                        }
                                        break;
                                    } else {
                                        p_index += 1;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    &ver
                } else {
                    ""
                },*/
                v.len(),
                save_curs_pos
            )
            .unwrap();
            if settings.depth > display_depth && settings.sec_num[display_depth] == item_num {
                if v.keys().len() == 0 {
                    write!(
                        stdout,
                        "\r\n\x1B[0K{}\x1B[7mempty\x1B[0m{}",
                        "    ".repeat(display_depth + 1),
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
                            settings,
                            resources,
                            v.get(&k).unwrap(),
                            stdout,
                            i,
                            display_depth + 1,
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
