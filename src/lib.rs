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


pub struct Button {
	switch: Switch,
	led: Led,
}

impl Button {
	pub fn new(
		sw_pin: Pin<DynPinId, FunctionSioInput, PullUp>,
		led_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
	) -> Self {
		Self {
			switch: Switch::new(sw_pin),
			led: Led::new(led_pin),
		}
	}

	pub fn update(&mut self) {
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
