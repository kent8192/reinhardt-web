// The model macro emits native cfgs; this fixture tests the missing unique accessor.
#![allow(unexpected_cfgs)]

use reinhardt::model;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "articles", table_name = "articles")]
struct Article {
    #[field(primary_key = true)]
    id: Option<i64>,

    #[field(max_length = 255)]
    title: String,
}

fn main() {
    let _ = Article::unique_title();
}
