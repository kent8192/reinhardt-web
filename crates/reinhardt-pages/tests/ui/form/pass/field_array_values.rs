//! form! emits collection value scaffolding for FieldArray.

use reinhardt_pages::{form, use_form};

fn main() {
	let invoice = form! {
		name: InvoiceForm,
		action: "/invoices",
		fields: {
			customer_name: CharField {
				required,
			}
			line_items: FieldArray {
				fields: {
					description: CharField {
						required,
					}
					quantity: IntegerField {
						required,
					}
				}
			}
		}
	};

	let runtime = use_form(&invoice).build();
	let values = runtime.get_values();
	let _customer_name: &String = &values.customer_name;
	let _line_items_len = values.line_items.len();
	let _item_fields = values.line_items.first().map(|item| {
		let _description: &String = &item.description;
		let _quantity: i64 = item.quantity;
	});
	let line_item_signals = invoice.line_items().get();
	let _line_item_signals: Vec<_> = line_item_signals;
	let _signal_item_fields = _line_item_signals.first().map(|item| {
		let _key = item.key();
		let _index: usize = item.index();
		let _description: &String = &item.value().description;
		let _quantity: i64 = item.value().quantity;
	});
	let _collection = invoice.line_items_collection();

	let addresses = form! {
		name: Addresses,
		action: "/addresses",
		fields: {
			addresses: FieldArray {
				fields: {
					street: CharField {}
				}
			}
		}
	};
	let _addresses_runtime = use_form(&addresses).build();
	let _address_item = addresses.new_addresses_item();
	let _addresses_collection = addresses.addresses_collection();
}
