//! Integration tests for keyed `page!` list rendering.

use reinhardt_pages::{Page, page};

#[derive(Clone)]
struct Todo {
	id: String,
	title: String,
}

#[test]
fn keyed_for_lowers_to_keyed_fragment() {
	let render = page!(|todos: Vec<Todo>| {
		ul {
			for todo in todos @key(todo.id.clone()) {
				li { { todo.title.clone() } }
			}
		}
	});

	let view = render(vec![
		Todo {
			id: "a".to_string(),
			title: "Alpha".to_string(),
		},
		Todo {
			id: "b".to_string(),
			title: "Beta".to_string(),
		},
	]);

	let Page::Element(ul) = view else {
		panic!("expected top-level element");
	};
	let [Page::Reactive(list)] = ul.child_views() else {
		panic!("expected keyed for loop to be auto-wrapped as a reactive child");
	};
	let rendered_list = list.render();

	let Page::KeyedFragment(children) = rendered_list else {
		panic!("expected keyed for loop to lower to KeyedFragment");
	};
	assert_eq!(children.len(), 2);
	assert_eq!(children[0].0, "a");
	assert_eq!(children[1].0, "b");
	assert_eq!(children[0].1.render_to_string(), "<li>Alpha</li>");
	assert_eq!(children[1].1.render_to_string(), "<li>Beta</li>");
}
