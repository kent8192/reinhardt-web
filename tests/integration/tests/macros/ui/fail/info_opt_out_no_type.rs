//! Fail case: `#[model(info = false)]` must NOT generate `{Model}Info` type.
//! Referencing the type should produce a compile error.
//! Issue #4194.

use reinhardt::model;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "no_info_items", info = false)]
struct NoInfoItem {
    #[field(primary_key = true)]
    id: Option<i64>,

    #[field(max_length = 50)]
    name: String,
}

fn main() {
    let _x: NoInfoItemInfo;
}
