+++
title = "Part 3: Detail Pages and Voting"
description = "Add poll detail, voting, and results pages backed by typed server functions."
weight = 30

[extra]
sidebar_weight = 30
+++

# Part 3: Detail Pages and Voting

The poll index now links to detail pages, but the detail route does not do useful work yet. In this part you will add the read path for one poll, a vote mutation, and a results page.

The browser submits votes through `form!` and a `#[server_fn]`. The server keeps the database rules: it verifies that the selected choice belongs to the selected question, increments the vote in a transaction, and returns the generated `ChoiceInfo` DTO.

## Add the Request Type

Open `src/shared/types.rs` and add the vote request DTO:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    pub question_id: i64,
    pub choice_id: i64,
}
```

The browser form will use individual `question_id` and `choice_id` fields, but this DTO stays useful for the direct `vote` server function and for tests.

## Load One Question

Add `get_question_detail` to `src/apps/polls/server_fn.rs`:

```rust
use crate::apps::polls::server::models::{Choice, Question};
use reinhardt::{DatabaseConnection, Model};
use std::result::Result;

#[server_fn]
pub async fn get_question_detail(
    question_id: i64,
    #[inject] _db: DatabaseConnection,
) -> Result<(QuestionInfo, Vec<ChoiceInfo>), ServerFnError> {
    let question_manager = Question::objects();
    let question = question_manager
        .get(question_id)
        .first()
        .await
        .map_err(|e| ServerFnError::application(e.to_string()))?
        .ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

    let choice_manager = Choice::objects();
    let choices = choice_manager
        .filter(Choice::field_question_id().eq(question_id))
        .all()
        .await
        .map_err(|e| ServerFnError::application(e.to_string()))?;

    let question_info = QuestionInfo::from(question);
    let choice_infos: Vec<ChoiceInfo> = choices.into_iter().map(ChoiceInfo::from).collect();

    Ok((question_info, choice_infos))
}
```

The query uses generated field helpers (`Choice::field_question_id()`) instead of formatting SQL by hand.

## Add the Results Query

The results page needs the same question and choices plus a total vote count:

```rust
use crate::apps::polls::server::models::{Choice, Question};
use reinhardt::{DatabaseConnection, Model};
use std::result::Result;

#[server_fn]
pub async fn get_question_results(
    question_id: i64,
    #[inject] _db: DatabaseConnection,
) -> Result<(QuestionInfo, Vec<ChoiceInfo>, i32), ServerFnError> {
    let question_manager = Question::objects();
    let question = question_manager
        .get(question_id)
        .first()
        .await
        .map_err(|e| ServerFnError::application(e.to_string()))?
        .ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

    let choice_manager = Choice::objects();
    let choices = choice_manager
        .filter(Choice::field_question_id().eq(question_id))
        .all()
        .await
        .map_err(|e| ServerFnError::application(e.to_string()))?;

    let total_votes: i32 = choices.iter().map(|c| c.votes()).sum();

    let question_info = QuestionInfo::from(question);
    let choice_infos: Vec<ChoiceInfo> = choices.into_iter().map(ChoiceInfo::from).collect();

    Ok((question_info, choice_infos, total_votes))
}
```

The total is computed server-side so the client can render percentages without reinterpreting the data model.

## Add the Vote Mutation

The reference example exposes two entry points:

- `vote(VoteRequest, db)` for direct typed calls and tests.
- `submit_vote(question_id, choice_id, db)` for `form!`, because the form macro submits individual typed fields.

Add both and share the implementation:

```rust
use crate::apps::polls::server::models::ChoiceInfo;
use crate::apps::polls::services::server::vote_internal;
use crate::shared::types::VoteRequest;
use reinhardt::DatabaseConnection;
use std::result::Result;

#[server_fn]
pub async fn vote(
    request: crate::shared::types::VoteRequest,
    #[inject] db: DatabaseConnection,
) -> Result<ChoiceInfo, ServerFnError> {
    vote_internal(request, db).await
}

#[server_fn]
pub async fn submit_vote(
    question_id: i64,
    choice_id: i64,
    #[inject] db: DatabaseConnection,
) -> Result<ChoiceInfo, ServerFnError> {
    let request = VoteRequest {
        question_id,
        choice_id,
    };

    vote_internal(request, db).await
}
```

The `vote` server function keeps the DTO argument on its canonical path because the generated marker code resolves that type from the `#[server_fn]` signature. The form wrapper still imports `VoteRequest` before constructing it.

Place the shared implementation in the server-only service module `src/apps/polls/services/server.rs`. The module is already gated from the app parent, so the service function itself does not need a local `#[cfg(server)]` gate:

```rust
use crate::apps::polls::server::models::{Choice, ChoiceInfo};
use crate::shared::types::VoteRequest;
use reinhardt::pages::server_fn::ServerFnError;
use reinhardt::{DatabaseConnection, Model, atomic};
use std::result::Result;

pub async fn vote_internal(
    request: VoteRequest,
    db: DatabaseConnection,
) -> Result<ChoiceInfo, ServerFnError> {

    let updated_choice = atomic(&db, || async {
        let choice_manager = Choice::objects();

        let mut choice = choice_manager
            .get(request.choice_id)
            .first()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .ok_or_else(|| anyhow::anyhow!("Choice not found"))?;

        if *choice.question_id() != request.question_id {
            return Err(anyhow::anyhow!("Choice does not belong to question"));
        }

        choice.vote().await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(choice)
    })
    .await
    .map_err(|e| ServerFnError::application(e.to_string()))?;

    Ok(ChoiceInfo::from(updated_choice))
}
```

