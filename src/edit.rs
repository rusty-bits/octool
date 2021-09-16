use crate::draw::{get_lossy_string, hex_str_with_style, Position};
use plist::{Integer, Value};
use termion::event::Key;
use termion::{color, cursor};
use termion::{input::TermRead, raw::RawTerminal, style};

use std::{
    error::Error,
    io::{Stdout, Write},
};

pub fn extract_value(position: &mut Position, mut plist_val: &Value, actual: bool) -> bool {
    let mut extracted = false;
    for i in 0..position.depth {
        match plist_val {
            Value::Dictionary(d) => {
                plist_val = match d.get(&position.sec_key[i]) {
                    Some(k) => k,
                    None => panic!("failure to get Value from Sample.plist"),
                }
            }
            Value::Array(a) => {
                plist_val = a.get(0).expect("No 0 element in Sample.plist");
            }
            _ => (),
        }
    }
    match plist_val {
        Value::Dictionary(d) => {
            //            let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()[0].clone();
            let key = if actual {
                position.sec_key[position.depth].clone()
            } else {
                d.keys().next().expect("Didn't get first key").clone()
            };
            position.held_item = d
                .get(&key)
                .expect("No value for key in Sample.plist")
                .to_owned();
            position.held_key = key.to_owned();
            extracted = true;
        }
        Value::Array(a) => {
            let num = if actual {
                position.depth
            } else {
                0
            };
            position.held_item = a.get(num).expect("No elment in Sample.plist").to_owned();
            let c = position.held_item.as_dictionary_mut().unwrap();
            for val in c.values_mut() {
                match val {
                    Value::String(_) => *val = Value::String("".to_string()),
                    Value::Boolean(_) => *val = Value::Boolean(false),
                    _ => (),
                }
            }

            position.held_key = Default::default();
            extracted = true;
        }
        _ => (),
    }
    extracted
}

pub fn add_delete_value(position: &mut Position, mut plist_val: &mut Value, add: bool) -> bool {
    let mut changed = false;
    for i in 0..position.depth {
        match plist_val {
            Value::Dictionary(d) => {
                //                let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                //                    [position.sec_num[i]]
                //                    .clone();
                plist_val = match d.get_mut(&position.sec_key[i]) {
                    Some(k) => k,
                    None => panic!("failure to get Value from Dict"),
                }
            }
            Value::Array(a) => {
                plist_val = a.get_mut(position.sec_num[i]).unwrap();
            }
            _ => (),
        }
    }
    match plist_val {
        Value::Dictionary(d) => {
            if add {
                if d.insert(position.held_key.to_owned(), position.held_item.to_owned()) == None {
                    changed = true;
                };
            } else {
                //                let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                //                    [position.sec_num[position.depth]]
                //                    .clone();
                position.held_item = d.remove(&position.sec_key[position.depth]).unwrap();
                position.held_key = position.sec_key[position.depth].to_owned();
                changed = true;
            }
            d.sort_keys();
        }
        Value::Array(a) => {
            if add {
                a.insert(
                    position.sec_num[position.depth],
                    position.held_item.to_owned(),
                );
            } else {
                position.held_item = a.remove(position.sec_num[position.depth]);
                position.held_key = Default::default();
            }
            changed = true;
        }
        _ => (),
    }
    changed
}

