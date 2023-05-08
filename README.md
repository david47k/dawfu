
# Da Watch Face Uploader (dawfu)
Face Uploader for MO YOUNG / DA FIT Smart Watches

Uses Bluetooth LE (via [btleplug](https://docs.rs/btleplug/latest/btleplug/)). 

It should build on btleplug's supported platforms (Windows 10, macOS, and Linux).

Make sure to disconnect your watch first (it will not work if it already has a bluetooth connection). Easiest to just turn off bluetooth on your phone.

## Usage
```
dawfu: Da Watch Face Uploader - Face Uploader for MO YOUNG / DA FIT Smart Watches
usage: dawfu mode [options] [filename]
mode:        info                        Show device information.
             upload                      Upload a binary watch file.
             help                        Show this help information.
options:     name=MyWatch                Limit to devices with matching name.
             address=01:23:45:67:89:ab   Limit to devices with matching address.
             verbosity=1                 Set debug message verbosity.
             adapter=1                   Select which bluetooth adapter to use.
filename:                                File to upload.
````

e.g.
```
dawfu upload 1234.bin
```

## License

MIT License.
