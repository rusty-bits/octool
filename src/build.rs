use crate::draw::Settings;
use crate::res::{res_version, get_or_update_local_parent, get_res_path, status, Resources};

use fs_extra::dir::{self, CopyOptions};
use std::error::Error;
use std::fs;
use std::io::{self, Stdout, Write};
use std::path::Path;
use termion::raw::RawTerminal;

pub fn build_output(
    settings: &mut Settings,
    resources: &Resources,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<bool, Box<dyn Error>> {
    dir::remove("OUTPUT")?;
    fs::create_dir_all("OUTPUT/EFI")?;
    let mut has_open_canopy = false;
    let mut build_okay = true;
    let mut missing_files: Vec<&str> = vec![];

    let mut options = CopyOptions::new();
    options.overwrite = true;
    dir::copy(
        &resources.open_core_binaries_path.join("X64/EFI"),
        "OUTPUT",
        &options,
    )?;
    dir::remove("OUTPUT/EFI/OC/Drivers")?;
    dir::remove("OUTPUT/EFI/OC/Tools")?;
    fs::create_dir_all("OUTPUT/EFI/OC/Drivers")?;
    fs::create_dir_all("OUTPUT/EFI/OC/Tools")?;
    resources
        .config_plist
        .to_file_xml("OUTPUT/EFI/OC/config.plist")?;

    let res_config: Vec<(String, String, String, String)> =
        serde_json::from_value(resources.octool_config["resource_sections"].clone()).unwrap();
    for (sec, sub, pth, out_pth) in res_config {
        write!(
            stdout,
            "\x1B[0J\r\n\x1B[32mCopying\x1B[0m enabled {} files ...\r\n",
            &sec
        )?;
        stdout.flush()?;
        let mut from_paths = Vec::new();
        let enabled_resources = resources.config_plist.as_dictionary().unwrap()[&sec]
            .as_dictionary()
            .unwrap()[&sub]
            .as_array()
            .unwrap();
        for val in enabled_resources {
            let enabled_res = val.as_dictionary().unwrap();
            if enabled_res["Enabled"].as_boolean().unwrap() {
                let res = enabled_res[&pth]
                    .as_string()
                    .unwrap()
                    .split('/')
                    .next()
                    .unwrap();
                if &sub == "Drivers" && res == "OpenCanopy.efi" {
                    has_open_canopy = true;
                }
                res_version(settings, &resources, &res);
                match get_res_path(&settings, &resources, &res, &sec, stdout) {
                    Some(res) => {
                        from_paths.push(res);
                    }
                    None => {
                        build_okay = false;
                        missing_files.push(&res);
                        write!(
                            stdout,
                            "\x1B[31mERROR: {} not found, skipping\x1B[0m\r\n",
                            res
                        )?;
                    }
                }
            }
        }
        from_paths.sort();
        from_paths.dedup();
        let mut to_path = "OUTPUT/EFI/OC/".to_string();
        to_path.push_str(&out_pth);
        fs_extra::copy_items(&from_paths, &to_path, &options)?;
        let mut s = "";
        if from_paths.len() > 1 {
            s = "s";
        };
        if from_paths.len() == 0 {
            write!(stdout, "No files to copy\r\n\n")?;
        } else {
            write!(
                stdout,
                "\x1B[32mdone\x1B[0m with {} file{}\r\n\n",
                from_paths.len(),
                s
            )?;
        }
    }

    if has_open_canopy {
        write!(
            stdout,
            "\x1B[32mFound\x1B[0m OpenCanopy.efi Enabled in UEFI->Drivers\r\n"
        )?;
        get_or_update_local_parent(
            "OcBinaryData",
            &resources.other,
            "release",
            &0,
            stdout,
        )?;
        let canopy_language = resources.octool_config["canopy_language"]
            .as_str()
            .unwrap_or("en");
        let mut lang = "_".to_string();
        lang.push_str(&canopy_language);
        lang.push('_');
        let input_resources = Path::new("INPUT/Resources");
        for res in &["Audio", "Font", "Image", "Label"] {
            let in_path = Path::new("resources/OcBinaryData/Resources");
            let out_path = Path::new("OUTPUT/EFI/OC/Resources");
            let mut entries = fs::read_dir(in_path.join(res))?
                .map(|r| r.map(|p| p.path()))
                .collect::<Result<Vec<_>, io::Error>>()?;
            if res == &"Audio" {
                entries.retain(|p| p.to_str().unwrap().contains(&lang));
                let f = Path::new("resources/OcBinaryData/Resources/Audio");
                for file in resources.octool_config["global_audio_files"]
                    .as_array()
                    .unwrap()
                {
                    entries.push(f.join(file.as_str().unwrap()));
                }
            }
            if input_resources.join(res).exists() {
                for r in fs::read_dir(input_resources.join(res))? {
                    entries.push(r?.path());
                }
            }
            let mut s = "";
            if entries.len() > 1 {
                s = "s";
            };
            write!(
                stdout,
                "\x1B[32mCopying\x1B[0m {} {} resource{} from OcBinaryData ... ",
                entries.len(),
                res,
                s
            )?;
            stdout.flush()?;
            fs_extra::copy_items(&entries, out_path.join(res), &options)?;
            write!(stdout, "\x1B[32mdone\x1B[0m\r\n")?;
            stdout.flush()?;
        }
        write!(stdout, "\r\n")?;
        stdout.flush()?;
    }

    match resources.config_plist.as_dictionary().unwrap()["Misc"]
        .as_dictionary()
        .unwrap()["Security"]
        .as_dictionary()
        .unwrap()["Vault"]
        .as_string()
        .unwrap()
    {
        "Basic" => {
            write!(
                stdout,
                "\x1B[32mFound\x1B[0m Misc->Security->Vailt set to Basic\r\n"
            )?;
            compute_vault_plist(resources, stdout)?;
        }
        "Secure" => {
            write!(
                stdout,
                "\x1B[32mFound\x1B[0m Misc->Security->Vault set to Secure\r\n"
            )?;
            compute_vault_plist(resources, stdout)?;
            write!(stdout, "\x1b[32mSigning\x1B[0m OpenCore.efi ... ")?;
            stdout.flush()?;
            let out = status("strings", &["-a", "-t", "d", "OUTPUT/EFI/OC/OpenCore.efi"])?;
            let mut offset = 0;
            for line in String::from_utf8(out.stdout).unwrap().lines() {
                let (off, s) = line.split_once(' ').unwrap();
                if s == "=BEGIN OC VAULT=" {
                    offset = off.parse::<i32>().unwrap() + 16;
                }
            }
            let mut seek = "seek=".to_string();
            seek.push_str(&offset.to_string());
            let _ = status(
                "dd",
                &[
                    "of=OUTPUT/EFI/OC/OpenCore.efi",
                    "if=OUTPUT/EFI/OC/vault.pub",
                    "bs=1",
                    &seek,
                    "count=528",
                    "conv=notrunc",
                ],
            );
            std::fs::remove_file("OUTPUT/EFI/OC/vault.pub")?;
            write!(stdout, "\x1B[32mdone\x1B[0m\r\n\n")?;
            stdout.flush()?;
        }
        _ => (),
    }
    if missing_files.len() > 0 {
        write!(
            stdout,
            "\x1B[31;7mWARNING:\x1B[0m the following file(s) are unknown by octool\x1B[33m\r\n"
        )?;
        for f in missing_files.iter() {
            write!(stdout, "{}\x1B[0K\r\n", f)?;
        }
        write!(
            stdout,
            "\x1B[31mIf you want octool to include them automatically,\r\n"
        )?;
        write!(
            stdout,
            "they need to be placed in the \x1B[32mINPUT\x1B[31m folder before building.\r\n"
        )?;
        write!(
            stdout,
            "Otherwise, they will need to be placed into your EFI manually\x1B[0m\r\n\n"
        )?;
        stdout.flush()?;
    }
    Ok(build_okay)
}

fn compute_vault_plist(
    resources: &Resources,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    write!(stdout, "\x1B[32mComputing\x1B[0m vault.plist ... ")?;
    stdout.flush()?;
    let _ = status(
        &resources
            .open_core_binaries_path
            .join("Utilities/CreateVault/create_vault.sh")
            .to_str()
            .unwrap(),
        &["OUTPUT/EFI/OC/."],
    );
    let _ = status(
        &resources
            .open_core_binaries_path
            .join("Utilities/CreateVault/RsaTool")
            .to_str()
            .unwrap(),
        &[
            "-sign",
            "OUTPUT/EFI/OC/vault.plist",
            "OUTPUT/EFI/OC/vault.sig",
            "OUTPUT/EFI/OC/vault.pub",
        ],
    );
    write!(stdout, "\x1B[32mdone\x1B[0m\r\n")?;
    stdout.flush()?;
    Ok(())
}
