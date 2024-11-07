#![no_std]

pub mod hid_desc;
pub use crate::hid_desc::*;

use rp_pico as bsp;

use bsp::hal::fugit::MicrosDurationU64;
use bsp::hal::timer::{Instant, Timer};
use bsp::hal::gpio::{
	DynPinId,
	Pin,
	FunctionSioInput,
	FunctionSioOutput,
	PullDown,
	PullUp,
};
use embedded_hal::digital::{InputPin, OutputPin};


/// The amount of buttons on the controller.
pub const BT_SIZE: usize = 7;
/// The duration (in microseconds) for debouncing the switches.
pub const SW_DEBOUNCE_DURATION_US: u64 = 4000;


static mut CONTROLLER: Option<SDVXController> = None;


/// Initializes the pins that are used by the controller.
///
/// Allows access to the Controller instance.
pub fn init(pins: bsp::Pins) {
	let mut pico_led_pin = pins.led.into_push_pull_output();

	/* ~~ GPIO/PINOUT CONFIGURATION START ~~ */

	// Feel free to change the GPIO configuration to best suit your controller layout.
	// ONLY change the GPIOX part of the code according to it's corresponding component.
	// Use the Raspberry Pi Pico pinout diagram to design your own configuration.
	// https://datasheets.raspberrypi.com/pico/Pico-R3-A4-Pinout.pdf

	// These are the switches of the buttons.

	let sw_start_pin = pins.gpio0.into_pull_up_input().into_dyn_pin();
	let sw_bt_a_pin = pins.gpio2.into_pull_up_input().into_dyn_pin();
	let sw_bt_b_pin = pins.gpio4.into_pull_up_input().into_dyn_pin();
	let sw_bt_c_pin = pins.gpio6.into_pull_up_input().into_dyn_pin();
	let sw_bt_d_pin = pins.gpio8.into_pull_up_input().into_dyn_pin();
	let sw_fx_l_pin = pins.gpio10.into_pull_up_input().into_dyn_pin();
	let sw_fx_r_pin = pins.gpio12.into_pull_up_input().into_dyn_pin();

	// These are the lamp holders/LEDs of the buttons.

	let led_start_pin = pins.gpio1.into_push_pull_output().into_dyn_pin();
	let led_bt_a_pin = pins.gpio3.into_push_pull_output().into_dyn_pin();
	let led_bt_b_pin = pins.gpio5.into_push_pull_output().into_dyn_pin();
	let led_bt_c_pin = pins.gpio7.into_push_pull_output().into_dyn_pin();
	let led_bt_d_pin = pins.gpio9.into_push_pull_output().into_dyn_pin();
	let led_fx_l_pin = pins.gpio11.into_push_pull_output().into_dyn_pin();
	let led_fx_r_pin = pins.gpio13.into_push_pull_output().into_dyn_pin();

	/* ~~ GPIO/PINOUT CONFIGURATION END ~~ */

	let button_start = Button::new(sw_start_pin, led_start_pin);
	let button_bt_a = Button::new(sw_bt_a_pin, led_bt_a_pin);
	let button_bt_b = Button::new(sw_bt_b_pin, led_bt_b_pin);
	let button_bt_c = Button::new(sw_bt_c_pin, led_bt_c_pin);
	let button_bt_d = Button::new(sw_bt_d_pin, led_bt_d_pin);
	let button_fx_l = Button::new(sw_fx_l_pin, led_fx_l_pin);
	let button_fx_r = Button::new(sw_fx_r_pin, led_fx_r_pin);

	// Initializes the controller with the configured pins.
	SDVXController::init(
		button_start,
		button_bt_a,
		button_bt_b,
		button_bt_c,
		button_bt_d,
		button_fx_l,
		button_fx_r,
	);

	// Turns the integrated LED on once the controller is plugged-in.
	pico_led_pin.set_high().unwrap();
}


/// Sound Voltex controller.
pub struct SDVXController {
	start: Button,	// 0
	bt_a: Button,	// 1
	bt_b: Button,	// 2
	bt_c: Button,	// 3
	bt_d: Button,	// 4
	fx_l: Button,	// 5
	fx_r: Button,	// 6

	// TODO: Add encoder fields.

	debounce_mode: DebounceMode,
	gamepad_report: GamepadReport,
}

impl SDVXController {
	fn init(
		start: Button,
		bt_a: Button,
		bt_b: Button,
		bt_c: Button,
		bt_d: Button,
		fx_l: Button,
		fx_r: Button
	) {
		unsafe {
			CONTROLLER = Some(Self {
				start,
				bt_a,
				bt_b,
				bt_c,
				bt_d,
				fx_l,
				fx_r,
				debounce_mode: DebounceMode::default(),
				gamepad_report: GamepadReport::default(),
			})
		}
	}

