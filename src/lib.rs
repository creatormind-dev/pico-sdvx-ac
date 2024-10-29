#![no_std]

use rp_pico as bsp;

use bsp::hal::gpio::{
	DynPinId,
	Pin,
	FunctionSioInput,
	FunctionSioOutput,
	PullDown,
	PullUp,
};
use cortex_m::peripheral::SYST;
use embedded_hal::digital::{InputPin, OutputPin};


/// The amount of buttons on the controller.
pub const BT_SIZE: usize = 7;
/// The duration (in microseconds) for debouncing the switches.
pub const SW_DEBOUNCE_DURATION_US: u32 = 4000;


/// Initializes the pins that are used by the controller and returns the latter.
pub fn init(pins: bsp::Pins) -> Controller {
	let mut pico_led_pin = pins.led.into_push_pull_output();

	/* ~~ GPIO/PINOUT CONFIGURATION START ~~ */

	// Feel free to change the GPIO configuration to best suit your controller layout.
	// ONLY change the GPIOX part of the code according to it's corresponding component.
	// Use the Raspberry Pi Pico pinout diagram to design your own configuration.
	// https://datasheets.raspberrypi.com/pico/Pico-R3-A4-Pinout.pdf

	// These are the switches of the buttons.

	let sw_bt_a_pin = pins.gpio2.into_pull_up_input().into_dyn_pin();
	let sw_bt_b_pin = pins.gpio4.into_pull_up_input().into_dyn_pin();
	let sw_bt_c_pin = pins.gpio6.into_pull_up_input().into_dyn_pin();
	let sw_bt_d_pin = pins.gpio8.into_pull_up_input().into_dyn_pin();
	let sw_fx_l_pin = pins.gpio10.into_pull_up_input().into_dyn_pin();
	let sw_fx_r_pin = pins.gpio12.into_pull_up_input().into_dyn_pin();
	let sw_start_pin = pins.gpio14.into_pull_up_input().into_dyn_pin();

	// These are the lamp holders/LEDs of the buttons.

	let led_bt_a_pin = pins.gpio3.into_push_pull_output().into_dyn_pin();
	let led_bt_b_pin = pins.gpio5.into_push_pull_output().into_dyn_pin();
	let led_bt_c_pin = pins.gpio7.into_push_pull_output().into_dyn_pin();
	let led_bt_d_pin = pins.gpio9.into_push_pull_output().into_dyn_pin();
	let led_fx_l_pin = pins.gpio11.into_push_pull_output().into_dyn_pin();
	let led_fx_r_pin = pins.gpio13.into_push_pull_output().into_dyn_pin();
	let led_start_pin = pins.gpio15.into_push_pull_output().into_dyn_pin();

	/* ~~ GPIO/PINOUT CONFIGURATION END ~~ */

	let button_bt_a = Button::new(sw_bt_a_pin, led_bt_a_pin);
	let button_bt_b = Button::new(sw_bt_b_pin, led_bt_b_pin);
	let button_bt_c = Button::new(sw_bt_c_pin, led_bt_c_pin);
	let button_bt_d = Button::new(sw_bt_d_pin, led_bt_d_pin);
	let button_fx_l = Button::new(sw_fx_l_pin, led_fx_l_pin);
	let button_fx_r = Button::new(sw_fx_r_pin, led_fx_r_pin);
	let button_start = Button::new(sw_start_pin, led_start_pin);

	// Turns the integrated LED on once the controller is plugged-in.
	pico_led_pin.set_high().unwrap();

	Controller {
		start: button_start,
		bt_a: button_bt_a,
		bt_b: button_bt_b,
		bt_c: button_bt_c,
		bt_d: button_bt_d,
		fx_l: button_fx_l,
		fx_r: button_fx_r
	}
}


/// Sound Voltex controller.
pub struct Controller {
	start: Button,
	bt_a: Button,
	bt_b: Button,
	bt_c: Button,
	bt_d: Button,
	fx_l: Button,
	fx_r: Button,

	// TODO: Add encoder fields.
}

impl Controller {
	/// Handles button presses using debouncing and updates the LEDs in the buttons accordingly.
	pub fn update_buttons(&mut self, syst: &SYST) {
		// Gets the amount of microseconds elapsed since the device booted.
		let current_time = syst.cvr.read();
		let buttons = self.buttons();

		for button in buttons {
			let is_pressed = button.switch.is_pressed();
			let button_state = &mut button.state;

			// Check for debouncing.
			if is_pressed != button_state.last_pressed && (current_time - button_state.last_debounce_time) > SW_DEBOUNCE_DURATION_US {
				button_state.last_debounce_time = current_time;

				if is_pressed {
					button.led.on();
				}
				else {
					button.led.off();
				}

				button_state.last_pressed = is_pressed;
			}
		}
	}

	// TODO: Implement encoder handling.

	fn buttons(&mut self) -> [&mut Button; BT_SIZE] {
		[
			&mut self.bt_a,
			&mut self.bt_b,
			&mut self.bt_c,
			&mut self.bt_d,
			&mut self.fx_l,
			&mut self.fx_r,
			&mut self.start,
		]
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
			switch: Switch::new(sw_pin),
			led: Led::new(led_pin),
			state: ButtonState::default(),
		}
	}
}


struct ButtonState {
	last_pressed: bool,
	last_debounce_time: u32,
}

impl Default for ButtonState {
	fn default() -> Self {
		Self {
			last_pressed: false,
			last_debounce_time: 0,
		}
	}
}


struct Switch {
	pin: Pin<DynPinId, FunctionSioInput, PullUp>,
}

impl Switch {
	fn new(pin: Pin<DynPinId, FunctionSioInput, PullUp>) -> Self {
		Self { pin }
	}

	fn is_pressed(&mut self) -> bool {
		self.pin
			.is_low()
			.unwrap()
	}
}


struct Led {
	pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
}

impl Led {
	fn new(pin: Pin<DynPinId, FunctionSioOutput, PullDown>) -> Self {
		Self { pin }
	}

	fn on(&mut self) {
		self.pin
			.set_high()
			.unwrap();
	}

	fn off(&mut self) {
		self.pin
			.set_low()
			.unwrap();
	}
}
