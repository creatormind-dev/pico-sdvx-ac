use crate::*;

use usbd_hid::descriptor::SerializedDescriptor;


/// The report ID for the input controller report.
pub const HID_JOYSTICK_REPORT_ID: u8 = 1;
/// The report ID for the output lighting report.
pub const HID_LIGHTING_REPORT_ID: u8 = 2;
/// The size (in bytes) for the gamepad report.
pub const HID_JOYSTICK_DATA_SIZE: usize = size_of::<JoystickReport>();
/// The size (in bytes) for the lighting report.
pub const HID_LIGHTING_DATA_SIZE: usize = size_of::<LightingReport>();

/// Report size for the buttons (in bits).
const SW_REPORT_SIZE: u8 = (((SW_GPIO_SIZE / 8) + 1) * 8) - SW_GPIO_SIZE;
/// Report size for the lights (in bits).
const LED_REPORT_SIZE: u8 = LED_GPIO_SIZE;


/// Descriptor template for a Joystick.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct JoystickReport {
	/// The buttons presses are repoted in a single byte.
	/// If you use more than 8 buttons, change the type to `u16`.
	pub buttons: u8,
	/// Reports an encoder as the joystick's X-axis.
	pub x: u8,
	/// Reports the other encoder as the joysticks's Y-axis.
	pub y: u8,
}

impl JoystickReport {
	/// Converts the report into raw bytes.
	/// An extra byte is added at the start, this is the report ID.
	pub fn to_bytes(&self) -> [u8; HID_JOYSTICK_DATA_SIZE + 1] {
		[
			HID_JOYSTICK_REPORT_ID,
			self.buttons,
			self.x,
			self.y,
		]
	}
}

impl SerializedDescriptor for JoystickReport {
	/// Returns the HID descriptor for a joystick.
	fn desc() -> &'static [u8] {
		&[
			0x05, 0x01,									// USAGE_PAGE (Generic Desktop)
			0x09, 0x04,									// USAGE (Joystick)
			0xA1, 0x01,									// COLLECTION (Application)
			0x85, HID_JOYSTICK_REPORT_ID,				//   REPORT_ID (_)
			0x05, 0x09,									//   USAGE_PAGE (Button)
			0x19, 0x01,									//   USAGE_MINIMUM (0x01)
			0x29, SW_GPIO_SIZE,							//   USAGE_MAXIMUM (_)
			0x15, 0x00,									//   LOGICAL_MINIMUM (0)
			0x25, 0x01,									//   LOGICAL_MAXIMUM (1)
			0x75, 0x01,									//   REPORT_SIZE (1)
			0x95, SW_GPIO_SIZE,							//   REPORT_COUNT (_)
			0x81, 0x02,									//   INPUT (Data,Var,Abs)
			0x75, SW_REPORT_SIZE,						//   REPORT_SIZE (_)
			0x95, 0x01,									//   REPORT_COUNT (1)
			0x81, 0x03,									//   INPUT (Const,Var,Abs)
			0x05, 0x01,									//   USAGE_PAGE (Generic Desktop)
			0x15, 0x00,									//   LOGICAL_MINIMUM (0)
			0x26, 0xFF, 0x00,							//   LOGICAL_MAXIMUM (255)
			0x09, 0x30,									//   USAGE (X)
			0x09, 0x31,									//   USAGE (Y)
			0x75, 0x08,									//   REPORT_SIZE (8)
			0x95, 0x02,									//   REPORT_COUNT (2)
			0x81, 0x02,									//   INPUT (Data,Var,Abs)
			0xC0,										// END_COLLECTION
		]
	}
}


/// Descriptor template for lighting.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct LightingReport {
	/// Represents lighting data for the buttons.
	pub buttons: [u8; LED_GPIO_SIZE as _],
}

impl LightingReport {
	/// Generates a report from raw data.
	pub fn from_bytes(buffer: &[u8]) -> Self {
		let mut buttons = [0u8; LED_GPIO_SIZE as _];

		buttons.copy_from_slice(buffer);

		Self { buttons }
	}
}

impl SerializedDescriptor for LightingReport {
	/// Returns the HID descriptor for lighting use.
	fn desc() -> &'static [u8] {
		&[
			0x05, 0x01,									// USAGE_PAGE (Generic Desktop)
			0x09, 0x00,									// USAGE (Undefined)
			0xA1, 0x01,									// COLLECTION (Application)
			0x85, HID_LIGHTING_REPORT_ID,				//   REPORT_ID (_)
			0x75, 0x08,									//   REPORT_SIZE (8)
			0x95, LED_REPORT_SIZE,						//   REPORT_COUNT (_)
			0x15, 0x00,									//   LOGICAL_MINIMUM (0)
			0x26, 0xFF, 0x00,							//   LOGICAL_MAXIMUM (255)
			0x05, 0x08,									//   USAGE_PAGE (LEDs)
			0x79, 0x04,									//   STRING_MINIMUM (4)
			0x89, 0x10,									//   STRING_MAXIMUM (16)
			0x19, 0x01,									//   USAGE_MINIMUM (0x01)
			0x29, 0x0D,									//   USAGE_MAXIMUM (0x0D)
			0x91, 0x02,									//   OUTPUT (Data,Var,Abs)
			0x75, 0x08,									//   REPORT_SIZE (8)
			0x95, 0x01,									//   REPORT_COUNT (1)
			0x81, 0x03,									//   INPUT (Const,Var,Abs)
			0xC0										// END_COLLECTION
		]
	}
}
