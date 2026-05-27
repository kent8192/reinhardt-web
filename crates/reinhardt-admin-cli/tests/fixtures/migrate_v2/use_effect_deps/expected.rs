#![rustfmt::skip]
use reinhardt_pages::reactive::hooks::use_effect;
use reinhardt_pages::reactive::Signal;
fn r(count: Signal<i32>) {
    let _ = use_effect(
        {
            let count = count.clone();
            move || {
                let _ = count.get();
            }
        },
        compile_error!(
            "manouche-v2 codemod: add explicit deps tuple here, e.g. `(count.clone(),)`"
        ),
    );
}
