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

// The macro for interrupt functions.
use bsp::hal::pac::interrupt;

// Shorter alias for the Hardware Abstraction Layer.
use bsp::hal;

// USB Device support.
use usb_device::{class_prelude::*, prelude::*};

// USB Human Interface Device (HID) Class support.
use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid::hid_class::HIDClass;


/// The USB Device Driver (shared with the interrupt).
static mut USB_DEVICE: Option<UsbDevice<hal::usb::UsbBus>> = None;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

/// The USB Human Interface Device (HID) Driver (shared with the interrupt).
static mut USB_HID: Option<HIDClass<hal::usb::UsbBus>> = None;


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
	unsafe {
		USB_BUS = Some(usb_bus);
	}

	let bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

	let usb_hid = HIDClass::new(bus_ref, GamepadReport::desc(), 60);
	unsafe {
		USB_HID = Some(usb_hid);
	}

	// Set up the USB Device.
	let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x00, 0x00))
		.strings(&[StringDescriptors::default()
			.manufacturer("creatormind")
			.product("SDVX Arcade Pico Controller")
			.serial_number("000000")
		])
		.unwrap()
		.device_class(0x00)
		.build();
	unsafe {
		USB_DEVICE = Some(usb_dev);
	}

	unsafe {
		// Enable the USB interrupt.
		pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
	}

	let mut controller = init(pins);

	loop {
		// TODO: Remove example code.

		controller.update_buttons(&core.SYST);

		let report = GamepadReport::new(0b0000_0001, 0, 0);

		submit_report(report)
			.ok()
			.unwrap_or(0);
	}
}


/// Submits a new report to the USB stack.
fn submit_report(report: impl AsInputReport) -> Result<usize, usb_device::UsbError> {
	critical_section::with(|_| unsafe {
		USB_HID.as_mut().map(|hid| hid.push_input(&report))
	})
	.unwrap()
}

/// This function is called whenever the USB hardware generates an interrupt request.
#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
	let usb_dev = USB_DEVICE.as_mut().unwrap();
	let usb_hid = USB_HID.as_mut().unwrap();

	usb_dev.poll(&mut [usb_hid]);
}
