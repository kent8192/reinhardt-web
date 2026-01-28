//! Tests for page tree functionality

use reinhardt_cms::pages::PageTree;
use rstest::*;

#[rstest]
#[tokio::test]
async fn test_create_root_page() {
	// Given: A new page tree
	let mut tree = PageTree::new();

	// When: Adding a root page
	let result = tree
		.add_page(None, "Home".to_string(), "home".to_string())
		.await;

	// Then: The page should be created successfully
	assert!(result.is_ok());
	let page = result.unwrap();
	assert_eq!(page.title, "Home");
	assert_eq!(page.slug, "home");
	assert_eq!(page.path, "/home");
	assert_eq!(page.depth, 0);
	assert!(page.parent_id.is_none());
}

#[rstest]
#[tokio::test]
async fn test_create_child_page() {
	// Given: A page tree with a root page
	let mut tree = PageTree::new();
	let root = tree
		.add_page(None, "Home".to_string(), "home".to_string())
		.await
		.unwrap();

	// When: Adding a child page
	let result = tree
		.add_page(Some(root.id), "About".to_string(), "about".to_string())
		.await;

	// Then: The child page should be created with correct hierarchy
	assert!(result.is_ok());
	let child = result.unwrap();
	assert_eq!(child.title, "About");
	assert_eq!(child.slug, "about");
	assert_eq!(child.path, "/home/about");
	assert_eq!(child.depth, 1);
	assert_eq!(child.parent_id, Some(root.id));
}

#[rstest]
#[tokio::test]
async fn test_get_children() {
	// Given: A page tree with parent and children
	let mut tree = PageTree::new();
	let root = tree
		.add_page(None, "Home".to_string(), "home".to_string())
		.await
		.unwrap();

	tree.add_page(Some(root.id), "About".to_string(), "about".to_string())
		.await
		.unwrap();
	tree.add_page(Some(root.id), "Contact".to_string(), "contact".to_string())
		.await
		.unwrap();

	// When: Getting children of the root page
	let children = tree.get_children(root.id).await;

	// Then: All children should be returned
	assert!(children.is_ok());
	let children = children.unwrap();
	assert_eq!(children.len(), 2);
	assert!(children.iter().any(|p| p.slug == "about"));
	assert!(children.iter().any(|p| p.slug == "contact"));
}

#[rstest]
#[tokio::test]
async fn test_move_page() {
	// Given: A page tree with multiple pages
	let mut tree = PageTree::new();
	let root1 = tree
		.add_page(None, "Blog".to_string(), "blog".to_string())
		.await
		.unwrap();
	let root2 = tree
		.add_page(None, "News".to_string(), "news".to_string())
		.await
		.unwrap();
	let child = tree
		.add_page(Some(root1.id), "Article".to_string(), "article".to_string())
		.await
		.unwrap();

	// When: Moving the article from Blog to News
	let result = tree.move_page(child.id, Some(root2.id)).await;

	// Then: The page should be moved successfully
	assert!(result.is_ok());

	let children = tree.get_children(root2.id).await.unwrap();
	assert_eq!(children.len(), 1);
	assert_eq!(children[0].slug, "article");
}

#[rstest]
#[tokio::test]
async fn test_get_ancestors() {
	// Given: A three-level page hierarchy
	let mut tree = PageTree::new();
	let root = tree
		.add_page(None, "Home".to_string(), "home".to_string())
		.await
		.unwrap();
	let child = tree
		.add_page(Some(root.id), "Blog".to_string(), "blog".to_string())
		.await
		.unwrap();
	let grandchild = tree
		.add_page(Some(child.id), "Post".to_string(), "post".to_string())
		.await
		.unwrap();

	// When: Getting ancestors of the grandchild
	let ancestors = tree.get_ancestors(grandchild.id).await;

	// Then: All ancestors should be returned in order
	assert!(ancestors.is_ok());
	let ancestors = ancestors.unwrap();
	assert_eq!(ancestors.len(), 2);
	assert_eq!(ancestors[0].slug, "home");
	assert_eq!(ancestors[1].slug, "blog");
}

#[rstest]
#[tokio::test]
async fn test_delete_page() {
	// Given: A page tree with a page
	let mut tree = PageTree::new();
	let page = tree
		.add_page(None, "Temp".to_string(), "temp".to_string())
		.await
		.unwrap();

	// When: Deleting the page
	let result = tree.delete_page(page.id, false).await;

	// Then: The page should be deleted successfully
	assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
async fn test_delete_page_recursive() {
	// Given: A page tree with parent and children
	let mut tree = PageTree::new();
	let root = tree
		.add_page(None, "Parent".to_string(), "parent".to_string())
		.await
		.unwrap();
	tree.add_page(Some(root.id), "Child".to_string(), "child".to_string())
		.await
		.unwrap();

	// When: Deleting the parent recursively
	let result = tree.delete_page(root.id, true).await;

	// Then: The parent and all children should be deleted
	assert!(result.is_ok());
}
