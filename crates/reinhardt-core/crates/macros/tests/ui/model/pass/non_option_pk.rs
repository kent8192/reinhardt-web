//! Model with non-Option primary key

use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[model(app_label = "test", table_name = "posts")]
struct Post {
    #[field(primary_key = true)]
    id: i64,

    #[field(max_length = 200)]
    title: String,

    #[field]
    content: String,
}

fn main() {
    // Compile test only - just verify the macro expands without errors
    let _post = Post {
        id: 42,
        title: "Test Post".to_string(),
        content: "Content here".to_string(),
    };
}
