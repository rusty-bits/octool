extern crate hex;
extern crate plist;

use crate::draw;

//use plist::Dictionary;
use plist::Value;

use console::{Key, Term};

use std::io::Write;

pub fn edit_value(position: &draw::Position, mut val: &mut Value, term: &Term) {
    term.show_cursor().unwrap();
    for i in 0..position.depth + 1 {
        match val {
            Value::Dictionary(d) => {
                let k = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                    [position.section[i]]
                    .clone();
                val = d.get_mut(&k).unwrap();
            }
            Value::Array(a) => {
                val = a.get_mut(position.section[i]).unwrap();
            }
            _ => (),
        }
    }
    match val {
        Value::Boolean(b) => *b = !*b,
        Value::Integer(i) => *i = plist::Integer::from(42),
        Value::String(s) => {
            let res = term.read_line_initial_text(s).unwrap();
/*
            let mut old = String::from(&*s);
            write!(&*term, "\x1B[u{}", old).unwrap();
            term.flush().unwrap();
            loop {
                let key = term.read_key().unwrap();
                match key {
                    Key::Enter => {
                        *s = old;
                        break;
                    }
                    Key::Backspace => {
                        if old.len() > 0 {
                            let _ = old.pop().unwrap();
                        }
                    }
                    Key::Char(c) => old.push(c),
                    Key::Escape => break,
                    _ => (),
                }
                write!(&*term, "\x1B[u{}\x1B[K", old).unwrap();
                term.flush().unwrap();
            }*/
            *s = res;
        }
        Value::Data(d) => *d = vec![66, 111, 111, 98],
        _ => (),
    }
    term.hide_cursor().unwrap();
}
