#![no_std]

pub mod hid_desc;
pub use crate::hid_desc::*;

use rp_pico as bsp;

use bsp::hal::pac;
use bsp::hal;

use embedded_hal::digital::{InputPin, OutputPin};
use hal::fugit::MicrosDurationU64;
use hal::gpio::{
	DynPinId,
	FunctionPio0,
	FunctionSioInput,
	FunctionSioOutput,
	Pin,
	PullDown,
	PullUp,
};
use hal::pio::{
	InstalledProgram,
	PinDir,
	PIO0SM0,
	PIO0SM1,
	PIOBuilder,
	Running,
	Rx,
	ShiftDirection,
	StateMachine,
	StateMachineIndex,
	Tx,
	UninitStateMachine,
};
use hal::timer::{Instant, Timer};


/// The amount of buttons on the controller.
pub const BT_SIZE: usize = 7;
/// The duration (in microseconds) for debouncing the switches.
pub const SW_DEBOUNCE_DURATION_US: u64 = 4000;
pub const ENC_PPR: u32 = 360;
pub const ENC_PULSE: u32 = ENC_PPR * 4;


static mut CONTROLLER: Option<SDVXController> = None;


/// Initializes the pins that are used by the controller.
///
/// Allows access to the Controller instance.
pub fn init_pins(pins: bsp::Pins) {
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

	// These are the encoders GPIO configurations.

	let enc_l_pin_a = pins.gpio14.reconfigure::<FunctionPio0, PullUp>().into_dyn_pin();
	let enc_l_pin_b = pins.gpio15.reconfigure::<FunctionPio0, PullUp>().into_dyn_pin();
	let enc_r_pin_a = pins.gpio16.reconfigure::<FunctionPio0, PullUp>().into_dyn_pin();
	let enc_r_pin_b = pins.gpio17.reconfigure::<FunctionPio0, PullUp>().into_dyn_pin();

	/* ~~ GPIO/PINOUT CONFIGURATION END ~~ */

	let button_start = Button::new(sw_start_pin, led_start_pin);
	let button_bt_a = Button::new(sw_bt_a_pin, led_bt_a_pin);
	let button_bt_b = Button::new(sw_bt_b_pin, led_bt_b_pin);
	let button_bt_c = Button::new(sw_bt_c_pin, led_bt_c_pin);
	let button_bt_d = Button::new(sw_bt_d_pin, led_bt_d_pin);
	let button_fx_l = Button::new(sw_fx_l_pin, led_fx_l_pin);
	let button_fx_r = Button::new(sw_fx_r_pin, led_fx_r_pin);

	let encoder_vol_l = Encoder::new(enc_l_pin_a, enc_l_pin_b);
	let encoder_vol_r = Encoder::new(enc_r_pin_a, enc_r_pin_b);

	// Initializes the controller with the configured pins.
	SDVXController::init(
		button_start,
		button_bt_a,
		button_bt_b,
		button_bt_c,
		button_bt_d,
		button_fx_l,
		button_fx_r,
		encoder_vol_l,
		encoder_vol_r,
	);

	// Turns the integrated LED on once the controller is plugged-in.
	pico_led_pin.set_high().unwrap();
}

fn init_encoder_program<SM: StateMachineIndex>(
	program: InstalledProgram<pac::PIO0>,
	sm: UninitStateMachine<(pac::PIO0, SM)>,
	pin_a: &Pin<DynPinId, FunctionPio0, PullUp>,
	pin_b: &Pin<DynPinId, FunctionPio0, PullUp>,
	debounce: bool,
) -> (StateMachine<(pac::PIO0, SM), Running>, Rx<(pac::PIO0, SM)>, Tx<(pac::PIO0, SM)>) {
	let (mut sm, rx, tx) = PIOBuilder::from_installed_program(program)
		.set_pins(pin_a.id().num, 2)
		.autopull(false)
		.in_pin_base(pin_a.id().num)
		.jmp_pin(pin_b.id().num)
		.in_shift_direction(ShiftDirection::Left)
		.build(sm);

	sm.set_pindirs([
		(pin_a.id().num, PinDir::Input),
		(pin_b.id().num, PinDir::Input),
	]);

	if debounce {
		sm.set_clock_divisor(5000.0);
	}

	(sm.start(), rx, tx)
}

fn parse_encoder<SM: StateMachineIndex>(
	rx: &mut Rx<(pac::PIO0, SM)>,
	state: &mut EncoderState,
) -> u8 {
	if let Some(value) = rx.read() {
		state.curr_value += (value - state.prev_value) as i32;

		while state.curr_value < 0 {
			state.curr_value = (ENC_PULSE as i32) + state.curr_value;
		}

		state.curr_value %= ENC_PULSE as i32;
		state.prev_value = value;
	}

	((state.curr_value as f32 / ENC_PULSE as f32) * (u8::MAX as f32 + 1.0)) as u8
}


