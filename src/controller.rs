use rp_pico as bsp;

use bsp::hal;
use hal::gpio;
use hal::pac;
use hal::pio;

use crate::*;

/// These are traits for handling digital pin states.
use embedded_hal::digital::{InputPin, OutputPin};

/// Represents a duration in microseconds.
use hal::fugit::MicrosDurationU64;


/// The amount of LEDs in the controller.
pub const LED_GPIO_SIZE: u8 = 7;
/// The amount of micro switches in the controller.
pub const SW_GPIO_SIZE: u8 = 7;
/// The amount of encoders on the controller.
pub const ENC_GPIO_SIZE: u8 = 2;
/// The resolution of the encoders in a pulses per revolution metric.
pub const ENC_PPR: i32 = 360;
/// The number of pulses needed to complete a full revolution.
/// Alias the number of reports per revolution.
pub const ENC_PULSE: i32 = ENC_PPR * 4;

/// The debounce algorithm to use for debouncing the micro switches.
pub const DEBOUNCE_MODE: DebounceMode = DebounceMode::Hold;
/// Whether to debounce the encoders.
pub const DEBOUNCE_ENCODERS: bool = false;
/// Whether to reverse the direction of the encoders.
pub const REVERSE_ENCODERS: (bool, bool) = (true, true); // (VOL-L, VOL-R)
/// The duration (in microseconds) to wait for before using a fallback lighting mode.
pub const REACTIVE_TIMEOUT_US: u64 = 3000;
/// Fallback mode to use when the reactive HID lighting hasn't reported data.
pub const FALLBACK_LIGHTING_MODE: FallbackLightingMode = FallbackLightingMode::Reflective;
/// The duration (in microseconds) for debouncing the micro switches.
pub const SW_DEBOUNCE_DURATION_US: u64 = 8000;

/// The interval at which the controller polls the inputs to the host.
/// * Higher values produce more latency, but generate less CPU stress.
/// * Lower values generate a quicker response, but may cause the controller to become unstable.
pub const HID_JOYSTICK_POLL_RATE_MS: u8 = 2;
/// The interval at which the controller polls the lights from the host.
/// There is no need to lower this value, as it will put more stress on the CPU.
pub const HID_LIGHTING_POLL_RATE_MS: u8 = 60;


static mut CONTROLLER: Option<SDVXController> = None;


// The GPIO pin order for the micro switches and LEDs is as follows:
// [START] -> [BT-A] -> [BT-B] -> [BT-C] -> [BT-D] -> [FX-L] -> [FX-R]

// The GPIO pin order for the encoders is as follows:
// [VOL-L (A, B)] -> [VOL-R (A, B)]


// TODO: Add Keyboard and Mouse reporting mode.
/// Sound Voltex controller.
pub struct SDVXController {
	leds: [Led; LED_GPIO_SIZE as _],
	switches: [Switch; SW_GPIO_SIZE as _],
	encoders: [Encoder; ENC_GPIO_SIZE as _],
	
	joystick: JoystickReport,

	rx_l: Option<pio::Rx<pio::PIO0SM0>>,
	rx_r: Option<pio::Rx<pio::PIO0SM1>>,

	last_timeout: hal::timer::Instant,
	
	timer: hal::Timer,
}

impl SDVXController {
	/// Initializes the components used by the controller.
	pub fn init(pins: bsp::Pins, timer: hal::Timer) {
		// Abort if the controller has already been initialized.
		if unsafe { CONTROLLER.is_some() } { return; }

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
	
		let enc_l_pin_a: DynPio0Pin = pins.gpio14.reconfigure().into_dyn_pin();
		let enc_l_pin_b: DynPio0Pin = pins.gpio15.reconfigure().into_dyn_pin();
		let enc_r_pin_a: DynPio0Pin = pins.gpio16.reconfigure().into_dyn_pin();
		let enc_r_pin_b: DynPio0Pin = pins.gpio17.reconfigure().into_dyn_pin();
	
		/* ~~ GPIO/PINOUT CONFIGURATION END ~~ */

		let leds: [Led; LED_GPIO_SIZE as _] = [
			Led::new(led_start_pin),
			Led::new(led_bt_a_pin),
			Led::new(led_bt_b_pin),
			Led::new(led_bt_c_pin),
			Led::new(led_bt_d_pin),
			Led::new(led_fx_l_pin),
			Led::new(led_fx_r_pin),
		];
		
		let switches: [Switch; SW_GPIO_SIZE as _] = [
			Switch::new(sw_start_pin),					// 0
			Switch::new(sw_bt_a_pin),					// 1
			Switch::new(sw_bt_b_pin),					// 2
			Switch::new(sw_bt_c_pin),					// 3
			Switch::new(sw_bt_d_pin),					// 4
			Switch::new(sw_fx_l_pin),					// 5
			Switch::new(sw_fx_r_pin),					// 6
		];

		let encoders: [Encoder; ENC_GPIO_SIZE as _] = [
			Encoder::new(enc_l_pin_a, enc_l_pin_b),		// 0
			Encoder::new(enc_r_pin_a, enc_r_pin_b),		// 1
		];

		unsafe {
			CONTROLLER = Some(Self {
				leds,
				switches,
				encoders,
				joystick: JoystickReport::default(),
				rx_l: None,
				rx_r: None,
				last_timeout: timer.get_counter(),
				timer,
			});
		}

		pico_led_pin.set_high().unwrap();
	}

