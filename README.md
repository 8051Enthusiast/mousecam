Some source files and notes from my [mouse cam blog post](https://8051enthusiast.github.io/2020/04/14/003-Stream_Video_From_Mouse.html).

Note that unfortunately, the firmware modifictations to the mouse microcontroller are not in this repo.
That's partly because it contains the original moues firmware everywhere and would therefore run into copyright issues, but it is mostly because it is a giant mess.
It mostly does SPI communication to the adns-9800 and usb reports and shouldn't be too hard to reimplement (since it is unlikely that you have the same mouse with the same firmware revision anyway).

For assembling the `.a51` files, asem51 is required.

The `adns9800` directory contains some random notes on some internal registers/memory locations and a program to encrypt a firmware image for upload.

The `mouse_uc` directory contains some random notes on the configuration protocol, instruction timings and two programs to upload and dump the rom.

The `mousecam` directory contains the program used to transfer images from the adns-9800 to the computer, including the adns-9800 firmware and the host software, but not the mouse uc firmware.

Note that many programs here have a small change of bricking your mouse, so do be careful.