/// Sound Voltex controller.
#[allow(unused)]
pub struct SDVXController {
	start: Button,	// 0
	bt_a: Button,	// 1
	bt_b: Button,	// 2
	bt_c: Button,	// 3
	bt_d: Button,	// 4
	fx_l: Button,	// 5
	fx_r: Button,	// 6

	vol_l: Encoder,
	vol_r: Encoder,

	rx0: Option<Rx<PIO0SM0>>,
	rx1: Option<Rx<PIO0SM1>>,

	debounce_encoders: bool,
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
		fx_r: Button,
		vol_l: Encoder,
		vol_r: Encoder,
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
				vol_l,
				vol_r,
				rx0: None,
				rx1: None,
				debounce_encoders: false,
				debounce_mode: DebounceMode::default(),
				gamepad_report: GamepadReport::default(),
			})
		}
	}

	/// Retrieves the Controller instance as a mutable reference.
	///
	/// Note: If the [`init_pins`] function has not been called the value will be `None`.
	pub fn get_mut() -> Option<&'static mut Self> {
		unsafe { CONTROLLER.as_mut() }
	}

	/// Retrieves the Controller instance as an immutable reference.
	///
	/// Note: If the [`init_pins`] function has not been called the value will be `None`.
	pub fn get_ref() -> Option<&'static Self> {
		unsafe { CONTROLLER.as_ref() }
	}

	/// Initializes the dedicated PIO for the encoders.
	pub fn init_encoders(
		&mut self,
		program: InstalledProgram<pac::PIO0>,
		sm0: UninitStateMachine<PIO0SM0>,
		sm1: UninitStateMachine<PIO0SM1>,
	) {
		let shared = unsafe { program.share() };

		let (_, rx0, _) = init_encoder_program(
			program,
			sm0,
			&self.vol_l.pin_a,
			&self.vol_l.pin_b,
			self.debounce_encoders,
		);

		let (_, rx1, _) = init_encoder_program(
			shared,
			sm1,
			&self.vol_r.pin_a,
			&self.vol_r.pin_b,
			self.debounce_encoders,
		);

		self.rx0 = Some(rx0);
		self.rx1 = Some(rx1);
	}

	/// Handles button presses using debouncing (if enabled) and updates the controller's lighting.
	pub fn update(&mut self, timer: &Timer) {
		let buttons_report = self.update_inputs(timer);
		let encoders_report = self.update_encoders();
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
		self.gamepad_report.x = encoders_report.0;
		self.gamepad_report.y = encoders_report.1;
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

	fn update_encoders(&mut self) -> (u8, u8) {
		let mut report = (0u8, 0u8);
		let rx0 = self.rx0.as_mut().unwrap();
		let rx1 = self.rx1.as_mut().unwrap();

		report.0 = parse_encoder(rx0, &mut self.vol_l.state);
		report.1 = parse_encoder(rx1, &mut self.vol_r.state);

		report
	}

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

	/// Sets whether or not to debounce the encoders.
	/// 
	/// Default is `false`.
	pub fn with_debounce_encoders(&mut self, debounce_encoders: bool) -> &mut Self {
		self.debounce_encoders = debounce_encoders;
		self
	}

	/// Sets the debounce mode to use on the buttons.
	/// 
	/// Default is [`DebounceMode::None`].
	pub fn with_debounce_mode(&mut self, debounce_mode: DebounceMode) -> &mut Self {
		self.debounce_mode = debounce_mode;
		self
	}
}


/// Determines the type of debounce algorithm to use with the buttons.
#[derive(Clone, Copy)]
pub enum DebounceMode {
	/// Disables debouncing.
	None,

	/// Immediately reports when a switch is triggered and holds it for an [SW_DEBOUNCE_DURATION_US]
	///	amount of time. Also known as "eager debouncing".
	Hold,

	/// Waits for a switch to output a constant [SW_DEBOUNCE_DURATION_US] amount of time before
	/// reporting. Also known as "deferred debouncing".
	Wait,
}

impl Default for DebounceMode {
	fn default() -> Self {
		Self::None
	}
}


struct Encoder {
	pin_a: Pin<DynPinId, FunctionPio0, PullUp>,
	pin_b: Pin<DynPinId, FunctionPio0, PullUp>,
	state: EncoderState,
}

impl Encoder {
	fn new(
		pin_a: Pin<DynPinId, FunctionPio0, PullUp>,
		pin_b: Pin<DynPinId, FunctionPio0, PullUp>,
	) -> Self {
		Self {
			pin_a,
			pin_b,
			state: EncoderState::default(),
		}
	}
}


#[derive(Default)]
struct EncoderState {
	prev_value: u32,
	curr_value: i32,
}


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
