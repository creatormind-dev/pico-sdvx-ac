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
pub const LED_GPIO_SIZE: usize = 7;
/// The duration (in microseconds) for debouncing the microswitches.
pub const SW_DEBOUNCE_DURATION_US: u64 = 4000;
/// The amount of micro switches in the controller.
pub const SW_GPIO_SIZE: usize = 7;
/// The amount of encoders on the controller.
pub const ENC_GPIO_SIZE: usize = 2;
/// The resolution of the encoders in a pulses per revolution metric.
pub const ENC_PPR: i32 = 360;
/// The number of pulses needed to complete a full revolution.
/// Alias the number of reports per revolution.
pub const ENC_PULSE: i32 = ENC_PPR * 4;
/// The speed at which the controller reports to the host.
/// Higher values produce more latency, but generate less CPU stress.
pub const USB_POLL_RATE_MS: u8 = 1; 


static mut CONTROLLER: Option<SDVXController> = None;


// The GPIO pin order for the microswitches and LEDs is as follows:
// [START] -> [BT-A] -> [BT-B] -> [BT-C] -> [BT-D] -> [FX-L] -> [FX-R]

// The GPIO pin order for the encoders is as follows:
// [VOL-L (A, B)] -> [VOL-R (A, B)]


// TODO: Add Keyboard and Mouse reporting mode.
/// Sound Voltex controller.
pub struct SDVXController {
	leds: [Led; LED_GPIO_SIZE],
	switches: [Switch; SW_GPIO_SIZE],
	encoders: [Encoder; ENC_GPIO_SIZE],

	options: SDVXControllerOptions,
	
	gamepad: GamepadReport,

	rx_l: Option<pio::Rx<pio::PIO0SM0>>,
	rx_r: Option<pio::Rx<pio::PIO0SM1>>,

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

		let leds: [Led; LED_GPIO_SIZE] = [
			Led::new(led_start_pin),
			Led::new(led_bt_a_pin),
			Led::new(led_bt_b_pin),
			Led::new(led_bt_c_pin),
			Led::new(led_bt_d_pin),
			Led::new(led_fx_l_pin),
			Led::new(led_fx_r_pin),
		];
		
		let switches: [Switch; SW_GPIO_SIZE] = [
			Switch::new(sw_start_pin),					// 0
			Switch::new(sw_bt_a_pin),					// 1
			Switch::new(sw_bt_b_pin),					// 2
			Switch::new(sw_bt_c_pin),					// 3
			Switch::new(sw_bt_d_pin),					// 4
			Switch::new(sw_fx_l_pin),					// 5
			Switch::new(sw_fx_r_pin),					// 6
		];

		let encoders: [Encoder; ENC_GPIO_SIZE] = [
			Encoder::new(enc_l_pin_a, enc_l_pin_b),		// 0
			Encoder::new(enc_r_pin_a, enc_r_pin_b),		// 1
		];