	/// Loads and starts the given PIO program for the encoders. One state machine per encoder.
	pub fn start(
		&mut self,
		program: &pio::InstalledProgram<pac::PIO0>,
		sm0: pio::UninitStateMachine<pio::PIO0SM0>,
		sm1: pio::UninitStateMachine<pio::PIO0SM1>,
	) {
		// Abort if the encoders have already been initialized.
		if self.rx_l.is_some() || self.rx_r.is_some() { return; }

		let enc_l = self.encoders[0].pins();
		let enc_r = self.encoders[1].pins();

		let (sm0, rx0, _) = load_encoder_program(
			unsafe { program.share() },
			sm0,
			enc_l.0.id().num,
			enc_l.1.id().num,
			DEBOUNCE_ENCODERS,
		);

		let (sm1, rx1, _) = load_encoder_program(
			unsafe { program.share() },
			sm1,
			enc_r.0.id().num,
			enc_r.1.id().num,
			DEBOUNCE_ENCODERS,
		);

		// Synchronizes both state machines and starts them at the same time.
		sm0.with(sm1).start();

		self.rx_l = Some(rx0);
		self.rx_r = Some(rx1);
	}

	/// Updates the HID report with the current state of the encoders.
	///
	/// Note: If the [`SDVXController::start`] method hasn't been called, this won't work.
	pub fn update_encoders(&mut self) {
		// Abort if the encoders have not been started.
		if self.rx_l.is_none() || self.rx_r.is_none() { return; }

		let rx_l = self.rx_l.as_mut().unwrap();
		let rx_r = self.rx_r.as_mut().unwrap();

		self.joystick.x = parse_encoder(
			rx_l,
			&mut self.encoders[0].state,
			ENC_PULSE,
			REVERSE_ENCODERS.0,
		);

		self.joystick.y = parse_encoder(
			rx_r,
			&mut self.encoders[1].state,
			ENC_PULSE,
			REVERSE_ENCODERS.1,
		);
	}

	/// Updates the HID report with the current state of the buttons.
	pub fn update_inputs(&mut self) {
		let now = self.timer.get_counter();
		let mut switches = self.switches.each_mut();
		let mut report = 0u8;

		// Button order is reversed to start with the MSD button and end with the LSD button in the report.
		switches.reverse();

		for sw in switches {
			let is_pressed = sw.is_pressed();
			let state = &mut sw.state;

			// If there's no debouncing just check if the button is pressed or not.
			if DEBOUNCE_MODE == DebounceMode::None {
				report = if is_pressed { (report << 1) | 1 } else { report << 1 };
				continue;
			}

			if is_pressed && state.last_pressed == false {
				state.last_debounce_time = Some(now);
			}

			state.last_pressed = is_pressed;

			if let Some(last_debounce_time) = state.last_debounce_time {
				// The amount of time that has elapsed since the button was "pressed", or 0.
				let elapsed = now.checked_duration_since(last_debounce_time)
					.unwrap_or(MicrosDurationU64::micros(0))
					.to_micros();

				// IF reports the button as pressed, ELSE reports otherwise.
				report = match DEBOUNCE_MODE {
					DebounceMode::Hold => {
						if is_pressed || (elapsed <= SW_DEBOUNCE_DURATION_US) { (report << 1) | 1 }
						else { report << 1 }
					}

					DebounceMode::Wait => {
						if is_pressed && (elapsed >= SW_DEBOUNCE_DURATION_US) { (report << 1) | 1 }
						else { report << 1 }
					}

					DebounceMode::None => 0 // Infallible case, already handled.
				};
			}
			else {
				report <<= 1;
			}
		}

		self.joystick.buttons = report;
	}

	// TODO: Allow disabling lighting.
	/// Handles the arcade buttons lighting system.
	pub fn update_lights(&mut self, report: Option<LightingReport>) {
		let now = self.timer.get_counter();
		let elapsed = now.checked_duration_since(self.last_timeout)
			.unwrap_or(MicrosDurationU64::micros(0))
			.to_micros();

		let mut lighting = report.unwrap_or_default();

		// Once the timeout for reactive lighting passes a fallback system is used.
		if elapsed > REACTIVE_TIMEOUT_US {
			match FALLBACK_LIGHTING_MODE {
				// Just turns all LEDs on.
				FallbackLightingMode::Idle => {
					lighting.buttons = [1u8; LED_GPIO_SIZE as _];
				}

				// Reflects inputs.
				FallbackLightingMode::Reflective => {
					for i in 0..lighting.buttons.len() {
						lighting.buttons[i] = (self.joystick.buttons >> i) & 1;
					}
				}

				_ => {}
			}
		}

		// Update the LEDs.
		for (i, led) in self.leds.iter_mut().enumerate() {
			if lighting.buttons[i] == 0 {
				led.off();
			}
			else {
				led.on();
			}
		}

		// Update the last timeout if the method was called via HID reporting.
		if report.is_some() {
			self.last_timeout = now;
		}
	}

