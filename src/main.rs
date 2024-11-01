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

// USB Device support.
use usb_device::{class_prelude::*, prelude::*};


#[entry]
fn main() -> ! {
	// Get access to the RP2040 peripherals.
	let mut pac = pac::Peripherals::take().unwrap();
	let core = pac::CorePeripherals::take().unwrap();

	// Set up the watchdog driver - needed by the clock setup code.
	let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

	// Configure the clocks.
	let clocks = hal::clocks::init_clocks_and_plls(
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

	// Set up the USB driver.
	let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
		pac.USBCTRL_REGS,
		pac.USBCTRL_DPRAM,
		clocks.usb_clock,
		true,
		&mut pac.RESETS,
	));

	let mut _usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x00, 0x00))
		.strings(&[StringDescriptors::default()
			.manufacturer("creatormind")
			.product("SDVX Arcade Controller")
			.serial_number("0000")
		])
		.unwrap()
		.device_class(0x00)
		.build();

	let mut controller = init(pins);

	loop {
		controller.update_buttons(&core.SYST);
	}
}
