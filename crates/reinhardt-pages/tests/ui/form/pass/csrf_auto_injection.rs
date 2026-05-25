//! CSRF token auto-injection - POST forms automatically include CSRF hidden input

use reinhardt_pages::form;

fn main() {
	// POST form automatically includes CSRF token hidden input
	let _post_form = form! {
		name: PostForm,
		action: "/api/submit",
		method: Post,

		fields: {
			message: CharField {
				required,
			}
		}

	};

	// PUT form also includes CSRF token
	let _put_form = form! {
		name: PutForm,
		action: "/api/update",
		method: Put,

		fields: {
			data: CharField {
				required,
			}
		}

	};

	// PATCH form includes CSRF token
	let _patch_form = form! {
		name: PatchForm,
		action: "/api/patch",
		method: Patch,

		fields: {
			field: CharField {
				required,
			}
		}

	};

	// DELETE form includes CSRF token
	let _delete_form = form! {
		name: DeleteForm,
		action: "/api/delete",
		method: Delete,

		fields: {
			id: IntegerField {
				required,
			}
		}

	};

	// GET form does NOT include CSRF token (safe method)
	let _get_form = form! {
		name: GetForm,
		action: "/api/search",
		method: Get,

		fields: {
			query: CharField {
				required,
			}
		}

	};
}