		unsafe {
			CONTROLLER = Some(Self {
				leds,
				switches,
				encoders,
				options: SDVXControllerOptions::default(),
				gamepad: GamepadReport::default(),
				rx_l: None,
				rx_r: None,
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
			self.options.debounce_encoders,
		);

		let (sm1, rx1, _) = load_encoder_program(
			unsafe { program.share() },
			sm1,
			enc_r.0.id().num,
			enc_r.1.id().num,
			self.options.debounce_encoders,
		);

		// Synchronizes both state machines and starts them at the same time.
		sm0.with(sm1).start();

		self.rx_l = Some(rx0);
		self.rx_r = Some(rx1);
	}

	/// Wrapper for update input methods. It is recommended to call this method instead of
	/// calling each update method individually.
	pub fn update(&mut self) {
		self.update_encoders();
		self.update_inputs();
	}

	/// Updates the HID report with the current state of the encoders.
	///
	/// Note: If the [`SDVXController::start`] method hasn't been called, this won't work.
	pub fn update_encoders(&mut self) {
		// Abort if the encoders have not been started.
		if self.rx_l.is_none() || self.rx_r.is_none() { return; }

		let rx_l = self.rx_l.as_mut().unwrap();
		let rx_r = self.rx_r.as_mut().unwrap();
		let reverse = self.options.reverse_encoders.state();

		self.gamepad.x = parse_encoder(
			rx_l,
			&mut self.encoders[0].state,
			ENC_PULSE,
			reverse.0,
		);

		self.gamepad.y = parse_encoder(
			rx_r,
			&mut self.encoders[1].state,
			ENC_PULSE,
			reverse.1,
		);
	}

	/// Updates the HID report with the current state of the buttons.
	pub fn update_inputs(&mut self) {
		let now = self.timer.get_counter();
		let debounce_mode = self.options.debounce_mode;
		let debounce_duration = self.options.debounce_duration;
		let mut switches = self.switches.each_mut();
		let mut report = 0u8;

		// Button order is reversed to start with the MSD button and end with the LSD button in the report.
		switches.reverse();

		for sw in switches {
			let is_pressed = sw.is_pressed();
			let state = &mut sw.state;

			// If there's no debouncing just check if the button is pressed or not.
			if debounce_mode == DebounceMode::None {
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
					.unwrap_or(MicrosDurationU64::micros(0));

				// IF reports the button as pressed, ELSE reports otherwise.
				report = match debounce_mode {
					DebounceMode::Hold => {
						if is_pressed || (elapsed <= debounce_duration) { (report << 1) | 1 }
						else { report << 1 }
					}

					DebounceMode::Wait => {
						if is_pressed && (elapsed >= debounce_duration) { (report << 1) | 1 }
						else { report << 1 }
					}

					DebounceMode::None => 0 // Infallible case, already handled.
				};
			}
			else {
				report <<= 1;
			}
		}

		self.gamepad.buttons = report;
	}

	// TODO: Add an "idle" lighting mode.
	// TODO: Allow disabling lighting.
	/// Handles the arcade buttons lighting system.
	pub fn update_lights(&mut self) {
		for (i, led) in self.leds.iter_mut().enumerate() {
			if (self.gamepad.buttons >> i) & 1 == 1 {
				led.on();
			}
			else {
				led.off();
			}
		}
	}

	// TODO: Update the function to allow dynamic reporting based on the preferred HID mode.
	/// Generates a new gamepad report based on the current state of the controller.
	pub fn report_gamepad(&self) -> GamepadReport {
		self.gamepad.clone()
	}

	/// Retrieves the controller's current options. Options can be chained for easier modification.
	pub fn options(&mut self) -> &mut SDVXControllerOptions {
		&mut self.options
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


/// Provides various configurations as to how the controller will operate.
pub struct SDVXControllerOptions {
	debounce_encoders: bool,
	debounce_duration: MicrosDurationU64,
	debounce_mode: DebounceMode,
	reverse_encoders: ReverseMode,
}

impl SDVXControllerOptions {
	/// Sets whether to debounce the encoders.
	///
	/// Default is `false`.
	pub fn with_debounce_encoders(&mut self, debounce_encoders: bool) -> &mut Self {
		self.debounce_encoders = debounce_encoders;
		self
	}

	/// Sets the duration to compare against when debouncing the buttons.
	/// The value must be in microseconds.
	///
	/// Default is [`SW_DEFAULT_DEBOUNCE_DURATION_US`].
	pub fn with_debounce_duration(&mut self, debounce_duration_us: u64) -> &mut Self {
		self.debounce_duration = MicrosDurationU64::micros(debounce_duration_us);
		self
	}

	/// Sets the debounce mode to use on the buttons.
	///
	/// Default is [`DebounceMode::None`].
	pub fn with_debounce_mode(&mut self, debounce_mode: DebounceMode) -> &mut Self {
		self.debounce_mode = debounce_mode;
		self
	}

	/// Sets whether any of the encoders should reverse its direction.
	/// 
	/// Default is [`ReverseMode::None`].
	pub fn with_reverse_encoders(&mut self, reverse_encoders: ReverseMode) -> &mut Self {
		self.reverse_encoders = reverse_encoders;
		self
	}

	pub fn debounce_encoders(&self) -> bool {
		self.debounce_encoders
	}

	pub fn debounce_duration(&self) -> MicrosDurationU64 {
		self.debounce_duration
	}

	pub fn debounce_mode(&self) -> DebounceMode {
		self.debounce_mode
	}

	pub fn reverse_encoders(&self) -> ReverseMode {
		self.reverse_encoders
	}
}

impl Default for SDVXControllerOptions {
	fn default() -> Self {
		Self {
			debounce_encoders: false,
			debounce_duration: MicrosDurationU64::micros(SW_DEBOUNCE_DURATION_US),
			debounce_mode: DebounceMode::default(),
			reverse_encoders: ReverseMode::default(),
		}
	}
}


/// Determines the type of debounce algorithm to use with the buttons.
/// Default is [`DebounceMode::None`].
#[derive(Clone, Copy, Default, PartialEq)]
pub enum DebounceMode {
	/// Disables debouncing.
	#[default] None,
	/// Immediately reports when a switch is triggered and holds it for an N amount of time.
	///	Also known as "eager debouncing".
	Hold,
	/// Waits for a switch to output a constant N amount of time before reporting.
	/// Also known as "deferred debouncing".
	Wait,
}


/// Determines which encoders should reverse their direction when reporting their data.
/// Default is [`ReverseMode::None`].
#[derive(Clone, Copy, Default)]
pub enum ReverseMode {
	/// Keeps the encoders' direction as reported.
	#[default] None,
	/// Reverses the direction of both encoders.
	Both,
	/// Reverses the direction of the left encoder **only**.
	Left,
	/// Reverses the direction of the right encoder **only**.
	Right,
}

impl ReverseMode {
	/// Returns the configuration of the encoders in a boolean tuple.
	/// The first item is the left encoder's configuration, while the second item is the right one.
	pub fn state(&self) -> (bool, bool) {
		match self {
			ReverseMode::None => (false, false), 
			ReverseMode::Both => (true, true),
			ReverseMode::Left => (true, false),
			ReverseMode::Right => (false, true),
		}
	}
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
