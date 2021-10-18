use crate::draw::{self, Settings};
use crate::res::Resources;
use plist::{Integer, Value};
use termion::event::Key;
use termion::{color, cursor};
use termion::{input::TermRead, raw::RawTerminal, style};

use std::{
    error::Error,
    i64,
    io::{Stdout, Write},
};

#[derive(Debug)]
pub struct Found {
    pub level: usize,
    pub section: [usize; 5],
    pub keys: Vec<String>,
}

impl Found {
    pub fn new() -> Found {
        Found {
            level: 0,
            section: [0; 5],
            keys: vec![],
        }
    }
}

/// find the current highlighted item in the given 'plist_val' plist
/// and place it into the settings. held_item and held_key
/// if 'first' is true get the first key of a dict or item 0 of an array
/// if 'preserve' is true do not clear the held values to their defaults
pub fn extract_value(
    settings: &mut Settings,
    mut plist_val: &Value,
    first: bool,
    preserve: bool,
) -> bool {
    let mut extracted = true;
    for i in 0..settings.depth {
        match plist_val {
            Value::Dictionary(d) => {
                plist_val = match d.get(&settings.sec_key[i]) {
                    Some(k) => k,
                    None => return false,
                }
            }
            Value::Array(a) => {
                plist_val = a.get(0).expect("No 0 element in Sample.plist");
            }
            _ => (),
        }
    }
    //    write!(std::io::stdout(), "{:?}\r\n", plist_val).unwrap();
    match plist_val {
        Value::Dictionary(d) => {
            let key = if first {
                d.keys().next().expect("Didn't get first key").clone()
            } else {
                settings.sec_key[settings.depth].clone()
            };
            match d.get(&key) {
                Some(v) => {
                    settings.held_item = Some(v.to_owned());
                    settings.held_key = key.to_owned();
                }
                None => extracted = false,
            };
        }
        Value::Array(a) => {
            let num = if first {
                0
            } else {
                settings.sec_num[settings.depth]
            };
            if num < a.len() {
                let mut ex_item = a.get(num).unwrap().to_owned();
                match ex_item.as_dictionary_mut() {
                    Some(d) => {
                        if !preserve {
                            for val in d.values_mut() {
                                match val {
                                    Value::String(_) => *val = Value::String("".to_string()),
                                    Value::Boolean(_) => *val = Value::Boolean(false),
                                    _ => (),
                                }
                            }
                        }
                        settings.held_item = Some(ex_item);
                        settings.held_key = settings.sec_key[settings.depth].clone();
                    }
                    None => extracted = false,
                }
            } else {
                extracted = false;
            }
        }
        _ => extracted = false,
    }
    extracted
}

/// if 'add' is true,
/// place the settings.held_item into the given 'plist_val' plist at the highlighted location
/// if 'add' is false
/// delete the highlighted value from the given 'plist_val' plist and place it in the settings.held_item
pub fn add_delete_value(settings: &mut Settings, mut plist_val: &mut Value, add: bool) -> bool {
    let mut changed = false;
    for i in 0..settings.depth {
        match plist_val {
            Value::Dictionary(d) => {
                plist_val = match d.get_mut(&settings.sec_key[i]) {
                    Some(k) => k,
                    None => return false,
                }
            }
            Value::Array(a) => {
                plist_val = a.get_mut(settings.sec_num[i]).unwrap();
            }
            _ => (),
        }
    }
    match plist_val {
        Value::Dictionary(d) => {
            if add {
                match settings.held_item.clone() {
                    Some(item) => {
                        if d.insert(settings.held_key.to_owned(), item) == None {
                            changed = true;
                        }
                    }
                    None => (),
                }
            } else {
                settings.held_item = Some(d.remove(&settings.sec_key[settings.depth]).unwrap());
                settings.held_key = settings.sec_key[settings.depth].to_owned();
                changed = true;
            }
            d.sort_keys();
            if changed {
                settings.sec_num[settings.depth] =
                    d.keys().position(|k| k == &settings.held_key).unwrap_or(0);
            };
        }
        Value::Array(a) => {
            if add {
                match settings.held_item.clone() {
                    Some(item) => {
                        a.insert(settings.sec_num[settings.depth], item);
                        changed = true;
                    }
                    None => (),
                }
            } else {
                settings.held_item = Some(a.remove(settings.sec_num[settings.depth]));
                settings.held_key = settings.sec_key[settings.depth].to_owned();
                changed = true;
            }
        }
        _ => (),
    }
    changed
}


