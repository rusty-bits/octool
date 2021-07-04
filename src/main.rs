mod draw;
mod edit;

extern crate hex;
extern crate plist;

//use plist::Dictionary;
use plist::Value;

use console::{Key, Term};

use std::env;
use std::io::Write;

use crate::draw::draw_screen;

fn main() {
    let file = env::args()
        .nth(1)
        .unwrap_or("INPUT/Sample.plist".to_string());

    let mut list =
        Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());

    let mut position = draw::Position {
        file_name: file.to_owned(),
        section: [0; 5],
        sec_length: [list.as_dictionary().unwrap().keys().len(), 0, 0, 0, 0],
        depth: 0,
        can_expand: true,
    };

    let term = Term::stdout();
    term.set_title("octool");
    term.hide_cursor().unwrap();

    draw::draw_screen(&mut position, &list, &term);

    loop {
        let key = term.read_key().unwrap();
        match key {
            Key::Escape | Key::Char('q') => break,
            Key::ArrowUp => position.up(),
            Key::ArrowDown => position.down(),
            Key::ArrowLeft => position.left(),
            Key::ArrowRight => position.right(),
            Key::Char(' ') => edit::edit_value(&position, &mut list, &term),
            Key::Char('s') => {
                list.to_file_xml("test1").unwrap();
                break;
            }

            _ => (),
        }
        draw_screen(&mut position, &list, &term);
    }

    term.show_cursor().unwrap();
    write!(&term, "\n\r").unwrap();
}
