//! Query engine for the native component test DOM.

use super::error::QueryError;
use super::events::ElementHandle;
use super::role::{Role, accessible_name, role_for};
use super::text_match::TextMatch;
use super::tree::{NodeId, ScreenInner};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn by_text(
	inner: &Rc<RefCell<ScreenInner>>,
	text: TextMatch,
) -> Result<ElementHandle, QueryError> {
	one(inner, find_by_text(&inner.borrow(), &text))
}

pub(crate) fn query_by_text(
	inner: &Rc<RefCell<ScreenInner>>,
	text: TextMatch,
) -> Result<Option<ElementHandle>, QueryError> {
	let matches = find_by_text(&inner.borrow(), &text);
	match matches.len() {
		0 => Ok(None),
		1 => Ok(Some(ElementHandle::new(inner.clone(), matches[0]))),
		_ => Err(QueryError::MultipleMatches),
	}
}

pub(crate) fn by_role_named(
	inner: &Rc<RefCell<ScreenInner>>,
	role: Role,
	name: TextMatch,
) -> Result<ElementHandle, QueryError> {
	one(inner, find_by_role(&inner.borrow(), role, Some(&name)))
}

pub(crate) fn by_label(
	inner: &Rc<RefCell<ScreenInner>>,
	label: TextMatch,
) -> Result<ElementHandle, QueryError> {
	let borrowed = inner.borrow();
	let matches = borrowed
		.dom
		.visible_elements()
		.into_iter()
		.filter(|node_id| {
			borrowed.dom.element(*node_id).is_some_and(|node| {
				node.supports_value()
					&& accessible_name(&borrowed.dom, *node_id)
						.as_deref()
						.is_some_and(|name| label.matches(name))
			})
		})
		.collect();
	one(inner, matches)
}

pub(crate) fn by_placeholder(
	inner: &Rc<RefCell<ScreenInner>>,
	placeholder: TextMatch,
) -> Result<ElementHandle, QueryError> {
	let borrowed = inner.borrow();
	let matches = borrowed
		.dom
		.visible_elements()
		.into_iter()
		.filter(|node_id| {
			borrowed.dom.element(*node_id).is_some_and(|node| {
				node.supports_value()
					&& node
						.attr("placeholder")
						.is_some_and(|value| placeholder.matches(value))
			})
		})
		.collect();
	one(inner, matches)
}

fn one(
	inner: &Rc<RefCell<ScreenInner>>,
	matches: Vec<NodeId>,
) -> Result<ElementHandle, QueryError> {
	match matches.len() {
		0 => Err(QueryError::NotFound),
		1 => Ok(ElementHandle::new(inner.clone(), matches[0])),
		_ => Err(QueryError::MultipleMatches),
	}
}

fn find_by_text(inner: &ScreenInner, text: &TextMatch) -> Vec<NodeId> {
	inner
		.dom
		.visible_elements()
		.into_iter()
		.filter(|node_id| element_has_exact_text_without_duplicate_child(inner, *node_id, text))
		.collect()
}

fn find_by_role(inner: &ScreenInner, role: Role, name: Option<&TextMatch>) -> Vec<NodeId> {
	inner
		.dom
		.visible_elements()
		.into_iter()
		.filter(|node_id| role_for(&inner.dom, *node_id) == Some(role))
		.filter(|node_id| {
			name.is_none_or(|name| {
				accessible_name(&inner.dom, *node_id)
					.as_deref()
					.is_some_and(|candidate| name.matches(candidate))
			})
		})
		.collect()
}

fn element_has_exact_text_without_duplicate_child(
	inner: &ScreenInner,
	node_id: NodeId,
	text: &TextMatch,
) -> bool {
	if !text.matches(&inner.dom.visible_text_content(node_id)) {
		return false;
	}
	!inner.dom.children(node_id).iter().any(|child| {
		inner.dom.element(*child).is_some()
			&& !inner.dom.is_hidden(*child)
			&& text.matches(&inner.dom.visible_text_content(*child))
	})
}
