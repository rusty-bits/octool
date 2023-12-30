# octool

[![license](https://img.shields.io/github/license/rusty-bits/octool.svg)](https://github.com/rusty-bits/octool/blob/main/LICENSE)
[![downloads](https://img.shields.io/github/downloads/rusty-bits/octool/total)](https://github.com/rusty-bits/octool/releases)
[![CI](https://github.com/rusty-bits/octool/actions/workflows/ci.yml/badge.svg)](https://github.com/rusty-bits/octool/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/rusty-bits/octool?display_name=tag)](https://github.com/rusty-bits/octool/releases)

A small project to help me continue my learning of the Rust language.  All suggestions and criticisms are welcome (but that doesn't mean I'll get to them in a timely manner, I can be lazy at times)  
You can build from the included source by running `cargo build --release` (if you have the Rust environment installed) or you can use the binary from [the Releases section on GitHub](https://github.com/rusty-bits/octool/releases).  Unlike my older OC-tool, this tool does not auto update itself to the latest version, but it will let you know if there is an update.  

You can find a basic picture guide for the use of octool here [https://rusty-bits.github.io/octool/](https://rusty-bits.github.io/octool/)  

## Command line options ##  

./octool [options] [-V x.y.z] [INPUT_folder || config.plist]  

-d  use `debug` versions for EFI instead of `release` versions  

-h  print help/usage message then exit  

-v  print octool version information and booted OpenCore version if the var is in NVRAM then exit  

-V x.y.z  select OpenCore version number to use e.g. `-V 0.9.7`  
 - without this option octool will make a quick guess as to which version to use based on the INPUT config.plist, if no INPUT config.plist is provided, octool will default to the latest OpenCore version  

octool takes a path to a folder whos name contains `INPUT` at any point.  This folder contains a config.plist and additional files for a specific build which allows the user to have numerous differing configs.  octool will also take a direct path to a specific `config.plist` to use if desired and will gather what is needed for that specific config in the generic `INPUT` folder
If you run octool with no path provided `./octool` will look for `config.plist` in the generic `INPUT` folder, if it doesn't find it there it will use the `OpenCorePkg/Docs/Sample.plist` file.  

At startup, octool checks for a local copy of [the builds branch of the Dortania/build-repo](https://github.com/dortania/build-repo/tree/builds) so it will know the urls and hashes of the prebuilt binary resources.  Thank you [dhinakg](https://github.com/dhinakg), [hieplpvip](https://github.com/hieplpvip), and [khronokernel](https://github.com/khronokernel).  
 - It will update or download it to the `build-repo` into the `tool_config_files` folder as needed.  

Next, octool does the same thing for [the master branch of the Acidanthera OpenCorePkg source files](https://github.com/acidanthera/OpenCorePkg), thanks to the [people of Acidanthera](https://github.com/acidanthera), in order to have the corresponding Sample.plist and Configuration.tex files, etc. for the version of OpenCore that you are building.  They will be placed into the `resources` folder along with the corresponding binaries from the Dortania builds.  This will allow `octool` to use Acidanthera tools while building the EFI, such as the ocvalitate and CreateVault tools.   Thanks, again [dhinakg](https://github.com/dhinakg).  

Lastly, octool will run the input config.plist through ocvalitade and give you the option to quit or continue.  
If you continue you then enter the  

## config.plist editor... ##  
```
Navigation: arrow keys or some standard vi keys
          'up'/'k'            jump to top of section
              ^                       't'
              |                        ^
'left'/'h' <-- --> 'right'/'l'         |
              |                        v
              v                       'b'
          'down'/'j'          jump to bottom of section
```
Usage:  
'ENTER' will switch to edit mode for string, integer, or data fields.  When editing a data field 'TAB' will toggle between editing the data as hex or as a string.  
 - 'ENTER' will save any changes made  
 - 'ESC' will discard and changes  
 - if the field being edited has values listed in the `Configuration.tex` file, octool will display a list of them  

'SPACE' will toggles boolean value between true/false  
- 'SPACE' will also toggle the Enabled status of kexts, drivers, tools, and amls when they are highlighted in the section list  
- and will toggle binary values for fields that have bit values listed in the `Configuration.tex` file  

'a' `add` - if in a resource section there is option to select from a list of known resources, or add a blank resource template to the working `plist` from the `Sample.plist`  
 - if in another section you can select a type and key name to add to the working plist  

'ctrl-c' `copy` - copy the highlighted field or section  

'd' `delete` - will delete the highlighted field or section after confirmation (`dd` command).  The deleted data can be replaced by using the 'p' paste command  

'f' `find` - find all occurances of a string in the plist  
- if there is only one occurance, it will jump to the location  
- if there is more than one occurance, it will present a list to select from  
- 'n' can be used to go to the next item without needing to do another find command  

'G' `go` (capital G) - make an OUTPUT/EFI/OC folder from the config.plist  
 - if `OpenCanopy.efi` is enabled it will copy the OcBinaryData Resources to `OUTPUT/EFI/OC/Resources`  
 - if `Misc > Security > Vault` is set to `Basic` or `Secure`, octool will compute the required files and sign the `OpenCore.efi` if needed  
 - octool will ignore resources that it doesn't know unless they are placed in the INPUT folder, it will print out a warning, but it will not make a change to the config.plist for the unknown resource  
 - any file placed in the `INPUT` folder will take priority and will be used for the `OUTPUT/EFI`, even if a more recent version of that resource is available elsewhere. This is good for using a specific version of a kext, for example, or for using a specific SSDT or USBMap  
 - lastly, it will again validate the `OUTPUT/EFI/OC/config.plist` file with ocvalidate  

'i' show `info` of highlighted item.  
 - If item is resource such as a kext or driver, octool will show the source of the file it will place in the `OUTPUT/EFI` folder.  
 - Otherwise, octool will show the description and info from the corresponding [Acidanthera](https://github.com/acidanthera) `Configuration.tex` file.  

'I' - Capital I - `Insert` - enter the path to a plist file, or drop it on the window, and octool will add the fields from that plist  
 - useful to add a plist that just contains Patches, for example the `patches_OC.plist` file created by  [CorpNewt's](https://github.com/corpnewt) SSDTTime tool  

'K' `Key` - capital K - edit the name of the highlighted key  

'M' `merge` - capital M - will add missing fields to the `config.plist` from the `Sample.plist` without changing any existing fields.  
 - this command, coupled with its companion Purge command (capital P) will update a config.plist when OpenCore plist format changes occur  

'n' `next` - jump to the next found item if more than one occurance was found  

'O' `order` - Capital O - if currently in the Kernel > Add section the 'O' command will check the order and dependencies of kexts.  
 - If a kext is in the wrong order based on a dependency then octool will reorder them.  
 - If a required dependency is missing then octool will add and enable the required dependency.  
 - If there are any duplicate enabled kexts then octool will disable the duplicates.  

'P' `purge` - Capital P - removes fields from the `config.plist` that are not in the `Sample.plist`  
 - this command, coupled with it's companion merge command (capital M) will update a config.plist when OpenCore plst format changes occur  

'p' `paste` - places the last deleted or modified etc. item into the plist (for those familiar with vi commands)  
 - if pasting into a dictionary, `octool` will append `-copy` to the pasted item  

'q' `quit` - if unsaved changes were made to the `config.plist` octool will show a warning so changes can be saved  

'r' `reset` - if a single item is selected, reset its value to the same as the `Sample.plist` value  
 - if a section is highlighted, reset the whole section to the same as the section in the `Sample.plist`  

's' `save` a copy of the `config.plist` as `INPUT/modified_config.plist`  
 - `modified_` will be added to the begining of the saved file unless you are already working on a `modified_` file  
 - the saved file will be checked with `ocvalidate` for any errors  

'V' `Version` - Capital V - change the version of OpenCore that will be checked against and used in the `OUTPUT` EFI  
 - or, if 'V' is used while a resource is highlighted, you can change the version of that specific resource  

'ctrl-x' `cut` - remove the highlighted field or section from the plist  

'y' `yank` - copy the highlighted field or section (included for those vim users used to 'y' for copying)  

'ctrl-v' `paste` - place the last cut, copied, etc. item into the plist  

## File and Folder Descriptions ##  
`tool_config_files` folder - contains various json formatted files  
 - `octool_config.json` - settings for octool itself, octool will create this if it doesn't exist    
 - `resource_list.json` - list of resources by full name e.g. `Lilu.kext` and their parent resource, octool will create this if it doesn't exist    
 - `build-repo` folder - contains the `config.json` file from the Dortania builds repo with url, version, hash, date created, etc. info for the parent resources. octool will download this from Dortania if it doesn't exist    
 - `other.json` - contains a list of additional parent resources not included in the Dortania `build--repo`, octool will create this if it doesn't exist  

`INPUT` folder - place your `config.plist` here along with other files to be included in the `OUTPUT/EFI`, such as custom SSDT files, custom Drivers, custom OpenCanopy themes, etc.  
 - `octool` will not overwrite the input config.plist on save, instead it will save a version called `modified_config.plist` in this folder so the original `config.plist` can still be used if needed  
 - `octool` will also automatically save a config.plist titled `last_built_config.plist` when the build command is run for easy reference to a copy of the config.plist that is in the OUTPUT/EFI folder  

`OUTPUT` folder - location where `octool` will put the created `EFI` folder 

`resources` folder - location where `octool` places the resources needed to create the `OUTPUT/EFI` folder. Can be deleted if desired, octool will gather any resources it needs when run  

## To Do: ##  
 - change tool configuration from inside tool, the configuration file `tool_config_files/octool_config.json` contains vars to set up octool, for example the language versions of the audio files for OpenCanopy for e.g. `en`  
 - keep the highlighted item on screen while reading long info from the Configuration.tex so the user can edit the field while also reading the info  
 - fix some style formatting of the info screens so underline, bold, etc. looks better when it crosses multiple lines  
