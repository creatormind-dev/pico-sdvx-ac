use usbd_hid::descriptor::{generator_prelude::*, SerializedDescriptor};


/// The report ID for the input controller report.
pub const HID_GAMEPAD_REPORT_ID: u8 = 0x01;
/// The report ID for the output lighting report.
pub const HID_LIGHTNING_REPORT_ID: u8 = 0x02;

pub const HID_GAMEPAD_SIZE: usize = size_of::<GamepadReport>();
pub const HID_LIGHTNING_SIZE: usize = size_of::<LightningReport>();


// Constants cannot be used with procedural macros :(
// TODO: Remove the attribute macro in favor of dynamically generating the descriptor.

#[gen_hid_descriptor(
	(report_id = 0x01, collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = JOYSTICK) = {
		(usage_page = BUTTON, usage_min = 0x01, usage_max = 0x07) = {
			#[packed_bits 7] #[item_settings data,variable,absolute] buttons = input;
		};
		(usage_page = GENERIC_DESKTOP,) = {
			(usage = X,) = {
				#[item_settings data,variable,absolute] x = input;
			};
			(usage = Y,) = {
				#[item_settings data,variable,absolute] y = input;
			};
		};
	},
	(report_id = 0x02, collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = 0x00) = {
		(usage_page = LEDS, usage_min = 0x01, usage_max = 0x07) = {
			#[item_settings data,variable,absolute] lights = output;
		};
	}
)]
pub struct HIDControllerDescriptor {
	buttons: u8,
	x: u8,
	y: u8,
	lights: [u8; 7],
}


#[derive(Default, Clone, Copy)]
pub struct GamepadReport {
	id: u8,
	pub buttons: u8,
	pub x: u8,
	pub y: u8,
}

impl GamepadReport {
	pub fn new(buttons: u8, x: u8, y: u8) -> Self {
		Self {
			id: HID_GAMEPAD_REPORT_ID,
			buttons,
			x,
			y,
		}
	}

	pub fn to_bytes(&self) -> [u8; HID_GAMEPAD_SIZE] {
		[
			self.id,
			self.buttons,
			self.x,
			self.y,
		]
	}
}


#[derive(Default, Clone, Copy)]
pub struct LightningReport {
	pub lights: [u8; 7],
}

impl LightningReport {
	pub fn from_bytes(report: &[u8]) -> Option<Self> {
		match report.get(0)? {
			&HID_LIGHTNING_REPORT_ID => {
				let mut lights = [0u8; 7];
				lights.copy_from_slice(&report[1..]);

				Some(Self { lights })
			}
			&_ => None,
		}
	}
}
