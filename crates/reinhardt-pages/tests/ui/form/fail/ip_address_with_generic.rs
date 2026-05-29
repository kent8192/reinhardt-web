//! `IpAddressField<i32>` must be rejected — IpAddressField is specialized
//! to Option<IpAddr> and does not accept a type parameter.

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: BadForm,
		action: "/x",
		fields: {
			client_ip: IpAddressField<i32> {},
		}
	};
}
