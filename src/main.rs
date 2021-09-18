mod build;
mod draw;
mod edit;
mod init;
mod parse_tex;
mod res;
mod snake;

use fs_extra::dir::{copy, CopyOptions};
use std::io::{stdin, stdout, Stdout, Write};
use std::path::{Path, PathBuf};
use std::{env, error::Error};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::{clear, color, style};

use crate::build::build_output;
use crate::draw::{update_screen, Position};
use crate::edit::{add_delete_value, add_item, edit_value, extract_value};
use crate::init::init;
use crate::res::Resources;
use crate::snake::snake;

fn process(
    config_plist: &PathBuf,
    current_dir: &PathBuf,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    let stdin = stdin();
    let mut resources = Resources {
        acidanthera: serde_json::Value::Bool(false),
        dortania: serde_json::Value::Bool(false),
        octool_config: serde_json::Value::Bool(false),
        resource_list: serde_json::Value::Bool(false),
        other: res::get_serde_json("tool_config_files/other.json", stdout)?,
        config_plist: plist::Value::Boolean(false),
        sample_plist: plist::Value::Boolean(false),
        working_dir: env::current_dir()?,
        open_core_pkg: PathBuf::new(),
    };

    let mut position = Position {
        config_file_name: String::new(),
        sec_num: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        item_clone: plist::Value::Boolean(false),
        held_item: plist::Value::Boolean(false),
        held_key: Default::default(),
        sec_length: [0; 5],
        resource_sections: vec![],
        build_type: "release".to_string(),
        can_expand: false,
    };

    init(&config_plist, &mut resources, &mut position, stdout)?;

    writeln!(
        stdout,
        "\x1B[32mdone with init, \x1B[0;7mq\x1B[0;32m to quit, any other key to continue\x1B[0m\r"
    )?;
    let key = std::io::stdin().keys().next().unwrap().unwrap();

    if key != Key::Char('q') {
        update_screen(&mut position, &resources.config_plist, stdout)?;
        stdout.flush().unwrap();
        let mut showing_info = false;
        for key in stdin.keys() {
            let key = key.unwrap();
            match key {
                Key::Char('q') => {
                    if showing_info {
                        showing_info = false;
                    } else {
                        break;
                    }
                }
                Key::Char('G') => {
                    //                        term.clear_screen()?;
                    let build_okay = build_output(&resources, stdout)?;
                    writeln!(
                        stdout,
                        "\n\x1B[32mValidating\x1B[0m OUTPUT/EFI/OC/config.plist\r"
                    )?;
                    let config_okay = init::validate_plist(
                        &Path::new("OUTPUT/EFI/OC/config.plist").to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    if !build_okay || !config_okay {
                        writeln!(stdout, "\n\x1B[31mErrors occured while building OUTPUT/EFI, you should fix them before using it\x1B[0m\r")?;
                    } else {
                        writeln!(stdout, "\n\x1B[32mFinished building OUTPUT/EFI\x1B[0m\r")?;
                        if &env::current_dir().unwrap() != current_dir {
                            writeln!(stdout, "Copying OUTPUT EFI folder to this directory\r")?;
                            let mut options = CopyOptions::new();
                            options.overwrite = true;
                            copy("OUTPUT/EFI", current_dir, &options)?;
                        }
                    }
                    break;
                }
                Key::Char('a') => add_item(&mut position, &mut resources, stdout),
                Key::Char('v') | Key::Char('p') => {
                    if add_delete_value(&mut position, &mut resources.config_plist, true) {
                        position.add();
                    }
                }
                Key::Char('P') => {
                    res::print_parents(&resources, stdout);
                    //                    write!(stdout, "{:?}", resources.config_plist).unwrap();
                    stdout.flush().unwrap();
                    std::io::stdin().keys().next().unwrap().unwrap();
                }
                Key::Up | Key::Char('k') => position.up(),
                Key::Down | Key::Char('j') => position.down(),
                Key::Left | Key::Char('h') => position.left(),
                Key::Right | Key::Char('l') => position.right(),
                Key::Home | Key::Char('t') => position.sec_num[position.depth] = 0,
                Key::End | Key::Char('b') => {
                    position.sec_num[position.depth] = position.sec_length[position.depth] - 1
                }
                Key::Char(' ') => {
                    if !showing_info {
                        //                   showing_info = false;
                        //               } else {
                        edit_value(&position, &mut resources.config_plist, stdout, true)?;
                    }
                }
                Key::Char('\n') | Key::Char('\t') => {
                    edit_value(&position, &mut resources.config_plist, stdout, false)?
                }
                Key::Char('D') | Key::Ctrl('x') => {
                    if add_delete_value(&mut position, &mut resources.config_plist, false) {
                        position.delete();
                    }
                }
                Key::Char('x') | Key::Char('d') => {
                    if position.sec_length[position.depth] > 0 {
                        write!(stdout,"\r\n{und}Press{res} '{grn}d{res}' or '{grn}x{res}' to remove {yel}{obj}{res}, any other key to cancel.{clr}\r\n{yel}You can use '{grn}p{yel}' to place {obj} back into plist{res}{clr}\r\n{clr}",
                            obj = &position.sec_key[position.depth],
                            yel = color::Fg(color::Yellow),
                            grn = color::Fg(color::Green),
                            und = style::Underline,
                            res = style::Reset,
                            clr = clear::UntilNewline,
                        )?;
                        stdout.flush()?;
                        let kp = std::io::stdin().keys().next().unwrap().unwrap();
                        if kp == Key::Char('d') || kp == Key::Char('x') {
                            if add_delete_value(&mut position, &mut resources.config_plist, false) {
                                position.delete();
                            }
                        }
                    }
                }
                Key::Char('c') | Key::Char('y') => {
                    let _ = extract_value(&mut position, &mut resources.config_plist, true);
                }
                Key::Char('r') => {
                    if position.depth < 4 {
                        let mut obj = String::new();
                        for i in 0..position.depth + 1 {
                            obj.push_str(&position.sec_key[i]);
                            obj.push(' ');
                        }
                        write!(stdout,"\r\n{und}Press{res} '{grn}r{res}' again to reset {yel}{obj}{res}to the Sample.plist values, any other key to cancel.{clr}\r\n{yel}You can use '{grn}p{yel}' to place old {grn}{cur}{yel} back into plist if needed{res}{clr}\r\n{clr}",
                            obj = &obj,
                            cur = &position.sec_key[position.depth],
                            yel = color::Fg(color::Yellow),
                            grn = color::Fg(color::Green),
                            und = style::Underline,
                            res = style::Reset,
                            clr = clear::UntilNewline,
                        )?;
                        stdout.flush()?;
                        if std::io::stdin().keys().next().unwrap().unwrap() == Key::Char('r') {
                            if extract_value(&mut position, &resources.config_plist, true) {
                                let tmp_item = position.held_item.clone();
                                let tmp_key = position.held_key.clone();
                                if extract_value(&mut position, &resources.sample_plist, true) {
                                    let _ = add_delete_value(
                                        &mut position,
                                        &mut resources.config_plist,
                                        true,
                                    );
                                }
                                position.held_key = tmp_key.to_owned();
                                position.held_item = tmp_item.to_owned();
                            }
                        }
                    }
                }
                Key::Char('i') => {
                    if !showing_info {
                        if position.is_resource() {
                            let _ = res::show_res_path(&resources, &position, stdout);
                            showing_info = true;
                        } else {
                            showing_info = parse_tex::show_info(&position, stdout)?;
                        }
                        write!(stdout, "{}\x1B[0K", "_".repeat(70))?;
                        stdout.flush()?;
                    } else {
                        showing_info = false;
                    }
                }
                Key::Char('M') => {
                    snake(stdout)?;
                    std::io::stdin().keys().next().unwrap().unwrap();
                    write!(stdout, "\x1B[2J")?;
                }
                Key::Char('s') => {
                    let mut config_file = PathBuf::from(position.config_file_name)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    if !config_file.starts_with("modified_") {
                        let mut tmp = "modified_".to_string();
                        tmp.push_str(&config_file);
                        config_file = tmp.to_owned();
                    }
                    let save_file = PathBuf::from("INPUT").join(&config_file);
                    write!(stdout, "\r\n\n\x1B[0JSaving copy of plist to INPUT directory\r\n\n\x1B[32mValidating\x1B[0m {} with Acidanthera/ocvalidate\r\n", config_file)?;
                    resources.config_plist.to_file_xml(&save_file)?;
                    let _ = init::validate_plist(
                        &Path::new(&save_file).to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    break;
                }
                _ => (),
            }
            if key != Key::Char('i') && key != Key::Char(' ') {
                showing_info = false;
            }
            if !showing_info {
                update_screen(&mut position, &resources.config_plist, stdout)?;
                stdout.flush().unwrap();
            }
        }
    }

    write!(stdout, "\n\r\x1B[0J")?;
    stdout.flush()?;

    Ok(())
}

fn main() {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(stdout, "{}", termion::clear::All).unwrap();
    write!(stdout, "{}", termion::cursor::Hide).unwrap();
    write!(stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();

    #[cfg(not(debug_assertions))] // point to octool dir no matter where tool run from
    {
        let current_dir = env::current_dir().unwrap();
        let working_dir = env::current_exe().unwrap();
        if working_dir != current_dir {
            let working_dir = working_dir.parent().unwrap();
            env::set_current_dir(working_dir).unwrap();
        }
    }
    let current_dir = env::current_dir().unwrap();

    let mut config_file = Path::new(&match env::args().nth(1) {
        Some(s) => s,
        None => "INPUT/config.plist".to_string(),
    })
    .to_owned();

    if !config_file.has_root() {
        config_file = current_dir.join(config_file);
    }

    if !config_file.exists() {
        write!(
            stdout,
            "\x1B[31mDid not find config at\x1B[0m {:?}\r\n",
            config_file
        )
        .unwrap();
        config_file = Path::new("tool_config_files/OpenCorePkg/Docs/Sample.plist").to_owned();
    }
    write!(
        stdout,
        "\x1B[32mUsing\x1B[0m {}\r\n",
        config_file.to_str().unwrap()
    )
    .unwrap();
    stdout.flush().unwrap();

    match process(&config_file, &current_dir, &mut stdout) {
        Ok(()) => (),
        Err(e) => write!(stdout, "\n{:?}\r\n", e).unwrap(),
    }
    write!(stdout, "{}", termion::cursor::Show).unwrap();
}
