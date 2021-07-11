use crate::draw::{Position, get_lossy_string, hex_str_with_style};
use console::{Key, Term, style};
use plist::{Integer, Value};
use std::io::{self, Write};

pub fn edit_value(
    position: &Position,
    mut val: &mut Value,
    term: &Term,
    space: bool,
) -> io::Result<()> {
    term.show_cursor()?;
    for i in 0..position.depth + 1 {
        match val {
            Value::Dictionary(d) => {
                let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                    [position.section[i]]
                    .clone();
                val = d.get_mut(&key).unwrap();
            }
            Value::Array(a) => {
                val = a.get_mut(position.section[i]).unwrap();
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

    if !space { //use space for toggle of bool or Enable only
        match val {
            Value::Integer(i) => edit_int(i, term),
            Value::String(s) => edit_string(s, term),
            Value::Data(d) => edit_data(d, term),
            _ => (),
        }
    }

    term.hide_cursor()?;

    Ok(())
}

fn edit_data(val: &mut Vec<u8>, term: &Term) {
    let mut edit_hex = hex::encode_upper(val.clone());
    let mut pos = edit_hex.len();
//    let mut tmp_val = val.clone();
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
            style("hex").yellow(),
            hex_str_with_style(edit_hex.clone()),
            style("string").yellow(),
            get_lossy_string(&tmp_val)
        )
        .unwrap();
        if hexedit {
            write!(&*term, "\x1B[G{}\x1B[u{}", style("hex").yellow().reverse(), "\x1B[C".repeat(pos)).unwrap();
        } else {
            write!(&*term, "\x1B[E{}\x1B[u\x1B[B{}", style("string").reverse().yellow(), "\x1B[C".repeat(pos / 2)).unwrap();
        }
        let key = term.read_key().unwrap();
        match key {
            Key::Enter => {
//                if new_hex.len() % 2 == 1 {
//                    new_hex.insert(0, '0');
//                }
//                *val = hex::decode(new_hex).unwrap();
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
                        for ic in hex::encode_upper(vec![c as u8]).chars() {
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

fn edit_string(val: &mut String, term: &Term) {
    let mut new = String::from(&*val);
    let mut pos = new.len();
    loop {
        write!(&*term, "\x1B[u{}\x1B[0K", new).unwrap();
        write!(&*term, "\x1B[u{}", "\x1B[C".repeat(pos)).unwrap();
        let key = term.read_key().unwrap();
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
}
