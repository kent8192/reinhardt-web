//! `IpAddressField` bind listener generates code compatible with `Signal<Option<IpAddr>>`.

use reinhardt_pages::form;

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _ = form! {
			name: NetworkForm,
			action: "/api/network",
			fields: {
				server_ip: IpAddressField,
			}
		};
	});
}
