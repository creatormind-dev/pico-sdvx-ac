#![no_std]
#![allow(static_mut_refs)]

pub mod controller;
pub mod hid_desc;

pub use crate::controller::*;
pub use crate::hid_desc::*;

use rp_pico as bsp;

use bsp::hal;
use hal::pac;
use hal::pio;


/// Loads the provided encoder program into the specified PIO0 state machine.
/// The user must manage and start the state machine independently.
/// 
/// Returns the configured state machine, the receiver and the transmiter in a tuple.
pub fn load_encoder_program<SM: pio::StateMachineIndex>(
	program: pio::InstalledProgram<pac::PIO0>,
	sm: pio::UninitStateMachine<(pac::PIO0, SM)>,
	pin_a: u8,
	pin_b: u8,
	debounce: bool,
) -> (
	pio::StateMachine<(pac::PIO0, SM), pio::Stopped>,
	pio::Rx<(pac::PIO0, SM)>,
	pio::Tx<(pac::PIO0, SM)>
) {
	// This configuration is usually made in the same PIO file using C.
	// However, Rust can't interoperate with C code, so the configuration is made here.
	// Understanding how this configuration works is essential to understanding the PIO code.

	let (mut sm, rx, tx) = pio::PIOBuilder::from_installed_program(program)
		.set_pins(pin_a, 2)
		.in_pin_base(pin_a)
		.jmp_pin(pin_b)
		.autopull(false)
		.in_shift_direction(pio::ShiftDirection::Left)
		.build(sm);

	sm.set_pindirs([
		(pin_a, pio::PinDir::Input),
		(pin_b, pio::PinDir::Input),
	]);

	if debounce {
		sm.set_clock_divisor(5000.0);
	}

	(sm, rx, tx)
}

// TODO: Figure out a way to use DMA to improve performance (if possible).
/// Reads data from the encoder and updates the delta to report which direction
/// is the encoder spinning.
pub fn parse_encoder<SM: pio::StateMachineIndex>(
	rx: &mut pio::Rx<(pac::PIO0, SM)>,
	state: &mut EncoderState,
	pulse: i32,
	reverse: bool,
) -> u8 {
	let direction = if reverse { -1 } else { 1 };

	// Find the delta between the previous value and the current value and update it.
	if let Some(value) = rx.read() {
		state.curr_value += (value as i32 - state.prev_value as i32) * direction;

		while state.curr_value < 0 {
			state.curr_value = pulse + state.curr_value;
		}

		state.curr_value %= pulse;
		state.prev_value = value;
	}

	((state.curr_value as f64 / pulse as f64) * (u8::MAX as f64 + 1.0)) as u8
}
