mod build;
mod draw;
mod edit;
mod init;
mod parse_tex;
mod res;
mod snake;

use fs_extra::dir::{copy, CopyOptions};
use std::io::{stdout, Stdout, Write};
use std::path::{Path, PathBuf};
use std::{env, error::Error};

use termion::{
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
    {clear, color, cursor, style},
};

fn process(
    config_plist: &mut PathBuf,
    current_dir: &PathBuf,
    settings: &mut draw::Settings,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    let mut found = vec![edit::Found::new()];
    let mut found_id: usize = 0;
    let mut resources = res::Resources {
        dortania: serde_json::json!(null),
        octool_config: serde_json::json!(null),
        resource_list: serde_json::json!(null),
        other: serde_json::json!(null),
        config_plist: plist::Value::Boolean(false),
        sample_plist: plist::Value::Boolean(false),
        working_dir_path: env::current_dir()?,
        open_core_binaries_path: PathBuf::new(),
        open_core_source_path: PathBuf::new(),
    };

    init::init(config_plist, &mut resources, settings, stdout)?;

    let mut key = Key::Char('q');

    if settings.oc_build_version != "not found" {
        writeln!(
        stdout,
        "\x1B[32mdone with init, \x1B[0;7mq\x1B[0;32m to quit, any other key to continue\x1B[0m\r"
    )?;
        stdout.flush()?;
        key = std::io::stdin().keys().next().unwrap().unwrap();
    }

    if key != Key::Char('q') {
        let mut showing_info = false;
        let mut keys = std::io::stdin().keys();
        loop {
            if !showing_info {
                draw::update_screen(settings, &mut resources, stdout)?;
                stdout.flush().unwrap();
            }
            let key = keys.next().unwrap().unwrap();
            match key {
                Key::Char('q') => {
                    if showing_info {
                        showing_info = false;
                    } else {
                        if settings.modified {
                            write!(stdout, "\r\n\x1B[33;7mNOTICE:\x1B[0m changes have been made to the plist file\x1B[0K\r\n capital 'Q' to quit without saving, any other key will cancel\x1B[0K\r\n\x1B[2K").unwrap();
                            stdout.flush().unwrap();
                            match std::io::stdin().keys().next().unwrap().unwrap() {
                                Key::Char('Q') => break,
                                _ => (),
                            }
                        } else {
                            break;
                        }
                    }
                }
                Key::Char('G') => {
                    let build_okay = build::build_output(settings, &resources, stdout)?;
                    writeln!(
                        stdout,
                        "\n\x1B[32mValidating\x1B[0m OUTPUT/EFI/OC/config.plist\r"
                    )?;
                    let config_okay = init::validate_plist(
                        &Path::new("OUTPUT/EFI/OC/config.plist").to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    let mut config_file = PathBuf::from(&settings.config_file_name)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    if !config_file.starts_with("last_built_") {
                        let mut tmp = "last_built_".to_string();
                        tmp.push_str(&config_file);
                        config_file = tmp.to_owned();
                    }
                    let save_file = PathBuf::from("INPUT").join(&config_file);
                    write!(
                        stdout,
                        "\r\n\x1B[0JSaving copy of plist as INPUT/{}\r\n\x1B[0K",
                        config_file
                    )
                    .unwrap();
                    resources.config_plist.to_file_xml(&save_file)?;
                    if !build_okay || !config_okay {
                        writeln!(
                            stdout,
                            "\n\x1B[31mErrors occured while building OUTPUT/EFI, \
                                 you should fix them before using it\x1B[0m\r"
                        )?;
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
                Key::Char('a') => edit::add_item(settings, &mut resources, stdout),
                Key::Char('f') => {
                    found = vec![];
                    found_id = 0;
                    edit::find(settings, &resources.config_plist, &mut found, stdout);
                    if found.len() == 1 {
                        settings.depth = found[0].level;
                        settings.sec_num = found[0].section;
                        settings.find_string = String::new();
                        found_id = 0;
                    } else if found.len() > 1 {
                        let mut selection = 1;
                        write!(stdout, "\r\n\x1B[2K{}", cursor::Save).unwrap();
                        loop {
                            write!(stdout, "\x1B8").unwrap();
                            for (i, f) in found.iter().enumerate() {
                                let mut fk = f.keys.iter();
                                write!(
                                    stdout,
                                    "  {}{}",
                                    if i == selection - 1 { "\x1B[7m" } else { "" },
                                    fk.next().unwrap()
                                )
                                .unwrap();
                                for next_key in fk {
                                    write!(stdout, "->{}", next_key).unwrap();
                                }
                                write!(stdout, "\x1B[0m\r\n\x1B[2K").unwrap();
                            }
                            stdout.flush().unwrap();
                            match std::io::stdin().keys().next().unwrap().unwrap() {
                                Key::Up => {
                                    if selection > 1 {
                                        selection -= 1;
                                    }
                                }
                                Key::Down => {
                                    if selection < found.len() {
                                        selection += 1;
                                    }
                                }
                                Key::Char('\n') => break,
                                Key::Esc => {
                                    selection = 0;
                                    break;
                                }
                                _ => (),
                            }
                        }
                        found_id = selection;
                        if selection > 0 {
                            settings.depth = found[selection - 1].level;
                            settings.sec_num = found[selection - 1].section;
                        }
                    }
                }
                Key::Char('n') => {
                    if found_id > 0 {
                        found_id += 1;
                        if found_id > found.len() {
                            found_id = 1;
                        }
                        settings.depth = found[found_id - 1].level;
                        settings.sec_num = found[found_id - 1].section;
                    }
                }
                Key::Char('p') | Key::Ctrl('v') => {
                    if edit::add_delete_value(settings, &mut resources.config_plist, true) {
                        settings.add();
                    }
                }
                Key::Char('P') => {
                    res::print_parents(&resources, stdout);
                    stdout.flush().unwrap();
                    std::io::stdin().keys().next().unwrap().unwrap();
                }
                Key::Up | Key::Char('k') => settings.up(),
                Key::Down | Key::Char('j') => settings.down(),
                Key::Left | Key::Char('h') => settings.left(),
                Key::Right | Key::Char('l') => settings.right(),
                Key::Home | Key::Char('t') => settings.sec_num[settings.depth] = 0,
                Key::End | Key::Char('b') => {
                    settings.sec_num[settings.depth] = settings.sec_length[settings.depth] - 1
                }
                // TODO: special check for driver section for OC 0.7.2 and earlier, uses # to enable/disable
                Key::Char(' ') => {
                    if !showing_info {
                        let empty_vec = vec![];
                        edit::edit_value(
                            settings,
                            &mut resources.config_plist,
                            &empty_vec,
                            stdout,
                            true,
                            false,
                        )?;
                    }
                }
                Key::Char('\n') | Key::Char('\t') => {
                    write!(stdout, "\x1b8\r\n")?;
                    let mut valid_values = vec![];
                    parse_tex::show_info(&resources, &settings, true, &mut valid_values, stdout)?;
                    edit::edit_value(
                        settings,
                        &mut resources.config_plist,
                        &valid_values,
                        stdout,
                        false,
                        false,
                    )?
                }
                Key::Char('K') => {
                    let empty_vec = vec![];
                    edit::edit_value(
                        settings,
                        &mut resources.config_plist,
                        &empty_vec,
                        stdout,
                        false,
                        true,
                    )?
                }
                Key::Char('D') | Key::Ctrl('x') => {
                    if settings.sec_length[settings.depth] > 0 {
                        if edit::add_delete_value(settings, &mut resources.config_plist, false) {
                            settings.delete();
                        }
                    }
                }
                Key::Char('d') => {
                    if settings.sec_length[settings.depth] > 0 {
                        write!(stdout,"\r\n{und}Press{res} '{grn}d{res}' again to remove {yel}{obj}{res}, \
                               any other key to cancel.{clr}\r\n{yel}You can use '{grn}p{yel}' to place {obj} \
                               back into plist{res}{clr}\r\n{clr}",
                            obj = &settings.sec_key[settings.depth],
                            yel = color::Fg(color::Yellow),
                            grn = color::Fg(color::Green),
                            und = style::Underline,
                            res = style::Reset,
                            clr = clear::UntilNewline,
                        )?;
                        stdout.flush()?;
                        let kp = std::io::stdin().keys().next().unwrap().unwrap();
                        if kp == Key::Char('d') || kp == Key::Char('x') {
                            if edit::add_delete_value(settings, &mut resources.config_plist, false)
                            {
                                settings.delete();
                            }
                        }
                    }
                }
                Key::Char('y') | Key::Ctrl('c') => {
                    let _ = edit::extract_value(settings, &mut resources.config_plist, false, true);
                }
                Key::Char('r') => {
                    if settings.depth < 4 {
                        let mut obj = String::new();
                        for i in 0..settings.depth + 1 {
                            obj.push_str(&settings.sec_key[i]);
                            obj.push(' ');
                        }
                        write!(stdout,"\r\n{und}Press{res} '{grn}r{res}' again to reset {yel}{obj}{res}to \
                               the Sample.plist values, any other key to cancel.{clr}\r\n{yel}You can use \
                               '{grn}p{yel}' to place old {grn}{cur}{yel} back into plist if needed{res}{clr}\r\n{clr}",
                            obj = &obj,
                            cur = &settings.sec_key[settings.depth],
                            yel = color::Fg(color::Yellow),
                            grn = color::Fg(color::Green),
                            und = style::Underline,
                            res = style::Reset,
                            clr = clear::UntilNewline,
                        )?;
                        stdout.flush()?;
                        if std::io::stdin().keys().next().unwrap().unwrap() == Key::Char('r') {
                            if edit::extract_value(settings, &resources.config_plist, false, true) {
                                settings.modified = true;
                                let tmp_item = settings.held_item.clone();
                                let tmp_key = settings.held_key.clone();
                                if edit::extract_value(
                                    settings,
                                    &resources.sample_plist,
                                    false,
                                    true,
                                ) {
                                    let _ = edit::add_delete_value(
                                        settings,
                                        &mut resources.config_plist,
                                        true,
                                    );
                                }
                                settings.held_key = tmp_key.to_owned();
                                settings.held_item = tmp_item;
                            }
                        }
                    }
                }
                Key::Char('m') => {
                    //    it might not make sense to merge an array, maybe use 'r'eset instead?
                    let initial_depth = settings.depth;
                    let initial_key = settings.held_key.to_owned();
                    let initial_item = settings.held_item.to_owned();
                    if !settings.can_expand && settings.depth > 0 {
                        settings.depth -= 1;
                    }
                    if edit::extract_value(settings, &resources.config_plist, false, true) {
                        match settings.held_item.clone().unwrap() {
                            plist::Value::Dictionary(mut d) => {
                                stdout.flush()?;
                                if edit::extract_value(
                                    settings,
                                    &resources.sample_plist,
                                    false,
                                    false,
                                ) {
                                    stdout.flush().unwrap();
                                    match settings.held_item.clone().unwrap() {
                                        plist::Value::Dictionary(d2) => {
                                            for (k, v) in d2 {
                                                if !d.contains_key(&k) {
                                                    d.insert(k.to_owned(), v.to_owned());
                                                }
                                            }
                                        }
                                        _ => (),
                                    }
                                    let _ = edit::add_delete_value(
                                        settings,
                                        &mut resources.config_plist,
                                        false,
                                    );
                                    d.sort_keys();
                                    settings.held_item =
                                        Some(plist::Value::Dictionary(d.to_owned()));
                                    let _ = edit::add_delete_value(
                                        settings,
                                        &mut resources.config_plist,
                                        true,
                                    );
                                    if initial_depth != settings.depth {
                                        settings.sec_length[initial_depth] = d.len();
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                    settings.held_key = initial_key;
                    settings.held_item = initial_item;
                    settings.depth = initial_depth;
                    settings.modified = true;
                }
                Key::Char('i') => {
                    if !showing_info {
                        if settings.is_resource() {
                            let _ = res::show_res_path(&resources, &settings, stdout);
                            showing_info = true;
                        } else {
                            let mut empty_vec = vec![];
                            showing_info = parse_tex::show_info(
                                &resources,
                                &settings,
                                false,
                                &mut empty_vec,
                                stdout,
                            )?;
                        }
                        write!(stdout, "{}\x1B[0K", "_".repeat(70))?;
                        stdout.flush()?;
                    } else {
                        showing_info = false;
                    }
                }
                Key::Char('M') => {
                    snake::snake(stdout)?;
                    std::io::stdin().keys().next().unwrap().unwrap();
                    write!(stdout, "\x1B[2J")?;
                }
                Key::Char('s') => {
                    let mut config_file = PathBuf::from(&settings.config_file_name)
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
                    write!(
                        stdout,
                        "\r\n\n\x1B[0JSaving copy of plist to INPUT directory\r\n\n\x1B[32m\
                           Validating\x1B[0m {} with Acidanthera/ocvalidate\r\n",
                        config_file
                    )?;
                    resources.config_plist.to_file_xml(&save_file)?;
                    settings.modified = false;
                    let _ = init::validate_plist(
                        &Path::new(&save_file).to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    showing_info = true;
                }
                _ => (),
            }
            if key != Key::Char('i') && key != Key::Char(' ') && key != Key::Char('s') {
                showing_info = false;
            }
        }
    }

    write!(stdout, "\n\r\x1B[0J")?;
    stdout.flush()?;

    #[cfg(debug_assertions)]
    {
        println!("debug:   HashMap {:?}", settings.resource_ver_indexes);
    }

    Ok(())
}

fn main() {
    let current_dir = env::current_dir().expect("Didn't find current directory");

    let working_dir;
    let ver;
    #[cfg(not(debug_assertions))]
    {
        working_dir = env::current_exe()
            .unwrap()
            .parent()
            .expect("Didn't find working directory")
            .to_path_buf();
        ver = "0.3.2";
    }

    #[cfg(debug_assertions)]
    {
        working_dir = current_dir.to_owned();
        ver = "0.3.2 d";
    }
    env::set_current_dir(&working_dir).expect("Unable to set environment");

    let mut setup = draw::Settings {
        config_file_name: String::new(),
        sec_num: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        item_instructions: String::new(),
        held_item: None,
        held_key: Default::default(),
        sec_length: [0; 5],
        resource_sections: vec![],
        build_type: "release".to_string(),
        oc_build_version: "latest".to_string(),
        oc_build_date: String::new(),
        oc_build_version_res_index: 0,
        resource_ver_indexes: Default::default(),
        can_expand: false,
        find_string: Default::default(),
        modified: false,
    };

    for (k, v) in env::vars() {
        println!("{}\t{}", k, v);
    }

    let mut config_file = working_dir.join("INPUT/config.plist");
    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut args = args.iter();
    loop {
        if let Some(arg) = args.next() {
            //    for arg in args {
            if arg.starts_with('-') {
                for c in arg.chars() {
                    match c {
                        'o' => match args.next() {
                            Some(version) => setup.oc_build_version = version.to_owned(),
                            _ => {
                                println!(
                                    "\n\x1B[33mERROR:\x1b[0m You need to supply a version number with the -o option\n"
                                );
                                println!("e.g. './octool -o \x1b[4m0.7.4\x1b[0m'\n");
                                std::process::exit(0);
                            }
                        },
                        'v' => {
                            println!("\noctool v{}", ver);
                            // TODO: check if running on macOS
                            match res::status(
                                "nvram",
                                &["4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:opencore-version"],
                            ) {
                                Ok(s) => println!(
                                    "\ncurrent loaded OpenCore version\n{}",
                                    String::from_utf8_lossy(&s.stdout)
                                ),
                                Err(_) => (),
                            }
                            std::process::exit(0);
                        }
                        'd' => setup.build_type = "debug".to_string(),
                        'h' => {
                            println!("SYNOPSIS\n\t./octool [options] [-o x.y.z] [config.plist]\n");
                            println!("OPTIONS\n\t-d  build debug version\n\t-h  print this help\n\t-o x.y.z  \
                                     select OpenCore version number\n\t-v  show version info");
                            std::process::exit(0);
                        }
                        _ => (),
                    }
                }
            } else {
                config_file = current_dir.join(arg);
            }
        } else {
            break;
        }
    }

    let mut stdout = stdout().into_raw_mode().expect("Couldn't set stdout");
    write!(
        stdout,
        "{}{}{}",
        termion::clear::All,
        termion::cursor::Hide,
        termion::cursor::Goto(1, 1)
    )
    .unwrap();

    if !config_file.exists() {
        write!(
            stdout,
            "\x1B[31mDid not find config at\x1B[0m {:?}\r\nWill use the Sample.plist from the OpenCorePkg\r\n",
            config_file
        )
        .unwrap();
        config_file = Path::new("").to_path_buf();
    } else {
        write!(stdout, "Using {:?}\r\n", config_file).unwrap();
    }
    stdout.flush().unwrap();

    match process(&mut config_file, &current_dir, &mut setup, &mut stdout) {
        Ok(()) => (),
        Err(e) => eprintln!("\r\n\x1B[31mERROR:\x1B[0m while processing plist: {:?}", e),
    }
    write!(stdout, "{}", termion::cursor::Show).unwrap();
}
