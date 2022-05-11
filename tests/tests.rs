use tracy_full as tracy;

fn main() {
	for i in 0..10000 {
		tracy::frame!("secondary");

		tracy::frame!();

		if i % 2 == 0 {
			tracy::frame!(discontinuous "discontinuous");

			tracy::zone!("hi", true);
		}
	}
}
