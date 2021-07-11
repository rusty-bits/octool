mod draw;
mod edit;
mod parse_tex;

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
        item_clone: list.clone(),
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
            Key::ArrowUp | Key::Char('k') => position.up(),
            Key::ArrowDown | Key::Char('j') => position.down(),
            Key::ArrowLeft | Key::Char('h') => position.left(),
            Key::ArrowRight | Key::Char('l') => position.right(),
            Key::Home => position.section[position.depth] = 0,
            Key::End => position.section[position.depth] = position.sec_length[position.depth] - 1,
            Key::Char(' ') => edit_value(&position, &mut list, &term, true)?,
            Key::Enter | Key::Tab => edit_value(&position, &mut list, &term, false)?,
            Key::Char('i') => {
                parse_tex::show_info(&position, &term);
                let _ = term.read_key()?;
            }
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

/*
fn get_info(position: &Position, term: &Term) {
//    let config = Config::new(env::args()).unwrap_or_else(|err| {
//        eprintln!("Problem parsing arguments: {}", err);
//        process::exit(1);
//    });

    let mut tmp = "\\section{".to_string();
    tmp.push_str(&position.sec_key[0]);
    tmp.push('}');
    let config = Config {
        query: tmp,
        filename: "INPUT/Configuration.tex".to_string(),
        case_sensitive: true,
    };

    if let Err(e) = parse_tex::run(config, &term) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}*/
