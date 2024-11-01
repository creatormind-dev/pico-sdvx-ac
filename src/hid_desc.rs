use usbd_hid::descriptor::{generator_prelude::*, SerializedDescriptor};


/// Gamepad Report Descriptor.
#[gen_hid_descriptor(
	(collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = GAMEPAD) = {
		(usage_page = BUTTON, usage_min = 0x1, usage_max = 0x7) = {
			#[packed_bits 7] #[item_settings data,variable,absolute] buttons=input;
		};
		(usage_page = GENERIC_DESKTOP,) = {
			(usage = X,) = {
				#[item_settings data,variable,absolute] x=input;
			};
			(usage = Y,) = {
				#[item_settings data,variable,absolute] y=input;
			};
		};
	}
)]
pub struct GamepadReport {
	pub buttons: u8,
	pub x: i8,
	pub y: i8,
}
