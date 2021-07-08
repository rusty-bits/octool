use crate::draw;
use console::{Key, Term};
use plist::{Integer, Value};
use std::io::{self, Write};

pub fn edit_value(
    position: &draw::Position,
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

    if !space {
        match val {
            Value::Integer(i) => edit_int(i, term),
            Value::String(s) => edit_string(s, term, false),
            Value::Data(d) => edit_data(d, term),
            _ => (),
        }
    }

    term.hide_cursor()?;

    Ok(())
}

fn edit_data(val: &mut Vec<u8>, term: &Term) {
    //    let mut new = String::from_utf8(val.clone()).unwrap();
    let mut new = hex::encode_upper(val.clone());
    let mut pos = new.len();
    loop {
        let mut tmp = new.clone();
        if tmp.len() % 2 == 1 {
            tmp = hex::encode("\u{fffd}");
            //            tmp = hex::encode("�");
            //            tmp.insert(0, '0');
        }
        let tmp = hex::decode(tmp).unwrap();
        write!(
            &*term,
            "\x1B[u{} | \x1B[0K",
            new //            String::from_utf8_lossy(&tmp)
        )
        .unwrap();
        draw::display_lossy_string(&tmp, term);
        write!(&*term, "\x1B[u{}", new.get(0..pos).unwrap()).unwrap();
        let key = term.read_key().unwrap();
        match key {
            Key::Enter => {
                //                *val = Vec::<u8>::from(new);
                if new.len() % 2 == 1 {
                    new.insert(0, '0');
                }
                *val = hex::decode(new).unwrap();
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
            Key::Char(c @ '0'..='9') | Key::Char(c @ 'A'..='F') | Key::Char(c @ 'a'..='f') => {
                new.insert(pos, c);
                pos += 1;
            }
            Key::Home => pos = 0,
            Key::End => pos = new.len(),
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
                *val = Integer::from(new.parse::<i32>().unwrap());
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

fn edit_string(val: &mut String, term: &Term, hex: bool) {
    let mut new = String::from(&*val);
    let mut pos = new.len();
    loop {
        write!(&*term, "\x1B[u{}   {}\x1B[0K\x1B[u", new, pos).unwrap();
        write!(&*term, "{}", new.get(0..pos).unwrap()).unwrap();
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
            Key::Char(c @ '0'..='9') | Key::Char(c @ 'A'..='F') | Key::Char(c @ 'a'..='f') => {
                new.insert(pos, c);
                pos += 1;
            }
            Key::Home => pos = 0,
            Key::End => pos = new.len(),
            Key::Escape => break,
            _ => (),
        }
        if !hex {
            match key {
                Key::Char(c) => {
                    new.insert(pos, c);
                    pos += 1;
                }
                _ => (),
            }
        }
    }
}
