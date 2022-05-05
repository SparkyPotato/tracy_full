#[repr(transparent)]
pub struct Color(u32);

impl Color {
	const BLACK: Color = Color::new(0, 0, 1);
	const BLUE: Color = Color::new(0, 0, 255);
	const CYAN: Color = Color::new(0, 255, 255);
	const GREEN: Color = Color::new(0, 255, 0);
	const MAGENTA: Color = Color::new(255, 0, 255);
	const RED: Color = Color::new(255, 0, 0);
	const WHITE: Color = Color::new(255, 255, 255);
	const YELLOW: Color = Color::new(255, 255, 0);
}

impl Color {
	pub const fn new(r: u8, g: u8, b: u8) -> Color { Color((r as u32) << 16 | (g as u32) << 8 | (b as u32) << 0) }

	pub const fn none() -> Color { Color(0) }
}
