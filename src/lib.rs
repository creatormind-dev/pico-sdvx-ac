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
use embedded_hal::digital::{InputPin, OutputPin};


pub fn init(pins: bsp::Pins) -> Controller {
	let mut pico_led_pin = pins.led.into_push_pull_output();

	/* ~~ GPIO/PINOUT CONFIGURATION START ~~ */

	// Feel free to change the GPIO configuration to best suit your controller layout.
	// ONLY change the GPIOX part of the code according to it's corresponding component.
	// Use the Raspberry Pi Pico's pinout diagram to design your own configuration.
	// https://datasheets.raspberrypi.com/pico/Pico-R3-A4-Pinout.pdf

	// These are the switches of the buttons.

	//                  ->|gpioX|<-
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

	Controller::new(
		button_start,
		button_bt_a,
		button_bt_b,
		button_bt_c,
		button_bt_d,
		button_fx_l,
		button_fx_r
	)
}


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
	fn new(
		start: Button,
		bt_a: Button,
		bt_b: Button,
		bt_c: Button,
		bt_d: Button,
		fx_l: Button,
		fx_r: Button,
	) -> Self {
		Self {
			start,
			bt_a,
			bt_b,
			bt_c,
			bt_d,
			fx_l,
			fx_r,
		}
	}

	// TODO: Implement button handling.
	// TODO: Implement encoder handling.
}


// TODO: Create Encoder struct.


struct Button {
	switch: Switch,
	led: Led,
}

impl Button {
	fn new(
		sw_pin: Pin<DynPinId, FunctionSioInput, PullUp>,
		led_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
	) -> Self {
		Self {
			switch: Switch::new(sw_pin),
			led: Led::new(led_pin),
		}
	}

	fn update(&mut self) {
		if self.switch.is_pressed() {
			self.led.on();
		}
		else {
			self.led.off();
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
