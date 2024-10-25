#![no_std]
#![no_main]

// Ensures that the program is halted on panic.
extern crate panic_halt;

use sdvx_ac_pico::*;

// The "rp_pico" crate is a Board Support Package for the RP2040 Hardware Abstraction Layer.
// Whenever the "bsp" alias is used, it is directly referencing the rp_pico crate.
use rp_pico as bsp;

// The macro for the start-up function.
use bsp::entry;

// Shorter alias for the Peripheral Access Crate.
use bsp::hal::pac;

// Shorter alias for the Hardware Abstraction Layer.
use bsp::hal;


#[entry]
fn main() -> ! {
    // Get access to the RP2040 peripherals.
    let mut pac = pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code.
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks.
    let _clocks = hal::clocks::init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
        .ok()
        .unwrap();

    // Set up the pins.
    let sio = hal::Sio::new(pac.SIO);
    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // TODO: Remove the this code once all the button and led testing has been done.

    let test_btn_pin = pins.gpio2.into_pull_up_input();
    let test_led_pin = pins.gpio3.into_push_pull_output();

    let mut button = MicroSwitch::new(test_btn_pin);
    let mut led = LampHolder::new(test_led_pin);

    loop {
        if button.is_pressed() {
            led.on();
        }
        else {
            led.off();
        }
    }
}
