#![no_std]

use rp_pico as bsp;

use bsp::hal::gpio::{
	Pin,
	PinId,
	FunctionSioInput,
	FunctionSioOutput,
	PullDown,
	PullUp,
};
use embedded_hal::digital::{InputPin, OutputPin};


pub struct MicroSwitch<I: PinId> {
	pin: Pin<I, FunctionSioInput, PullUp>,
}

impl<I: PinId> MicroSwitch<I> {
	pub fn new(pin: Pin<I, FunctionSioInput, PullUp>) -> Self {
		Self { pin }
	}

	pub fn is_pressed(&mut self) -> bool {
		self.pin
			.is_low()
			.unwrap()
	}
}


pub struct LampHolder<I: PinId> {
	pin: Pin<I, FunctionSioOutput, PullDown>,
}

impl<I: PinId> LampHolder<I> {
	pub fn new(pin: Pin<I, FunctionSioOutput, PullDown>) -> Self {
		Self { pin }
	}

	pub fn on(&mut self) {
		self.pin
			.set_high()
			.unwrap();
	}

	pub fn off(&mut self) {
		self.pin
			.set_low()
			.unwrap();
	}
}
