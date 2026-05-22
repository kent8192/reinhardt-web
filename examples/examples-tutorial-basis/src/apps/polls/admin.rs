//! Admin configuration for the polls app.
//!
//! Demonstrates the `#[admin(model, ...)]` macro by registering the two
//! polls models (Question, Choice) so they appear in the auto-generated
//! `/admin/` interface.
use crate::apps::polls::models::{Choice, Question};
use reinhardt::admin;
/// Admin configuration for the Question model.
///
/// Lists id / question_text / pub_date / author_id columns, supports
/// search over the question text, and orders newest-first by default.
#[admin(
    model,
    for = Question,
    name = "Question",
    list_display = [id,
    question_text,
    pub_date,
    author_id],
    fields = [question_text,
    author_id],
    list_filter = [pub_date],
    search_fields = [question_text],
    ordering = [(pub_date, desc)],
    readonly_fields = [id,
    pub_date],
    list_per_page = 25,
    permissions = allow_all,
)]
pub struct QuestionAdmin;
/// Admin configuration for the Choice model.
///
/// Shows the foreign-key `question_id` alongside the choice text and
/// vote count, allowing operators to inspect and adjust vote totals
/// directly when seeding tutorial data.
#[admin(
    model,
    for = Choice,
    name = "Choice",
    list_display = [id,
    question_id,
    choice_text,
    votes],
    fields = [question_id,
    choice_text,
    votes],
    list_filter = [question_id],
    search_fields = [choice_text],
    ordering = [(id, asc)],
    readonly_fields = [id],
    list_per_page = 50,
    permissions = allow_all,
)]
pub struct ChoiceAdmin;
