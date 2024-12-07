<h1 align="center">
    <br />
    <div><img src="icon.svg" alt="Pico SDVX" width="200"  /></div>
    <br />
    Pico SDVX Arcade Controller
    <br />
</h1>

<h4 align="center">
    Firmware for a Sound Voltex arcade-sized controller using a Raspberry Pi Pico.
    <br />
    Written in <a href="https://www.rust-lang.org/" target="_blank">Rust</a>.
</h4>

<p align="center">
    <a href="https://github.com/creatormind-dev/pico-sdvx-ac/releases/latest"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/creatormind-dev/pico-sdvx-ac?color=green"></a>
    <img alt="GitHub Downloads (all assets, latest release)" src="https://img.shields.io/github/downloads/creatormind-dev/pico-sdvx-ac/total?color=blue">
    <img alt="GitHub License" src="https://img.shields.io/github/license/creatormind-dev/pico-sdvx-ac">
    <a href="https://buymeacoffee.com/creatormind"><img alt="Donate" src="https://img.shields.io/badge/%24-donate-bb5794"></a>
</p>

<p align="center">
    <a href="#features">Features</a> •
    <a href="#download">Download</a> •
    <a href="#to-do">To-Do</a> •
    <a href="#support">Support</a> •
    <a href="#license">License</a>
</p>


## Features

- Capable of handling 7 buttons and 2 encoders.
- Device is recognized as an HID-compliant game controller.
- 1000Hz polling rate (1ms latency).
- Two optional debouncing modes for the switches: eager and deferred.
    - Configurable debounce duration in microseconds.
- Reversible encoders with optional debouncing.
- Encoder logic handled by a PIO core (less CPU overhead).

## Download

If you are only looking to flash the Pico with the firmware, using its default configuration,
you can follow these instructions:

1. Download the firmware from the [latest release](https://github.com/creatormind-dev/pico-sdvx-ac/releases/latest)
   (the `.uf2` file).
2. Hold down the `BOOTSEL` button on the Pico when plugging it in.
3. Drag and drop the `.uf2` into the RPI-RP2 "storage device".
4. Your Pico should now be registering as an HID-compliant game controller.

If you want to modify some of the default configurations and/or tweak some settings to better
match your controller's layout or gaming preferences you can follow these instructions:

1. Install [the Rust toolchain](https://www.rust-lang.org/tools/install) and add it to PATH.
2. Add the RP2040/Pico as a build target:
```
rustup target add thumbv6m-none-eabi
```
3. Install the ELF to UF2 Rust tool:
```
cargo install elf2uf2-rs
```
4. [Manually set up the development environment for the Pico](https://datasheets.raspberrypi.com/pico/getting-started-with-pico.pdf#manual-env).
5. Add the Pico SDK to PATH.
6. Clone the repo or download the project code as a `.zip` file from the [latest release](https://github.com/creatormind-dev/pico-sdvx-ac/releases/latest).
7. Open the project and tweak the code to fit your needs. You can change the controller
   configuration in the [main.rs](src/main.rs) file. Additionally, you can tweak other
   parameters in the [controller.rs](src/controller.rs) file.
8. Once you have configured the controller, connect the Pico in `BOOTSEL` mode.
9. Open a terminal at the root of the project and build the project with `cargo build --release`.
   Then, upload it to the Pico with `cargo run --release`.
10. Your Pico should now be registered as an HID-compliant game controller.

## To-Do

This is a list of features to implement / issues to be resolved:

- [ ] Add Keyboard and Mouse HID reporting mode.
- [ ] Add an "idle" lighting mode.
- [ ] Allow disabling lighting.
- [ ] Implement DMA for enhanced performance.

## Support

<a href="https://buymeacoffee.com/creatormind" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/purple_img.png" alt="Buy Me A Coffee" style="height: 41px !important;width: 174px !important;box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;-webkit-box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;" ></a>

## License

This project is licensed under the **GNU General Public License v3** - see the [LICENSE](LICENSE.md) file for more details.

---

> GitHub [@creatormind-dev](https://github.com/creatormind-dev) &nbsp; &middot; &nbsp;
> Bluesky [@creatormind.bsky.social](https://bsky.app/profile/creatormind.bsky.social)