	// TODO: Update the function to allow dynamic reporting based on the preferred HID mode.
	/// Generates a new gamepad report based on the current state of the controller.
	pub fn report(&self) -> JoystickReport {
		self.joystick.clone()
	}
	
	/// Retrieves the Controller instance as a mutable reference.
	///
	/// Note: If the [`SDVXController::init`] function has not been called the value will be `None`.
	pub fn get_mut() -> Option<&'static mut Self> {
		unsafe { CONTROLLER.as_mut() }
	}

	/// Retrieves the Controller instance as an immutable reference.
	///
	/// Note: If the [`SDVXController::init`] function has not been called the value will be `None`.
	pub fn get_ref() -> Option<&'static Self> {
		unsafe { CONTROLLER.as_ref() }
	}
}


/// Determines the type of debounce algorithm to use with the buttons.
#[derive(Clone, Copy, PartialEq)]
pub enum DebounceMode {
	/// Disables debouncing.
	None,
	/// Immediately reports when a switch is triggered and holds it for an N amount of time.
	///	Also known as "eager debouncing".
	Hold,
	/// Waits for a switch to output a constant N amount of time before reporting.
	/// Also known as "deferred debouncing".
	Wait,
}


/// Specifies the lighting system to use in the controller if no HID data is reported within a
/// [`REACTIVE_TIMEOUT_US`] duration.
#[derive(Clone, Copy, PartialEq)]
pub enum FallbackLightingMode {
	/// No fallback lighting is used.
	None,
	/// An "idle" lighting mode will be activated.
	Idle,
	/// The lighting system will reflect the inputs entered.
	/// This mode is recommended if the software doesn't support HID reporting.
	Reflective,
}


/// Represents an encoder (knob) on the controller.
pub struct Encoder {
	pin_a: DynPio0Pin,
	pin_b: DynPio0Pin,
	state: EncoderState,
}

impl Encoder {
	/// Associates a new encoder.
	pub fn new(pin_a: DynPio0Pin, pin_b: DynPio0Pin) -> Self {
		Self {
			pin_a,
			pin_b,
			state: EncoderState::default(),
		}
	}

	/// Returns pins A and B of the encoder in a tuple.
	pub fn pins(&self) -> (&DynPio0Pin, &DynPio0Pin) {
		(&self.pin_a, &self.pin_b)
	}
}


#[derive(Default)]
pub struct EncoderState {
	/// The previous value reported by the encoder.
	pub prev_value: u32,
	/// The current delta reported by the encoder.
	pub curr_value: i32,
}


/// Represents a LED on the controller.
struct Led {
	pin: DynOutputPin
}

impl Led {
	/// Associates the given pin to a LED.
	fn new(pin: DynOutputPin) -> Self {
		Self { pin }
	}

	/// Turns the LED on.
	fn on(&mut self) {
		self.pin
			.set_high()
			.unwrap();
	}

	/// Turns the LED off.
	fn off(&mut self) {
		self.pin
			.set_low()
			.unwrap();
	}
}


/// Represents a micro switch in the controller.
struct Switch {
	pin: DynInputPin,
	state: SwitchState,
}

impl Switch {
	/// Associates the given pin as a micro switch.
	fn new(pin: DynInputPin) -> Self {
		Self {
			pin,
			state: SwitchState::default(),
		}
	}

	/// Checks whether the micro switch is pressed.
	fn is_pressed(&mut self) -> bool {
		self.pin
			.is_low()
			.ok()
			.unwrap_or(false)
	}
}


/// Represents the state of a micro switch. For debouncing purposes.
#[derive(Default)]
struct SwitchState {
	/// The last time the switch was pressed.
	last_debounce_time: Option<hal::timer::Instant>,
	/// The last state (pressed or not) the switch had.
	last_pressed: bool,
}


/// Type alias for a non-ID pin with a pull-up input configuration.
pub type DynInputPin = gpio::Pin<gpio::DynPinId, gpio::FunctionSioInput, gpio::PullUp>;
/// Type alias for a non-ID pin with a pull-down output configuration.
pub type DynOutputPin = gpio::Pin<gpio::DynPinId, gpio::FunctionSioOutput, gpio::PullDown>;
/// Type alias for a non-ID pin for use with the PIO0.
pub type DynPio0Pin = gpio::Pin<gpio::DynPinId, gpio::FunctionPio0, gpio::PullUp>;