/// ask for a search string and give a scrollable list of locations to jump to in 'found'
/// if only 1 result is found, jump immediately
pub fn find(
    settings: &mut Settings,
    resource: &plist::Value,
    found: &mut Vec<Found>,
    stdout: &mut RawTerminal<Stdout>,
) {
    settings.find_string = String::new();
    write!(
        stdout,
        "{}\r\x1B[2KEnter search term: {}\r\n\x1B[2K\x1B8",
        cursor::Show,
        cursor::Save
    )
    .unwrap();
    let empty_vec = vec![];
    edit_string(&mut settings.find_string, &empty_vec, stdout).unwrap();
    if settings.find_string.len() > 0 {
        let search = settings.find_string.to_lowercase();
        let resource = resource.as_dictionary().unwrap();
        for (i, key) in resource.keys().enumerate() {
            let low_key = key.to_lowercase();
            if low_key.contains(&search) {
                found.push(Found {
                    level: 0,
                    section: [i, 0, 0, 0, 0],
                    keys: vec![key.to_owned()],
                });
            }
            match resource.get(key).unwrap().as_dictionary() {
                Some(sub) => {
                    for (j, s_key) in sub.keys().enumerate() {
                        let low_key = s_key.to_lowercase();
                        if low_key.contains(&search) {
                            found.push(Found {
                                level: 1,
                                section: [i, j, 0, 0, 0],
                                keys: vec![key.to_owned(), s_key.to_owned()],
                            });
                        }
                        let sub_sub = sub.get(s_key).unwrap();
                        match sub_sub {
                            plist::Value::Dictionary(d) => {
                                for (k, s_s_key) in d.keys().enumerate() {
                                    let low_key = s_s_key.to_lowercase();
                                    if low_key.contains(&search) {
                                        found.push(Found {
                                            level: 2,
                                            section: [i, j, k, 0, 0],
                                            keys: vec![
                                                key.to_owned(),
                                                s_key.to_owned(),
                                                s_s_key.to_owned(),
                                            ],
                                        });
                                    }
                                    match d.get(s_s_key).unwrap() {
                                        plist::Value::Dictionary(sub_d) => {
                                            for (l, sub_d_key) in sub_d.keys().enumerate() {
                                                let low_key = sub_d_key.to_lowercase();
                                                if low_key.contains(&search) {
                                                    found.push(Found {
                                                        level: 3,
                                                        section: [i, j, k, l, 0],
                                                        keys: vec![
                                                            key.to_owned(),
                                                            s_key.to_owned(),
                                                            s_s_key.to_owned(),
                                                            sub_d_key.to_owned(),
                                                        ],
                                                    });
                                                }
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                            }
                            plist::Value::Array(a) => {
                                for (k, v) in a.iter().enumerate() {
                                    match v {
                                        plist::Value::Dictionary(d) => {
                                            for (l, s_s_key) in d.keys().enumerate() {
                                                let low_key = s_s_key.to_lowercase();
                                                if low_key.contains(&search) {
                                                    found.push(Found {
                                                        level: 3,
                                                        section: [i, j, k, l, 0],
                                                        keys: vec![
                                                            key.to_owned(),
                                                            s_key.to_owned(),
                                                            k.to_string(),
                                                            s_s_key.to_owned(),
                                                        ],
                                                    });
                                                }
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                            }

                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
    }
    write!(stdout, "{}", cursor::Hide).unwrap();
}

/// add an item of a user selected type to the loaded config.plist as the highlighted
/// location.  If the highlighted location is inside a section that holds resources
/// e.g. Kexts, Drivers, etc. then give an option to insert a blank template made from
/// the format in the corresponding Sample.plist
pub fn add_item(
    mut settings: &mut Settings,
    resources: &mut Resources,
    stdout: &mut RawTerminal<Stdout>,
) {
    settings.modified = true;
    let mut selection = 1;
    let mut item_types = Vec::<&str>::new();
    let new_res_msg = format!(
        "New {} {} template from Sample.plist",
        settings.sec_key[0], settings.sec_key[1]
    );
    if settings.is_resource() {
        item_types.push(&new_res_msg);
    }
    for s in [
        "plist array",
        "plist boolean",
        "plist data",
        "plist dict",
        "plist integer",
        "plist string",
    ] {
        item_types.push(s);
    }
    write!(
        stdout,
        "\r\nSelect type of item to add to plist:\x1B[0K\r\n{}",
        cursor::Save
    )
    .unwrap();
    loop {
        write!(stdout, "\x1B8").unwrap();
        for (i, item_type) in item_types.iter().enumerate() {
            if i == selection - 1 {
                write!(stdout, "\x1B[7m").unwrap();
            }
            write!(stdout, "{}\x1B[0m\x1B[0K\r\n", item_type).unwrap();
        }
        write!(stdout, "\x1B[2K").unwrap();
        stdout.flush().unwrap();
        match std::io::stdin().keys().next().unwrap().unwrap() {
            Key::Up => {
                if selection > 1 {
                    selection -= 1;
                }
            }
            Key::Down => {
                if selection < item_types.len() {
                    selection += 1;
                }
            }
            Key::Char('\n') => break,
            Key::Esc => {
                selection = 0;
                break;
            }
            _ => (),
        }
    }
    if selection == 0 {
        return;
    };
    if item_types[selection - 1] == &new_res_msg {
        if !extract_value(&mut settings, &resources.sample_plist, true, false) {
            return;
        }
    } else {
        write!(
            stdout,
            "Enter key for new {} item: {}{}\x1B[0K\r\n\x1B[2K",
            item_types[selection - 1],
            cursor::Save,
            cursor::Show
        )
        .unwrap();
        stdout.flush().unwrap();
        let mut key = String::new();
        let empty_vec = vec![];
        edit_string(&mut key, &empty_vec, stdout).unwrap();
        settings.held_key = String::from(key.trim());
        settings.held_item = Some(match item_types[selection - 1] {
            "plist array" => plist::Value::Array(vec![]),
            "plist boolean" => false.into(),
            "plist data" => plist::Value::Data(vec![]),
            "plist dict" => plist::Value::Dictionary(plist::Dictionary::default()),
            "plist integer" => 0.into(),
            "plist string" => plist::Value::String("".to_string()),
            _ => panic!("How did you select this?"),
        });
        write!(stdout, "{}", cursor::Hide).unwrap();
    }
    if add_delete_value(settings, &mut resources.config_plist, true) {
        settings.add();
    }
}

/// edit the highlighted value in the loaded config.plist
pub fn edit_value(
    settings: &mut Settings,
    mut val: &mut Value,
    valid_values: &Vec<String>,
    stdout: &mut RawTerminal<Stdout>,
    space_pressed: bool,
    edit_key: bool,
) -> Result<(), Box<dyn Error>> {
    write!(
        stdout,
        "{}\x1B[H\x1B[0K {inv}enter{res} save changes   {inv}esc{res} cancel changes",
        cursor::Show,
        inv = style::Invert,
        res = style::Reset,
    )?;
    let mut search_depth = settings.depth + 1;
    if edit_key {
        search_depth -= 1;
    }
    for i in 0..search_depth {
        match val {
            Value::Dictionary(d) => {
                let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                    [settings.sec_num[i]]
                    .clone();
                val = match d.get_mut(&key) {
                    Some(k) => k,
                    None => panic!("failure to get Value from Dict"),
                };
            }
            Value::Array(a) => {
                val = a.get_mut(settings.sec_num[i]).unwrap();
            }
            _ => (),
        }
    }

    if space_pressed {
        match val {
            Value::Boolean(b) => *b = !*b,
            Value::Dictionary(d) => match d.get_mut("Enabled") {
                Some(Value::Boolean(b)) => *b = !*b,
                _ => (),
            },
            _ => (),
        }
    } else if edit_key {
        match val {
            Value::Dictionary(d) => {
                let mut key = settings.sec_key[search_depth].to_owned();
                let hold = d.remove(&key);
                //        write!(stdout, "\r\n{:?}\r\n{:?}\r\n", hold, val)?;
                match hold {
                    Some(v) => {
                        write!(stdout, "\x1B8\r{}| \x1B7", "    ".repeat(settings.depth))?;
                        edit_string(&mut key, valid_values, stdout)?;
                        d.insert(key.clone(), v);
                        d.sort_keys();
                        settings.sec_num[settings.depth] =
                            d.keys().position(|k| k == &key).unwrap_or(0);
                    }
                    None => (),
                }
            }
            _ => (),
        }
    } else {
        match val {
            Value::Integer(i) => edit_int(i, valid_values, stdout),
            Value::String(s) => edit_string(s, valid_values, stdout)?,
            Value::Data(d) => edit_data(d, stdout)?,
            _ => (),
        }
    }

    write!(stdout, "{}", cursor::Hide)?;
    settings.modified = true;
    Ok(())
}

fn edit_data(val: &mut Vec<u8>, stdout: &mut RawTerminal<Stdout>) -> Result<(), Box<dyn Error>> {
    let mut edit_hex = hex::encode(val.clone());
    let mut pos = edit_hex.len();
    let mut hexedit = true;
    let mut keys = std::io::stdin().keys();
    loop {
        let mut tmp_val = edit_hex.clone();
        if tmp_val.len() % 2 == 1 {
            tmp_val.insert(0, '0');
        }
        let tmp_val = hex::decode(tmp_val).unwrap();
        write!(
            stdout,
            "\x1B8\x1B[G{mag}as hex{res}\x1B8{}\x1B[0K\x1B[E{mag}as string\x1B[0K\x1B8\x1B[B{}\x1B8",
            draw::hex_str_with_style(edit_hex.clone()),
            draw::get_lossy_string(&tmp_val),
            mag = color::Fg(color::Magenta),
            res = style::Reset,
        )?;
        if hexedit {
            write!(
                stdout,
                "\x1B[G{}{}as hex{}\x1B8{}",
                style::Invert,
                color::Fg(color::Magenta),
                style::Reset,
                "\x1B[C".repeat(pos)
            )?;
        } else {
            write!(
                stdout,
                "\x1B[E{}{}as string{}\x1B8\x1B[B{}",
                style::Invert,
                color::Fg(color::Magenta),
                style::Reset,
                "\x1B[C".repeat(pos / 2)
            )
            .unwrap();
        }
        stdout.flush()?;
        match keys.next().unwrap().unwrap() {
            Key::Char('\n') => {
                *val = tmp_val;
                break;
            }
            Key::Backspace => {
                if edit_hex.len() > 0 {
                    if pos > 0 {
                        let _ = edit_hex.remove(pos - 1);
                        pos -= 1;
                        if !hexedit {
                            let _ = edit_hex.remove(pos - 1);
                            pos -= 1;
                        }
                    }
                }
            }
            Key::Char('\t') | Key::Up | Key::Down => {
                if hexedit {
                    if edit_hex.len() % 2 == 1 {
                        edit_hex.insert(0, '0');
                    }
                    if pos % 2 == 1 {
                        pos += 1;
                    }
                }
                hexedit = !hexedit;
            }
            Key::Delete => {
                if edit_hex.len() > 0 {
                    if pos < edit_hex.len() {
                        let _ = edit_hex.remove(pos);
                        if !hexedit {
                            let _ = edit_hex.remove(pos);
                        }
                    }
                }
            }
            Key::Left => {
                if pos > 0 {
                    pos -= 1;
                    if !hexedit {
                        pos -= 1;
                    }
                }
            }
            Key::Right => {
                if pos < edit_hex.len() {
                    pos += 1;
                    if !hexedit {
                        pos += 1;
                    }
                }
            }
            Key::Char(c) => {
                if hexedit {
                    if c.is_ascii_hexdigit() {
                        edit_hex.insert(pos, c);
                        pos += 1;
                    }
                } else {
                    if c.is_ascii() {
                        for ic in hex::encode(vec![c as u8]).chars() {
                            edit_hex.insert(pos, ic);
                            pos += 1;
                        }
                    }
                }
            }
            Key::Home => pos = 0,
            Key::End => pos = edit_hex.len(),
            Key::Esc => break,
            _ => (),
        }
    }
    Ok(())
}

fn edit_int(val: &mut Integer, valid_values: &Vec<String>, stdout: &mut RawTerminal<Stdout>) {
    let mut new_int = val.as_signed().unwrap();
    let mut keys = std::io::stdin().keys();
    let mut selected = 0;
    let mut hit_space = false;
    let mut new = new_int.to_string();
    loop {
        if valid_values.len() > 0 {
            //            new_int = new.parse::<i64>().unwrap();
            let mut hex_val;
            write!(stdout, "\x1b8\r\n\x1B[2K\r\n").unwrap();
            for (i, vals) in valid_values.iter().enumerate() {
                if i == selected {
                    write!(stdout, "\x1b[7m").unwrap();
                }
                hex_val = vals.split("---").next().unwrap().trim().to_owned();
                if hex_val.contains(' ') {
                    hex_val = hex_val.split(" ").next().unwrap().trim().to_owned();
                }
                if hex_val.len() > 2 && &hex_val[..2] == "0x" {
                    let dec_val = i64::from_str_radix(&hex_val[2..], 16).unwrap();
                    if dec_val & new_int == dec_val {
                        write!(stdout, "\x1b[32m").unwrap();
                        if hit_space && i == selected {
                            new_int -= dec_val;
                            new = new_int.to_string();
                            hit_space = false;
                            write!(stdout, "\x1b[31m").unwrap();
                        }
                    } else {
                        write!(stdout, "\x1b[31m").unwrap();
                        if hit_space && i == selected {
                            new_int += dec_val;
                            new = new_int.to_string();
                            hit_space = false;
                            write!(stdout, "\x1b[32m").unwrap();
                        }
                    }
                }
                write!(stdout, "{}\x1b[0m\x1B[0K\r\n", vals).unwrap();
            }
            write!(stdout, "\x1B[2K\r\n").unwrap();
        }
        write!(stdout, "\x1B8{}\x1B[0K", new).unwrap();
        stdout.flush().unwrap();

        match keys.next().unwrap() {
            Ok(key) => match key {
                Key::Char('\n') => {
                    *val = match new.parse::<i64>() {
                        Ok(i) => Integer::from(i),
                        _ => Integer::from(0),
                    };
                    break;
                }
                Key::Backspace => {
                    if new.len() > 0 {
                        let _ = new.pop().unwrap();
                    }
                    if new.len() == 0 {
                        new_int = 0;
                    } else if &new != "-" {
                        new_int = new.parse::<i64>().unwrap();
                    }
                }
                Key::Char(' ') => hit_space = true,
                Key::Char(c @ '0'..='9') => {
                    new.push(c);
                    new_int = new.parse::<i64>().unwrap();
                }
                Key::Char('-') => {
                    if new.len() == 0 {
                        new.push('-');
                    }
                }
                Key::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                Key::Down => {
                    if selected < valid_values.len() - 1 {
                        selected += 1;
                    }
                }
                Key::Esc => break,
                _ => (),
            },
            _ => (),
        }
    }
}

fn edit_string(
    val: &mut String,
    valid_values: &Vec<String>,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    let mut new = String::from(&*val);
    let mut pos = new.len();
    let mut keys = std::io::stdin().keys();
    let mut selected = valid_values.len();
    if valid_values.len() > 0 {
        for (i, vals) in valid_values.iter().enumerate() {
            if vals.split("---").next().unwrap().trim() == &new {
                selected = i;
            }
        }
    }
    loop {
        if valid_values.len() > 0 {
            write!(stdout, "\x1b8\r\n\x1B[2K\r\n").unwrap();
            for (i, vals) in valid_values.iter().enumerate() {
                if i == selected {
                    write!(stdout, "\x1b[7m").unwrap();
                }
                write!(stdout, "{}\x1b[0m\x1B[0K\r\n", vals).unwrap();
            }
            write!(stdout, "\x1B[2K\r\n").unwrap();
        }
        write!(stdout, "\x1B8{}\x1B[0K", new)?;
        write!(stdout, "\x1B8{}", "\x1B[C".repeat(pos))?;
        stdout.flush()?;
        match keys.next().unwrap() {
            Ok(key) => match key {
                Key::Char('\n') => {
                    *val = new;
                    break;
                }
                Key::Backspace => {
                    if new.len() > 0 {
                        if pos > 0 {
                            let _ = new.remove(pos - 1);
                            pos -= 1;
                        }
                    }
                }
                Key::Delete => {
                    if new.len() > 0 {
                        if pos < new.len() {
                            let _ = new.remove(pos);
                        }
                    }
                }
                Key::Up => {
                    if selected > 0 {
                        selected -= 1;
                        new = valid_values[selected]
                            .split("---")
                            .next()
                            .unwrap()
                            .trim()
                            .to_owned();
                        pos = new.len();
                    }
                }
                Key::Down => {
                    if selected < valid_values.len() - 1 {
                        selected += 1;
                    }
                    if selected == valid_values.len() {
                        selected = 0;
                    }
                    new = valid_values[selected]
                        .split("---")
                        .next()
                        .unwrap()
                        .trim()
                        .to_owned();
                    pos = new.len();
                }
                Key::Left => {
                    if pos > 0 {
                        pos -= 1;
                    }
                }
                Key::Right => {
                    if pos < new.len() {
                        pos += 1;
                    }
                }
                Key::Char(c) => {
                    if c.is_ascii() {
                        new.insert(pos, c);
                        pos += 1;
                    }
                }
                Key::Home => pos = 0,
                Key::End => pos = new.len(),
                Key::Esc => break,
                _ => (),
            },
            _ => (),
        }
    }
    Ok(())
}
