fn main() {
    // Declare custom cfg features to avoid "unexpected cfg" warnings
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"postgres-tests\", \"mysql-tests\"))");
}
