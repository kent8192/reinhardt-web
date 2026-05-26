//! `IpAddressField` bind listener generates code compatible with `Signal<Option<IpAddr>>`.

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: NetworkForm,
		action: "/api/network",

		fields: {
			server_ip: IpAddressField,
		}

	};
}