pub fn add_item(position: &mut Position, plist: &mut Value, stdout: &mut RawTerminal<Stdout>) {
    let mut selection = 1;
    let item_types = [
        "Array",
        "Boolean",
        "Data",
        "Dictionary",
        "Integer",
        "String",
    ];
    write!(
        stdout,
        "\r\nSelect new item type:\x1B[0K\r\n{}",
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
    stdout.suspend_raw_mode().unwrap();
    write!(
        stdout,
        "Enter key for new {} item: {}\x1B[0K",
        item_types[selection - 1],
        cursor::Show
    )
    .unwrap();
    stdout.flush().unwrap();
    let mut key = String::new();
    match std::io::stdin().read_line(&mut key) {
        Ok(_) => (),
        Err(err) => panic!("{} Error reading key", err),
    }
    position.held_key = String::from(key.trim());
    position.held_item = match item_types[selection - 1] {
        "Array" => plist::Value::Array(vec![]),
        "Boolean" => false.into(),
        "Data" => plist::Value::Data(vec![]),
        "Dictionary" => plist::Value::Dictionary(plist::Dictionary::default()),
        "Integer" => 0.into(),
        "String" => plist::Value::String("".to_string()),
        _ => panic!("How did you select this?"),
    };
    write!(stdout, "{}", cursor::Hide).unwrap();
    stdout.activate_raw_mode().unwrap();

    if add_delete_value(position, plist, true) {
        position.add();
    }
}

pub fn edit_value(
    position: &Position,
    mut val: &mut Value,
    stdout: &mut RawTerminal<Stdout>,
    space: bool,
) -> Result<(), Box<dyn Error>> {
    write!(
        stdout,
        "{}\x1B[H\x1B[0K {inv}enter{res} save changes   {inv}esc{res} cancel changes",
        cursor::Show,
        inv = style::Invert,
        res = style::Reset,
    )?;
    for i in 0..position.depth + 1 {
        match val {
            Value::Dictionary(d) => {
                let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                    [position.sec_num[i]]
                    .clone();
                val = match d.get_mut(&key) {
                    Some(k) => k,
                    None => panic!("failure to get Value from Dict"),
                }
            }
            Value::Array(a) => {
                val = a.get_mut(position.sec_num[i]).unwrap();
            }
            _ => (),
        }
    }

    if space {
        match val {
            Value::Boolean(b) => *b = !*b,
            Value::Dictionary(d) => match d.get_mut("Enabled") {
                Some(Value::Boolean(b)) => *b = !*b,
                _ => (),
            },
            /*            Value::String(s) => {
                if s.starts_with('#') {
                    s.remove(0);
                } else {
                    s.insert(0, '#');
                }
            }
            */
            _ => (),
        }
    } else {
        match val {
            Value::Integer(i) => edit_int(i, stdout),
            Value::String(s) => edit_string(s, stdout)?,
            Value::Data(d) => edit_data(d, stdout)?,
            _ => (),
        }
    }

    write!(stdout, "{}", cursor::Hide)?;
    Ok(())
}

fn edit_data(val: &mut Vec<u8>, stdout: &mut RawTerminal<Stdout>) -> Result<(), Box<dyn Error>> {
    let mut edit_hex = hex::encode(val.clone());
    let mut pos = edit_hex.len();
    let mut hexedit = true;
    loop {
        let mut tmp_val = edit_hex.clone();
        if tmp_val.len() % 2 == 1 {
            tmp_val.insert(0, '0');
        }
        let tmp_val = hex::decode(tmp_val).unwrap();
        write!(
            stdout,
            "\x1B8\x1B[G{mag}as hex{res}\x1B8{}\x1B[0K\x1B[E{mag}as string\x1B[0K\x1B8\x1B[B{}\x1B8",
            hex_str_with_style(edit_hex.clone()),
            get_lossy_string(&tmp_val),
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
        let key = std::io::stdin().keys().next().unwrap().unwrap();
        match key {
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
        //        stdout.flush()?;
    }
    Ok(())
}

fn edit_int(val: &mut Integer, stdout: &mut RawTerminal<Stdout>) {
    let mut new = val.to_string();
    loop {
        write!(stdout, "\x1B8{}\x1B[0K", new).unwrap();
        stdout.flush().unwrap();
        let key = std::io::stdin().keys().next().unwrap();
        match key {
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
                }
                Key::Char(c @ '0'..='9') => new.push(c),
                Key::Char('-') => {
                    if new.len() == 0 {
                        new.push('-');
                    }
                }
                Key::Esc => break,
                _ => (),
            },
            _ => (),
        }
    }
}

fn edit_string(val: &mut String, stdout: &mut RawTerminal<Stdout>) -> Result<(), Box<dyn Error>> {
    let mut new = String::from(&*val);
    let mut pos = new.len();
    loop {
        write!(stdout, "\x1B8{}\x1B[0K", new)?;
        write!(stdout, "\x1B8{}", "\x1B[C".repeat(pos))?;
        stdout.flush()?;
        let key = std::io::stdin().keys().next().unwrap();
        match key {
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
