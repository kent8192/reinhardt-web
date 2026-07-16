//! `IpAddressField` must compile and produce `Signal<Option<IpAddr>>`.

use reinhardt_pages::form;

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _ = form! {
			name: NetForm,
			action: "/api/net",
			fields: {
				client_ip: IpAddressField,
			}
		};
	});
}
