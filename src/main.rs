#![no_std]
#![no_main]
#![allow(static_mut_refs)]

// Ensures that the program is halted on panic.
extern crate panic_halt;

use pico_sdvx_ac::*;

// The "rp_pico" crate is a Board Support Package for the RP2040 Hardware Abstraction Layer.
// Whenever the "bsp" alias is used, it is directly referencing the rp_pico crate.
use rp_pico as bsp;

// The macro for the start-up function.
use bsp::entry;

// Shorter alias for the Hardware Abstraction Layer.
use bsp::hal;

// Shorter alias for the Peripheral Access Crate.
use hal::pac;

use hal::Timer;
use hal::pio::PIOExt;

// The macro for interrupt functions.
use pac::interrupt;

use pio_proc::pio_file;

// USB Device support.
use usb_device::{class_prelude::*, prelude::*};

// USB Human Interface Device (HID) Class support.
use usbd_hid::descriptor::generator_prelude::SerializedDescriptor;
use usbd_hid::hid_class::{HIDClass, ReportInfo, ReportType};


/// The USB Device Driver (shared with the interrupt).
static mut USB_DEV: Option<UsbDevice<hal::usb::UsbBus>> = None;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

/// The USB HID Joystick Driver (shared with the interrupt).
static mut HID_JOY: Option<HIDClass<hal::usb::UsbBus>> = None;

/// The USB HID Lighting Driver (shared with the interrupt).
static mut HID_LED: Option<HIDClass<hal::usb::UsbBus>> = None;


#[entry]
fn main() -> ! {
	// Get access to the RP2040 peripherals.
	let mut pac = pac::Peripherals::take().unwrap();

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

	let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

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
	unsafe { USB_BUS = Some(usb_bus) };

	let bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

	let hid_joy = HIDClass::new_ep_in(bus_ref, JoystickReport::desc(), USB_POLL_RATE_MS);
	unsafe { HID_JOY = Some(hid_joy) };

	let hid_led = HIDClass::new_ep_out(bus_ref, LightingReport::desc(), USB_POLL_RATE_MS);
	unsafe { HID_LED = Some(hid_led) };

	// Set up the USB Device.
	let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x00, 0x00))
		.strings(&[StringDescriptors::default()
			.manufacturer("creatormind")
			.product("Pico SDVX Controller")
			.serial_number("000000")
		])
		.unwrap()
		.device_class(0x00)
		.build();
	unsafe { USB_DEV = Some(usb_dev) };

	unsafe {
		// Enable the USB interrupt.
		pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
	}

	// Retrieves the PIO0 and two of its state machines.
	let (mut pio0, sm0, sm1, ..) = pac.PIO0.split(&mut pac.RESETS);

	// Parses and installs the encoder program into the PIO.
	let program = pio_file!("./pio/encoders.pio");
	let installed = pio0.install(&program.program).unwrap();
	
	SDVXController::init(pins, timer);

	// Retrieves the controller instance.
	let controller = SDVXController::get_mut().unwrap();

	controller.start(&installed, sm0, sm1);

	loop {
		controller.update();

		let report = controller.report();

		submit_report(&report.to_bytes())
			.ok()
			.unwrap_or(0);
	}
}


/// Submits a new report to the USB stack.
fn submit_report(report: &[u8]) -> Result<usize, UsbError> {
	critical_section::with(|_| unsafe {
		HID_JOY.as_mut().map(|hid| hid.push_raw_input(report))
	})
	.unwrap()
}

fn handle_report(info: ReportInfo, buffer: &[u8]) {
	if info.report_id == HID_LIGHTING_REPORT_ID 
		&& info.len >= HID_LIGHTING_SIZE
		&& info.report_type == ReportType::Output
	{
		critical_section::with(|_| {
			let controller = SDVXController::get_mut().unwrap();
			let report = LightingReport::from_bytes(buffer);

			controller.update_lights(report);
		});
	}
}

/// This function is called whenever the USB hardware generates an interrupt request.
#[interrupt]
unsafe fn USBCTRL_IRQ() {
	let usb_dev = USB_DEV.as_mut().unwrap();
	let hid_joy = HID_JOY.as_mut().unwrap();
	let hid_led = HID_LED.as_mut().unwrap();

	if usb_dev.poll(&mut [hid_joy, hid_led]) {
		let mut buffer = [0u8; HID_JOYSTICK_SIZE];
		
		if let Some(info) = hid_led.pull_raw_report(&mut buffer).ok() {
			handle_report(info, &buffer);
		}
	}
}
