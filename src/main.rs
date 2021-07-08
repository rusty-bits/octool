mod draw;
mod edit;

use console::{Key, Term};
use plist::Value;
use std::io::Write;
use std::{env, io};

use draw::{draw_screen, Position};
use edit::edit_value;

fn do_stuff() -> io::Result<()> {
    let file = env::args()
        .nth(1)
        .unwrap_or("INPUT/config.plist".to_string());

    let mut list =
        Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());

    let mut position = Position {
        file_name: file.to_owned(),
        section: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        sec_length: [list.as_dictionary().unwrap().keys().len(), 0, 0, 0, 0],
    };

    let term = Term::stdout();
    term.set_title("octool");
    term.hide_cursor()?;

    draw_screen(&mut position, &list, &term);

    loop {
        let key = term.read_key()?;
        match key {
            Key::Escape | Key::Char('q') => break,
            Key::ArrowUp => position.up(),
            Key::ArrowDown => position.down(),
            Key::ArrowLeft => position.left(),
            Key::ArrowRight => position.right(),
            Key::Home => position.section[position.depth] = 0,
            Key::End => position.section[position.depth] = position.sec_length[position.depth] - 1,
            Key::Char(' ') => edit_value(&position, &mut list, &term, true)?,
            Key::Enter => edit_value(&position, &mut list, &term, false)?,
            Key::Char('s') => {
                list.to_file_xml("test1").unwrap();
                break;
            }

            _ => (),
        }
        draw_screen(&mut position, &list, &term);
    }

    term.show_cursor()?;
    write!(&term, "\n\r")?;

    Ok(())
}

fn main() {
    do_stuff().unwrap();
}
