use crate::res::{get_or_update_local_parent, get_res_path, status, Resources};

use fs_extra::copy_items;
use fs_extra::dir::{copy, CopyOptions};
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn build_output(resources: &Resources) -> Result<bool, Box<dyn Error>> {
    fs_extra::dir::remove("OUTPUT")?;
    std::fs::create_dir_all("OUTPUT/EFI")?;
    let mut has_open_canopy = false;
    let mut build_okay = true;

    let mut options = CopyOptions::new();
    options.overwrite = true;
    copy(&resources.open_core_pkg.join("X64/EFI"), "OUTPUT", &options)?;
    fs_extra::dir::remove("OUTPUT/EFI/OC/Drivers")?;
    fs_extra::dir::remove("OUTPUT/EFI/OC/Tools")?;
    std::fs::create_dir_all("OUTPUT/EFI/OC/Drivers")?;
    std::fs::create_dir_all("OUTPUT/EFI/OC/Tools")?;
    resources
        .config_plist
        .to_file_xml("OUTPUT/EFI/OC/config.plist")?;

    println!("\x1B[32mCopying\x1B[0m enabled ACPI files ...");
    let mut from_paths = Vec::new();
    let acpis = resources.config_plist.as_dictionary().unwrap()["ACPI"]
        .as_dictionary()
        .unwrap()["Add"]
        .as_array()
        .unwrap();
    for val in acpis {
        let acpi = val.as_dictionary().unwrap();
        if acpi["Enabled"].as_boolean().unwrap() {
            let r = acpi["Path"].as_string().unwrap().split('/').next().unwrap();
            match get_res_path(&resources, r, "ACPI") {
                Some(res) => from_paths.push(res),
                None => {
                    build_okay = false;
                    println!("\x1B[31mERROR: {} not found, skipping\x1B[0m", r);
                }
            }
        }
    }
    from_paths.sort();
    from_paths.dedup();
    copy_items(&from_paths, "OUTPUT/EFI/OC/ACPI", &options)?;
    println!("\x1B[32mdone\x1B[0m\n");

    println!("\x1B[32mCopying\x1B[0m enabled Kexts ...");
    let mut from_paths = Vec::new();
    let kexts = resources.config_plist.as_dictionary().unwrap()["Kernel"]
        .as_dictionary()
        .unwrap()["Add"]
        .as_array()
        .unwrap();
    for val in kexts {
        let kext = val.as_dictionary().unwrap();
        if kext["Enabled"].as_boolean().unwrap() {
            let r = kext["BundlePath"]
                .as_string()
                .unwrap()
                .split('/')
                .next()
                .unwrap();
            match get_res_path(&resources, r, "Kernel") {
                Some(res) => from_paths.push(res),
                None => {
                    build_okay = false;
                    println!("\x1B[31mERROR: {} not found, skipping\x1B[0m", r);
                }
            }
        }
    }
    from_paths.sort();
    from_paths.dedup();
    copy_items(&from_paths, "OUTPUT/EFI/OC/Kexts", &options)?;
    println!("\x1B[32mdone\x1B[0m\n");

    println!("\x1B[32mCopying\x1B[0m enabled Tools ...");
    let mut from_paths = Vec::new();
    let tools = resources.config_plist.as_dictionary().unwrap()["Misc"]
        .as_dictionary()
        .unwrap()["Tools"]
        .as_array()
        .unwrap();
    for val in tools {
        let tool = val.as_dictionary().unwrap();
        if tool["Enabled"].as_boolean().unwrap() {
            let r = tool["Path"].as_string().unwrap().split('/').next().unwrap();
            match get_res_path(&resources, r, "Misc") {
                Some(res) => from_paths.push(res),
                None => {
                    build_okay = false;
                    println!("\x1B[31mERROR: {} not found, skipping\x1B[0m", r);
                }
            }
        }
    }
    from_paths.sort();
    from_paths.dedup();
    copy_items(&from_paths, "OUTPUT/EFI/OC/Tools", &options)?;
    println!("\x1B[32mdone\x1B[0m\n");

    println!("\x1B[32mCopying\x1B[0m enabled Drivers ...");
    let mut from_paths = Vec::new();
    let drivers = resources.config_plist.as_dictionary().unwrap()["UEFI"]
        .as_dictionary()
        .unwrap()["Drivers"]
        .as_array()
        .unwrap();
    for val in drivers {
        let driver = val.as_string().unwrap().to_string();
        if !driver.starts_with('#') {
            if &driver == "OpenCanopy.efi" {
                has_open_canopy = true;
            }
            match get_res_path(&resources, &driver, "UEFI") {
                Some(res) => from_paths.push(res),
                None => {
                    build_okay = false;
                    println!("\x1B[31mERROR: {} not found, skipping\x1B[0m", &driver);
                }
            }
        }
    }
    from_paths.sort();
    from_paths.dedup();
    copy_items(&from_paths, "OUTPUT/EFI/OC/Drivers", &options)?;
    println!("\x1B[32mdone\x1B[0m\n");

    if has_open_canopy {
        println!("\x1B[32mFound\x1B[0m OpenCanopy.efi in UEFI->Drivers");
        let _ = get_or_update_local_parent("OcBinaryData", &resources.acidanthera, "release")?;
        let canopy_language = resources.octool_config["canopy_language"]
            .as_str()
            .unwrap_or("en");
        let mut lang = "_".to_string();
        lang.push_str(&canopy_language);
        lang.push('_');
        let input_resources = Path::new("INPUT/Resources");
/*
        let mut entries = fs::read_dir("resources/OcBinaryData/Resources/Audio")?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        entries.retain(|p| p.to_str().unwrap().contains(&lang));
        let f = Path::new("resources/OcBinaryData/Resources/Audio");
        for file in resources.octool_config["global_audio_files"]
            .as_array()
            .unwrap()
        {
            entries.push(f.join(file.as_str().unwrap()));
        }

        print!(
            "\x1B[32mCopying\x1B[0m {} Audio resources from OcBinaryData ... ",
            entries.len()
        );

        copy_items(&entries, "OUTPUT/EFI/OC/Resources/Audio", &options)?;
        println!("\x1B[32mdone\x1B[0m");
*/
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
                entries.push(
                    fs::read_dir(input_resources.join(res))?
                        .map(|r| r.map(|p| p.path()))
                        .collect::<Result<PathBuf, io::Error>>()?,
                );
            }
            print!(
                "\x1B[32mCopying\x1B[0m {} {} resources from OcBinaryData ... ",
                entries.len(),
                res
            );
            copy_items(&entries, out_path.join(res), &options)?;
            println!("\x1B[32mdone\x1B[0m");
        }
        println!();
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
            println!("\x1B[32mFound\x1B[0m Misc->Security->Vailt set to Basic");
            compute_vault_plist(resources)?;
        }
        "Secure" => {
            println!("\x1B[32mFound\x1B[0m Misc->Security->Vault set to Secure");
            compute_vault_plist(resources)?;
            print!("\x1b[32mSigning\x1B[0m OpenCore.efi ... ");
            io::stdout().flush()?;
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
            println!("\x1B[32mdone\x1B[0m\n");
        }
        _ => (),
    }
    Ok(build_okay)
}

fn compute_vault_plist(resources: &Resources) -> Result<(), Box<dyn Error>> {
    print!("\x1B[32mComputing\x1B[0m vault.plist ... ");
    io::stdout().flush()?;
    let _ = status(
        &resources
            .open_core_pkg
            .join("Utilities/CreateVault/create_vault.sh")
            .to_str()
            .unwrap(),
        &["OUTPUT/EFI/OC/."],
    );
    let _ = status(
        &resources
            .open_core_pkg
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
    println!("\x1B[32mdone\x1B[0m");
    Ok(())
}
