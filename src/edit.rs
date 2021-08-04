use crate::draw::{get_lossy_string, hex_str_with_style, Position};
use console::{style, Key, Term};
use plist::{Integer, Value};

use std::{error::Error, io::Write};

pub fn edit_value(
    position: &Position,
    mut val: &mut Value,
    term: &Term,
    space: bool,
) -> Result<(), Box<dyn Error>> {
    term.show_cursor()?;
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
    match val {
        Value::Boolean(b) => *b = !*b,
        Value::Dictionary(d) => match d.get_mut("Enabled") {
            Some(Value::Boolean(b)) => *b = !*b,
            _ => (),
        },
        _ => (),
    }

    if space {
        match val {
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
            Value::Integer(i) => edit_int(i, term),
            Value::String(s) => edit_string(s, term)?,
            Value::Data(d) => edit_data(d, term)?,
            _ => (),
        }
    }

    term.hide_cursor()?;
    Ok(())
}

fn edit_data(val: &mut Vec<u8>, term: &Term) -> Result<(), Box<dyn Error>> {
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
            &*term,
            "\x1B[u\x1B[G{}\x1B[u{}\x1B[0K\x1B[E{}\x1B[0K\x1B[u\x1B[B{}\x1B[u",
            style("as hex").magenta(),
            hex_str_with_style(edit_hex.clone()),
            style("as string").magenta(),
            get_lossy_string(&tmp_val)
        )?;
        if hexedit {
            write!(
                &*term,
                "\x1B[G{}\x1B[u{}",
                style("as hex").reverse().magenta(),
                "\x1B[C".repeat(pos)
            )?;
        } else {
            write!(
                &*term,
                "\x1B[E{}\x1B[u\x1B[B{}",
                style("as string").reverse().magenta(),
                "\x1B[C".repeat(pos / 2)
            )
            .unwrap();
        }
        let key = term.read_key()?;
        match key {
            Key::Enter => {
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
            Key::Tab | Key::ArrowUp | Key::ArrowDown => {
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
            Key::Del => {
                if edit_hex.len() > 0 {
                    if pos < edit_hex.len() {
                        let _ = edit_hex.remove(pos);
                        if !hexedit {
                            let _ = edit_hex.remove(pos);
                        }
                    }
                }
            }
            Key::ArrowLeft => {
                if pos > 0 {
                    pos -= 1;
                    if !hexedit {
                        pos -= 1;
                    }
                }
            }
            Key::ArrowRight => {
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
            Key::Escape => break,
            _ => (),
        }
    }
    Ok(())
}

fn edit_int(val: &mut Integer, term: &Term) {
    let mut new = val.to_string();
    loop {
        write!(&*term, "\x1B[u{}\x1B[0K", new).unwrap();
        let key = term.read_key().unwrap();
        match key {
            Key::Enter => {
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
            Key::Escape => break,
            _ => (),
        }
    }
}

fn edit_string(val: &mut String, term: &Term) -> Result<(), Box<dyn Error>> {
    let mut new = String::from(&*val);
    let mut pos = new.len();
    loop {
        write!(&*term, "\x1B[u{}\x1B[0K", new)?;
        write!(&*term, "\x1B[u{}", "\x1B[C".repeat(pos))?;
        let key = term.read_key()?;
        match key {
            Key::Enter => {
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
            Key::Del => {
                if new.len() > 0 {
                    if pos < new.len() {
                        let _ = new.remove(pos);
                    }
                }
            }
            Key::ArrowLeft => {
                if pos > 0 {
                    pos -= 1;
                }
            }
            Key::ArrowRight => {
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
            Key::Escape => break,
            _ => (),
        }
    }
    Ok(())
}
