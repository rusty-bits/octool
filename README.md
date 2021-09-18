w.i.p. octool written in Rust  

A small project to help me learn the Rust language.  All suggestions and criticisms are welcome.  
You can build from the included source by running `cargo build --release` (if you have the Rust environment installed) or you can use the included `octool` binary, which I will try to keep current with the code.


octool currently takes no command line arguments except for a path to a `config.plist` to use if desired.
If you run octool with no path provided `./octool` will first look for a `config.plist` in the `INPUT` folder, if it doesn't find one there it will use the latest `OpenCorePkg/Docs/Sample.plist` file.  


## Here's a rundown of the current process octool uses. ##  

At startup, octool checks for a local copy of [the builds branch of the Dortania/build-repo](https://github.com/dortania/build-repo/tree/builds) so it will know the urls and hashes of the latest binary resources.  Thank you [dhinakg](https://github.com/dhinakg), [hieplpvip](https://github.com/hieplpvip), and [khronokernel](https://github.com/khronokernel).  
 - If it finds it locally, it checks it for updates  
 - If it doesn't find it locally, octool pulls the repo into the tool_config_files folder.  

Next, octool does the same thing for [the master branch of the Acidanthera OpenCorePkg source files](https://github.com/acidanthera/OpenCorePkg) in order to have the latest Sample.plist and Configuration.tex files, etc.  Thanks to the [people of Acidanthera](https://github.com/acidanthera)  

octool then pulls the latest build of the OpenCorePkg from the Dortania builds so it will have compiled tools to use while building the EFI, such as the ocvalitate and CreateVault tools.    
Lastly, octool will run the input config.plist through ocvalitade, display any errors, and give you the option to quit or continue.  
If you continue, you then enter the config.plist editor...  
```
Navigation: arrow keys or standard vi keys
          'up'/'k'            jump to top of section
              ^                       't'
              |                        ^
'left'/'h' <-- --> 'right'/'l'         |
              |                        v
              v                       'b'
          'down'/'j'          jump to bottom of section
```
Usage:  
'i' show info of highlighted item.  
 - If item is resource such as a kext or driver, octool will show the source of the file it will place in the OUTPUT EFI folder.  
 - If the highlighted item is a field of the config.plist, octool will show the relevant description and info from the latest [Acidanthera](https://github.com/acidanthera) Configuration.tex file.  

'TAB/ENTER' will switch to edit mode for string, integer, or data fields. 'TAB' will also toggle between editing a data field as hex or as a string.  
 - 'ENTER' will save any changes made  
 - 'ESC' will discard and changes  

'SPACE' will toggle a boolean value between true/false  
- 'SPACE' will also toggle the Enabled status of kexts, drivers, tools, and amls when they are highlighted in the section list  

'a' add - if in a resource section, add a blank resource template to the working plist from the Sample.plist  
 - if in some other section, select a type and key name to add to the working plist  

'd' will delete the highlighted field or section after confirmation.  The deleted data can be replaced by using the 'p' place command  

'G' go (capital G) - make an OUTPUT/EFI/OC folder from the config.plist  
 - if `OpenCanopy.efi` is enabled it will copy the OcBinaryData Resources to `OUTPUT/EFI/OC/Resources`  
 - if Misc->Security->Vault is set to Basic or Secure, octool will compute the required files and sign the `OpenCore.efi` if needed  
 - right now, octool will ignore resources that it doesn't know about unless they are placed in the INPUT folder, it will print out a warning, but it will not make a change to the config.plist for the unknown resource  
 - any file placed in the `INPUT` folder will take priority and will be used for the `OUTPUT/EFI`, even if a more recent version of that resource is available elsewhere. This is good for using a specific version of a kext, for example, or for using a specific SSDT or USBMap, but I need to have octool print a message as a reminder when it uses files from INPUT  
 - lastly, it will again validate the `OUTPUT/EFI/OC/config.plist` file with ocvalidate  

'p' place - places the last deleted or modified item into the plist  

'P' (capital P) prints out some resource_list.json data for debugging  

'r' reset - if a single item is selected, reset its value to the same as the Sample.plist value  
 - if a section is highlighted, reset the whole section to the same as the Sample.plist  

's' save a copy of the config.plist as INPUT/modified_config.plist  
 - modified_ will be added to the begining of the saved file unless you are already working on a file whose name starts with modified_  

'q' quit without saving  

## To Do: ##  
 - change tool configuration from inside tool, the configuration file `tool_config_files/octool_config.json` contains vars to set up octool, for example using either `release` or `degug` versions of resources, or only copy specific language versions of the audio files for OpenCanopy for example `en`  
 - add the ability to choose which version of OpenCore is used, e.g. `0.7.1 0.7.2 0.7.3` etc., currently octool just uses the latest build from the Dortania builds and then falls back to the latest releases from Acidanthera for anything not included in Dortania  
 - cross compile the tool for windows/linux use, currently only supports running on macOS  
 - add a search function  
 - highlight if the kext/driver/etc exists in the known repos  
