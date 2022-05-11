#[repr(transparent)]
pub struct Color(u32);

impl Color {
	pub const BLACK: Color = Color::new(0, 0, 1);
	pub const BLUE: Color = Color::new(0, 0, 255);
	pub const CYAN: Color = Color::new(0, 255, 255);
	pub const GREEN: Color = Color::new(0, 255, 0);
	pub const MAGENTA: Color = Color::new(255, 0, 255);
	pub const RED: Color = Color::new(255, 0, 0);
	pub const WHITE: Color = Color::new(255, 255, 255);
	pub const YELLOW: Color = Color::new(255, 255, 0);
}

impl Color {
	#[inline(always)]
	pub const fn new(r: u8, g: u8, b: u8) -> Color { Color((r as u32) << 16 | (g as u32) << 8 | (b as u32) << 0) }

	#[inline(always)]
	pub const fn none() -> Color { Color(0) }

	#[inline(always)]
	pub const fn to_u32(&self) -> u32 { self.0 }
}

impl Into<u32> for Color {
	#[inline(always)]
	fn into(self) -> u32 { self.0 }
}