Add the `Choice::vote()` model helper in `src/apps/polls/server/models.rs`:

```rust
#[cfg(server)]
use reinhardt::core::exception::Result;

#[cfg(server)]
impl Choice {
    pub async fn vote(&mut self) -> Result<()> {
        self.votes += 1;
        self.save().await
    }
}
```

The helper increments `votes` and saves the row through the model lifecycle.

## Register the Server Functions

Add the new markers in `src/apps/polls/urls/server_router.rs`:

```rust
ServerRouter::new()
    .server_fn(get_questions::marker)
    .server_fn(get_question_detail::marker)
    .server_fn(get_question_results::marker)
    .server_fn(vote::marker)
    .server_fn(submit_vote::marker)
```

Server-function routes stay app-local. `src/config/urls.rs` should still mount `polls::urls::server_url_patterns()` rather than list each handler.

## Add Client Routes

Add detail and results routes in `src/apps/polls/urls/client_router.rs`:

```rust
ClientRouter::new()
    .component(components::polls_index::polls_index)
    .component(components::polls_detail::polls_detail)
    .component(components::polls_results::polls_results)
```

Keep the route names stable. Client components should reverse named routes:

```rust
polls_routes::reverse("detail", &[("question_id", question.id.to_string().as_str())])
```

## Build the Detail Page

The detail component loads the question and choices:

```rust
pub fn polls_detail(question_id: i64) -> Page {
    let qid = question_id;

    let load_detail = use_resource(
        move || async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
        (),
    );
```

Then it defines the voting form. This form is the important part of the chapter:

```rust
let voting_form = form! {
    name: VotingForm,
    server_fn: submit_vote,
    method: Post,
    success_url: |_form| polls_routes::reverse("results", &[("question_id", qid.to_string().as_str())]),
    fields: {
        question_id: HiddenField<i64> {
            initial: qid,
        }
        choice_id: ChoiceField<i64> {
            widget: RadioSelect,
            required,
            label: "Select your choice",
            class: "poll-choice-input",
            wrapper_class: "poll-choice-field",
            label_class: "poll-choice-label",
            choices_from: "choices",
            choice_value: "id",
            choice_label: "choice_text",
        }
    }
    watch: {
        submit_button: |form| {
            let is_loading = form.loading().get();
            let back_href = polls_routes::reverse("index", &[]);
            page!(|is_loading: bool, back_href: String| {
                div {
                    class: "mt-3",
                    button {
                        type: "submit",
                        disabled: is_loading,
                        {
                            if is_loading { "Voting..." } else { "Vote" }
                        }
                    }
                    a {
                        href: back_href,
                        class: "btn-secondary ml-2",
                        "Back to Polls"
                    }
                }
            })(is_loading, back_href)
        },
    }
};
```

`choices_from: "choices"` binds the radio options to the choices returned by `get_question_detail`. The `watch` block stays in `form!` because it declares a render slot that depends on form state; the runtime state itself is still produced by the form runtime, and the static metadata path below explicitly calls `use_form(&form).build()`. The generated `#[server_fn]` client stub supplies the CSRF header for WASM submits; you do not pass CSRF as a business argument.

The final example also hides owner-only edit/delete controls here. Defer those branches until Part 5.

## Add Static Form Metadata

The reference example also exposes server-side form metadata from `src/apps/polls/server/forms.rs`:

```rust
pub fn create_vote_form() -> StaticFormMetadata {
    let form = form! {
        name: VoteForm,
        server_fn: submit_vote,
        method: Post,
        fields: {
            question_id: HiddenField<i64> {
                initial: 0i64,
            }
            choice_id: HiddenField<i64> {
                initial: 0i64,
                label: "Choice",
                required,
            }
        }
    };
    let _runtime = use_form(&form).build();
    form.metadata()
}
```

`src/apps/polls/server.rs` gates this module because form metadata is server-only:

```rust
pub mod forms;
```

## Build the Results Page

The results component loads `get_question_results`:

```rust
pub fn polls_results(question_id: i64) -> Page {
    let load_results = use_resource(
        move || async move {
            get_question_results(question_id)
                .await
                .map_err(|e| e.to_string())
        },
        (),
    );
```

When data is available, calculate a display percentage for each choice:

```rust
let percentage = if total > 0 {
    (choice.votes as f64 / total as f64 * 100.0) as i32
} else {
    0
};
```

Render a link back to the detail page so users can vote again:

```rust
let detail_href = polls_routes::reverse(
    "detail",
    &[("question_id", question_id.to_string().as_str())],
);
```

## Checkpoint

Run the app and vote:

```bash
cargo make dev
```

Open `http://127.0.0.1:8000/`, click a poll, choose a radio option, and submit. The app should navigate to `/polls/<id>/results/` and show the incremented vote count.

Before continuing:

- `get_question_detail`, `get_question_results`, `vote`, and `submit_vote` are registered in the polls server router.
- Detail and results routes are registered in the polls client router.
- The voting form uses `server_fn: submit_vote`, not an ad hoc HTTP endpoint.
- Vote updates run inside `atomic(&db, ...)` and verify the choice belongs to the question.
