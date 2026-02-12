fn main() {
	cfg_aliases::cfg_aliases! {
		// Platform detection
		android: { target_os = "android" },
		ios: { target_os = "ios" },
		mobile: { any(target_os = "android", target_os = "ios") },

		// Desktop platforms (for reference)
		desktop: { not(any(target_os = "android", target_os = "ios")) },

		// Feature combinations
		android_experimental: { all(target_os = "android", feature = "experimental") },
		ios_experimental: { all(target_os = "ios", feature = "experimental") },
	}
}
