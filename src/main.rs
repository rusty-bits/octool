mod draw;
mod edit;
mod parse_tex;

use console::{Key, Term};
use git2::Repository;
use plist::Value;
use std::io::Write;
use std::{env, error::Error};

use draw::{update_screen, Position};
use edit::edit_value;

//fn do_stuff() -> io::Result<()> {
fn do_stuff() -> Result<(), Box<dyn Error>> {
    let term = Term::stdout();
    term.set_title("octool");
    term.clear_screen()?;
    term.hide_cursor()?;

    let url = "https://github.com/acidanthera/OpenCorePkg";
    let path = "resources/OpenCorePkg";
    write!(&term, "checking for {}\r\n", path)?;
    let _repo = match Repository::open(path) {
        Ok(repo) => {
            write!(&term, "Found OpenCorePkg at {}  ", path)?;
            repo
        }
        Err(_) => {
            write!(&term, "Cloning OpenCorePkg ... ")?;
            match Repository::clone(url, path) {
                Ok(repo) => repo,
                Err(e) => panic!("\r\nfailed to clone: {}", e),
            }
        }
    };
    write!(&term, "done\r\n")?;

    let file = env::args()
        .nth(1)
        .unwrap_or("resources/OpenCorePkg/Docs/Sample.plist".to_string());

    let mut list =
        Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());

    let mut position = Position {
        file_name: file.to_owned(),
        section: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        item_clone: list.clone(),
        sec_length: [list.as_dictionary().unwrap().keys().len(), 0, 0, 0, 0],
    };

    update_screen(&mut position, &list, &term);
    let mut showing_info = false;

    loop {
        let key = term.read_key()?;
        match key {
            Key::Escape | Key::Char('q') => break,
            Key::ArrowUp | Key::Char('k') => position.up(),
            Key::ArrowDown | Key::Char('j') => position.down(),
            Key::ArrowLeft | Key::Char('h') => position.left(),
            Key::ArrowRight | Key::Char('l') => position.right(),
            Key::Home => position.section[position.depth] = 0,
            Key::End => position.section[position.depth] = position.sec_length[position.depth] - 1,
            Key::Char(' ') => edit_value(&position, &mut list, &term, true)?,
            Key::Enter | Key::Tab => edit_value(&position, &mut list, &term, false)?,
            Key::Char('i') => {
                if !showing_info {
                    parse_tex::show_info(&position, &term);
                    showing_info = true;
                } else {
                    showing_info = false;
                }
            }
            Key::Char('s') => {
                list.to_file_xml("test1")?;
                break;
            }

            _ => (),
        }
        if key != Key::Char('i') {
            showing_info = false;
        }
        if !showing_info {
            update_screen(&mut position, &list, &term);
        }
    }
    term.show_cursor()?;
    write!(&term, "\n\r")?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    do_stuff()?;
    Ok(())
}
