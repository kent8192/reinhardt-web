//! Basic operation tests for association proxies
//!
//! These tests verify the basic functionality of association proxies,
//! based on SQLAlchemy's tests.

use reinhardt_proxy::{AssociationProxy, ProxyBuilder, ScalarValue};

// NOTE: These tests currently test the API surface
// because reinhardt-proxy's actual ORM integration is not yet complete.

#[test]
fn test_constructor() {
    // Test: コンストラクタが正しく動作することを確認
    // Based on: test_constructor from SQLAlchemy

    let proxy: AssociationProxy<(), ()> = AssociationProxy::new("children", "name");
    assert_eq!(proxy.relationship, "children");
    assert_eq!(proxy.attribute, "name");
    assert!(proxy.creator.is_none());
}

#[test]
fn test_with_creator() {
    // Test: creatorを指定したプロキシが正しく動作することを確認

    fn creator(_name: String) -> i32 {
        42
    }

    let proxy = AssociationProxy::<i32, String>::new("children", "name").with_creator(creator);

    assert_eq!(proxy.relationship, "children");
    assert_eq!(proxy.attribute, "name");
    assert!(proxy.creator.is_some());
}

#[test]
fn test_proxy_builder_basic() {
    // Test: ビルダーパターンが正しく動作することを確認

    let proxy: AssociationProxy<(), ()> = ProxyBuilder::new()
        .relationship("rel")
        .attribute("attr")
        .build();

    assert_eq!(proxy.relationship, "rel");
    assert_eq!(proxy.attribute, "attr");
}

#[test]
fn test_builder_with_creator() {
    // Test: creatorを含むビルダーが正しく動作することを確認

    fn creator(_: String) -> i32 {
        42
    }

    let proxy = ProxyBuilder::new()
        .relationship("rel")
        .attribute("attr")
        .creator(creator)
        .build();

    assert!(proxy.creator.is_some());
}

#[test]
fn test_try_build_success() {
    // Test: try_buildが成功することを確認

    let proxy: Option<AssociationProxy<(), ()>> = ProxyBuilder::new()
        .relationship("rel")
        .attribute("attr")
        .try_build();

    assert!(proxy.is_some());
}

#[test]
fn test_try_build_failure() {
    // Test: try_buildが失敗することを確認

    let proxy: Option<AssociationProxy<(), ()>> =
        ProxyBuilder::new().relationship("rel").try_build();

    assert!(proxy.is_none());
}

#[test]
#[should_panic(expected = "Attribute must be set")]
fn test_builder_panic_no_attribute() {
    // Test: 属性が設定されていない場合にpanicすることを確認

    let _proxy: AssociationProxy<(), ()> = ProxyBuilder::new().relationship("rel").build();
}

#[test]
#[should_panic(expected = "Relationship must be set")]
fn test_builder_panic_no_relationship() {
    // Test: リレーションシップが設定されていない場合にpanicすることを確認

    let _proxy: AssociationProxy<(), ()> = ProxyBuilder::new().attribute("attr").build();
}

#[test]
fn test_proxy_basic_scalar_conversions() {
    // Test: ScalarValueの型変換が正しく動作することを確認

    let s = ScalarValue::String("test".to_string());
    assert_eq!(s.as_string().unwrap(), "test");
    assert!(s.as_integer().is_err());

    let i = ScalarValue::Integer(42);
    assert_eq!(i.as_integer().unwrap(), 42);
    assert!(i.as_string().is_err());

    let f = ScalarValue::Float(3.14);
    assert_eq!(f.as_float().unwrap(), 3.14);
    assert!(f.as_boolean().is_err());

    let b = ScalarValue::Boolean(true);
    assert_eq!(b.as_boolean().unwrap(), true);
    assert!(b.as_float().is_err());

    let n = ScalarValue::Null;
    assert!(n.is_null());
    assert!(n.as_string().is_err());
}

#[test]
fn test_scalar_value_from_conversions() {
    // Test: ScalarValueへの変換が正しく動作することを確認

    let s: ScalarValue = "test".into();
    assert_eq!(s.as_string().unwrap(), "test");

    let s: ScalarValue = String::from("test").into();
    assert_eq!(s.as_string().unwrap(), "test");

    let i: ScalarValue = 42i64.into();
    assert_eq!(i.as_integer().unwrap(), 42);

    let f: ScalarValue = 3.14f64.into();
    assert_eq!(f.as_float().unwrap(), 3.14);

    let b: ScalarValue = true.into();
    assert_eq!(b.as_boolean().unwrap(), true);
}

#[test]
fn test_proxy_basic_scalar_type_mismatch() {
    // Test: 型の不一致エラーが正しく動作することを確認

    let s = ScalarValue::String("test".to_string());
    let result = s.as_integer();
    assert!(result.is_err());

    match result {
        Err(e) => {
            let error_msg = e.to_string();
            assert!(error_msg.contains("Type mismatch"));
            assert!(error_msg.contains("Integer"));
        }
        Ok(_) => panic!("Expected error"),
    }
}

#[test]
fn test_association_proxy_helper() {
    // Test: association_proxyヘルパー関数が正しく動作することを確認

    use reinhardt_proxy::builder::association_proxy;

    let proxy: AssociationProxy<(), ()> = association_proxy("users", "name");
    assert_eq!(proxy.relationship, "users");
    assert_eq!(proxy.attribute, "name");
}
