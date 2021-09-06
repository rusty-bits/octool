use crate::draw::{get_lossy_string, hex_str_with_style, Position};
use plist::{Integer, Value};
use termion::{color, cursor};
use termion::event::Key;
use termion::{input::TermRead, raw::RawTerminal, style};

use std::{
    error::Error,
    io::{Stdout, Write},
};

pub fn delete_value(position: &Position, mut val: &mut Value) -> bool {
    let mut deleted = false;
    for i in 0..position.depth {
        match val {
            Value::Dictionary(d) => {
                let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                    [position.section_num[i]]
                    .clone();
                val = match d.get_mut(&key) {
                    Some(k) => k,
                    None => panic!("failure to get Value from Dict"),
                }
            }
            Value::Array(a) => {
                val = a.get_mut(position.section_num[i]).unwrap();
            }
            _ => (),
        }
    }
    match val {
        Value::Dictionary(d) => {
            let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                [position.section_num[position.depth]]
                .clone();
            let _ = d.remove(&key);
            d.sort_keys();
            deleted = true;
        }
        Value::Array(a) => {
            let _ = a.remove(position.section_num[position.depth]);
            deleted = true;
        }
        _ => (),
    }
    deleted
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
                    [position.section_num[i]]
                    .clone();
                val = match d.get_mut(&key) {
                    Some(k) => k,
                    None => panic!("failure to get Value from Dict"),
                }
            }
            Value::Array(a) => {
                val = a.get_mut(position.section_num[i]).unwrap();
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
            Value::String(s) => {
                if s.starts_with('#') {
                    s.remove(0);
                } else {
                    s.insert(0, '#');
                }
            }
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
            "\x1B8\x1B[G{inv}{mag}as hex{res}\x1B8{}\x1B[0K\x1B[E{mag}as string\x1B[0K\x1B8\x1B[B{}\x1B8",
            hex_str_with_style(edit_hex.clone()),
            get_lossy_string(&tmp_val),
            mag = color::Fg(color::Magenta),
            res = style::Reset,
            inv = style::Invert,
        )?;
        if hexedit {
            write!(
                stdout,
                "\x1B[G{}{}as hex{}\x1B8{}",
                style::Invert, color::Fg(color::Magenta), style::Reset,
                "\x1B[C".repeat(pos)
            )?;
        } else {
            write!(
                stdout,
                "\x1B[E{}{}as string{}\x1B8\x1B[B{}",
                style::Invert, color::Fg(color::Magenta), style::Reset,
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