	/// Retrieves the Controller instance as a mutable reference.
	///
	/// Note: If the [`init`] function has not been called the value will be `None`.
	pub fn get_mut() -> Option<&'static mut Self> {
		unsafe { CONTROLLER.as_mut() }
	}

	/// Retrieves the Controller instance as an immutable reference.
	///
	/// Note: If the [`init`] function has not been called the value will be `None`.
	pub fn get_ref() -> Option<&'static Self> {
		unsafe { CONTROLLER.as_ref() }
	}

	/// Handles button presses using debouncing (if enabled) and updates the controller's lighting.
	pub fn update(&mut self, timer: &Timer) {
		let buttons_report = self.update_inputs(timer);
		let mut buttons = self.buttons_mut();

		// TODO: Update lighting based on the HID report provided by the host.

		for (i, button) in buttons.iter_mut().enumerate() {
			if (buttons_report << i) & 1 == 1 {
				button.led.on();
			}
			else {
				button.led.off();
			}
		}

		self.gamepad_report.buttons = buttons_report;
	}

	fn update_inputs(&mut self, timer: &Timer) -> u8 {
		// State report for all buttons.
		let mut report = 0u8;
		// Gets the controller's debounce mode before borrowing self.
		let debounce_mode = self.debounce_mode;
		// Gets the amount of time elapsed since the timer was initiated (booted).
		let now = timer.get_counter();
		// Includes all the buttons in an array for easy iteration.
		// (Button order is reversed to properly report the status).
		let mut buttons = self.buttons_mut();

		// Button order is reversed to start with the MSD button (FX_R) and end with the LSD button (START).
		buttons.reverse();

		for button in buttons {
			let is_pressed = button.switch.is_pressed();
			let state = &mut button.state;

			if is_pressed && state.last_pressed == false {
				state.last_debounce_time = Some(now);
			}

			state.last_pressed = is_pressed;

			// The amount of time that has elapsed since the button was "pressed", or 0.
			let elapsed = match state.last_debounce_time {
				Some(last_debounce_time) => {
					now.checked_duration_since(last_debounce_time)
						.unwrap_or(MicrosDurationU64::micros(0))
						.to_micros()
				}

				// This is to avoid registering a press on eager/hold debouncing mode if the button
				// hasn't been pressed before. It does not affect the deferred/wait and none mode.
				None => SW_DEBOUNCE_DURATION_US + 1
			};

			// Debounce checking and reporting.
			report = match debounce_mode {

				// For all cases the if statement reports the button as being pressed,
				// while the else clause reports the opposite.

				DebounceMode::Hold => {
					if is_pressed || (elapsed <= SW_DEBOUNCE_DURATION_US) { (report << 1) | 1 }
					else { report << 1 }
				}

				DebounceMode::Wait => {
					if is_pressed && (elapsed >= SW_DEBOUNCE_DURATION_US) { (report << 1) | 1 }
					else { report << 1 }
				}

				DebounceMode::None => {
					if is_pressed { (report << 1) | 1 }
					else { report << 1 }
				}
			};
		}

		report
	}

	// TODO: Implement encoder handling.

	/// Returns all of the controller's buttons in an array.
	fn buttons_mut(&mut self) -> [&mut Button; BT_SIZE] {
		[
			&mut self.start,
			&mut self.bt_a,
			&mut self.bt_b,
			&mut self.bt_c,
			&mut self.bt_d,
			&mut self.fx_l,
			&mut self.fx_r,
		]
	}

	/// Generates a new report based on the current state of the controller.
	pub fn report(&self) -> GamepadReport {
		GamepadReport::new(
			self.gamepad_report.buttons,
			self.gamepad_report.x,
			self.gamepad_report.y,
		)
	}

	/// Sets the debounce mode to use.
	pub fn with_debounce_mode(&mut self, debounce_mode: DebounceMode) -> &mut Self {
		self.debounce_mode = debounce_mode;
		self
	}
}


/// Determines the type of debounce algorithm to use with the buttons.
#[derive(Clone, Copy)]
pub enum DebounceMode {
	/// Immediately reports when a switch is triggered and holds it for an [SW_DEBOUNCE_DURATION_US]
	///	amount of time. Also known as "eager debouncing".
	Hold,

	/// Waits for a switch to output a constant [SW_DEBOUNCE_DURATION_US] amount of time before
	/// reporting. Also known as "deferred debouncing".
	Wait,

	/// Disables debouncing.
	None,
}

impl Default for DebounceMode {
	fn default() -> Self {
		Self::None
	}
}


// TODO: Create Encoder struct.


struct Button {
	switch: Switch,
	led: Led,
	state: ButtonState,
}

impl Button {
	fn new(
		sw_pin: Pin<DynPinId, FunctionSioInput, PullUp>,
		led_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
	) -> Self {
		Self {
			switch: Switch(sw_pin),
			led: Led(led_pin),
			state: ButtonState::default(),
		}
	}
}


struct ButtonState {
	last_pressed: bool,
	last_debounce_time: Option<Instant>,
}

impl Default for ButtonState {
	fn default() -> Self {
		Self {
			last_pressed: false,
			last_debounce_time: None,
		}
	}
}


struct Switch(Pin<DynPinId, FunctionSioInput, PullUp>);

impl Switch {
	fn is_pressed(&mut self) -> bool {
		self.0
			.is_low()
			.unwrap()
	}
}


struct Led(Pin<DynPinId, FunctionSioOutput, PullDown>);

impl Led {
	fn on(&mut self) {
		self.0
			.set_high()
			.unwrap();
	}

	fn off(&mut self) {
		self.0
			.set_low()
			.unwrap();
	}
}
