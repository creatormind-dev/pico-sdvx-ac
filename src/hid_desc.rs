use usbd_hid::descriptor::{generator_prelude::*, SerializedDescriptor};


/// The report ID for the input controller report.
pub const HID_JOYSTICK_REPORT_ID: u8 = 1;
/// The report ID for the output lighting report.
pub const HID_LIGHTING_REPORT_ID: u8 = 2;
/// The size (in bytes) for the gamepad report.
pub const HID_JOYSTICK_SIZE: usize = size_of::<JoystickReport>();
/// The size (in bytes) for the lighting report.
pub const HID_LIGHTING_SIZE: usize = size_of::<LightingReport>();


#[derive(Default)]
#[gen_hid_descriptor(
	(collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = JOYSTICK) = {
		(report_id = 0x01,) = {
			(usage_page = BUTTON, usage_min = 1, usage_max = 7) = {
				#[packed_bits 7] #[item_settings data, variable, absolute] buttons = input;
			};
			(usage_page = GENERIC_DESKTOP,) = {
				(usage = X,) = {
					#[item_settings data, variable, absolute] x = input;
				};
				(usage = Y,) = {
					#[item_settings data, variable, absolute] y = input;
				};
			};
		}
	}
)]
pub struct JoystickReport {
	pub buttons: u8,
	pub x: u8,
	pub y: u8,
}

impl JoystickReport {
	/// Converts the report into raw bytes.
	/// An extra byte is added at the start, this is the report ID.
	pub fn to_bytes(&self) -> [u8; HID_JOYSTICK_SIZE + 1] {
		[
			HID_JOYSTICK_REPORT_ID,
			self.buttons,
			self.x,
			self.y,
		]
	}
}


#[derive(Default)]
#[gen_hid_descriptor(
	(collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = 0x00) = {
		(report_id = 0x02,) = {
			(usage_page = ORDINAL, usage_min = 1, usage_max = 16) = {
				#[item_settings data, variable, absolute] buttons = output;
			};
		};
	}
)]
pub struct LightingReport {
	pub buttons: [u8; 16],
}

impl LightingReport {
	pub fn from_bytes(buffer: &[u8]) -> Self {
		let mut buttons = [0u8; 16];

		buttons.copy_from_slice(buffer);

		Self { buttons }
	}
}
