//! Redirects integration tests
//!
//! Based on Django's redirects tests from:
//! - django/tests/redirects_tests/tests.py

use reinhardt_contrib::{Redirect, RedirectManager};

#[test]
fn test_redirect_model_string_representation() {
    let redirect = Redirect::new("/initial", "/new_target");

    // Test basic redirect properties
    assert_eq!(redirect.old_path, "/initial");
    assert_eq!(redirect.new_path, "/new_target");
    assert!(redirect.is_permanent);
}

#[test]
fn test_contrib_redirects_basic() {
    let mut manager = RedirectManager::new();
    let redirect = Redirect::new("/initial", "/new_target");

    manager.add(redirect);

    let result = manager.process_path(1, "/initial");
    assert!(result.is_some());

    let (new_path, status_code) = result.unwrap();
    assert_eq!(new_path, "/new_target");
    assert_eq!(status_code, 301);
}

#[test]
fn test_redirect_with_different_site() {
    let mut manager = RedirectManager::new();
    let redirect = Redirect::new("/initial", "/new_target").for_site(2);

    manager.add(redirect);

    // Should not find redirect for site 1
    assert!(manager.process_path(1, "/initial").is_none());

    // Should find redirect for site 2
    let result = manager.process_path(2, "/initial");
    assert!(result.is_some());
}

#[test]
fn test_permanent_vs_temporary_redirect() {
    let permanent = Redirect::new("/old", "/new");
    let temporary = Redirect::new("/old2", "/new2").temporary();

    assert_eq!(permanent.status_code(), 301);
    assert_eq!(temporary.status_code(), 302);
}

#[test]
fn test_redirect_manager_remove() {
    let mut manager = RedirectManager::new();
    let redirect = Redirect::new("/remove-me", "/target");

    manager.add(redirect);
    assert!(manager.get(1, "/remove-me").is_some());

    manager.remove(1, "/remove-me");
    assert!(manager.get(1, "/remove-me").is_none());
}

#[test]
fn test_list_redirects_for_site() {
    let mut manager = RedirectManager::new();

    manager.add(Redirect::new("/page1", "/new1").for_site(1));
    manager.add(Redirect::new("/page2", "/new2").for_site(1));
    manager.add(Redirect::new("/page3", "/new3").for_site(2));

    let site1_redirects = manager.list_for_site(1);
    assert_eq!(site1_redirects.len(), 2);

    let site2_redirects = manager.list_for_site(2);
    assert_eq!(site2_redirects.len(), 1);
}

#[test]
fn test_contrib_redirects_not_found() {
    let manager = RedirectManager::new();

    let result = manager.process_path(1, "/nonexistent");
    assert!(result.is_none());
}

#[test]
fn test_same_path_different_sites() {
    let mut manager = RedirectManager::new();

    manager.add(Redirect::new("/common", "/site1-target").for_site(1));
    manager.add(Redirect::new("/common", "/site2-target").for_site(2));

    let site1_result = manager.process_path(1, "/common");
    assert_eq!(site1_result.unwrap().0, "/site1-target");

    let site2_result = manager.process_path(2, "/common");
    assert_eq!(site2_result.unwrap().0, "/site2-target");
}
