#![rustfmt::skip]
use reinhardt_pages::deps;
use reinhardt_pages::reactive::hooks::{use_callback, use_effect, use_memo};
use reinhardt_pages::reactive::Signal;
fn r(count: Signal<i32>) {
    let _ = use_effect(
        {
            let count = count.clone();
            move || {
                let _ = count.get();
            }
        },
        deps![count],
    );
    let _ = use_effect(move || {}, deps![]);
    let _ = use_effect(
        build_callback(),
        compile_error!(
            "manouche-v2 codemod: add explicit deps list here, e.g. `deps![count]`"
        ),
    );
    let _ = use_memo(move || count.get(), deps![count]);
    let _ = use_callback(move |_| {}, deps![]);
}
