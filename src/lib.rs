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


pub struct ArcadeButton<MI: PinId, LI: PinId> {
	micro_switch: MicroSwitch<MI>,
	lamp_holder: LampHolder<LI>,
}

impl<MI: PinId, LI: PinId> ArcadeButton<MI, LI> {
	pub fn new(
		micro_switch_pin: Pin<MI, FunctionSioInput, PullUp>,
		lamp_holder_pin: Pin<LI, FunctionSioOutput, PullDown>,
	) -> Self {
		Self {
			micro_switch: MicroSwitch::new(micro_switch_pin),
			lamp_holder: LampHolder::new(lamp_holder_pin),
		}
	}

	pub fn update(&mut self) {
		if self.micro_switch.is_pressed() {
			self.lamp_holder.on();
		}
		else {
			self.lamp_holder.off();
		}
	}
}


struct MicroSwitch<I: PinId> {
	pin: Pin<I, FunctionSioInput, PullUp>,
}

impl<I: PinId> MicroSwitch<I> {
	fn new(pin: Pin<I, FunctionSioInput, PullUp>) -> Self {
		Self { pin }
	}

	fn is_pressed(&mut self) -> bool {
		self.pin
			.is_low()
			.unwrap()
	}
}


struct LampHolder<I: PinId> {
	pin: Pin<I, FunctionSioOutput, PullDown>,
}

impl<I: PinId> LampHolder<I> {
	fn new(pin: Pin<I, FunctionSioOutput, PullDown>) -> Self {
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
