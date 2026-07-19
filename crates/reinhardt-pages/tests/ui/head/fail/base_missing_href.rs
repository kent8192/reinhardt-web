use reinhardt_pages::head;

fn main() {
	// Workaround for reinhardt-web#5738.
	// Remove this workaround when fmt-all preserves empty head blocks.
	//
	// Ideal implementation (without workaround):
	//   let _ = head!(|| { base {} });
	// reinhardt-fmt: ignore
	let _ = head!(|| { base {} });
}
