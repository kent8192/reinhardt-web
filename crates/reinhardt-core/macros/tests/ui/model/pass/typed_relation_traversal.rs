use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(table_name = "corpus_files")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CorpusFile {
	#[field(primary_key = true, db_column = "corpus_pk")]
	id: i64,
	#[field(max_length = 255)]
	normalized_path: String,
}

#[model(table_name = "tags")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tag {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	label: String,
}

#[model(table_name = "projects")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Project {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	name: String,
	#[field(skip = true)]
	#[rel(one_to_many, to = Document, foreign_key = "project_id")]
	documents: Vec<Document>,
	#[rel(
		many_to_many,
		through = "project_tags",
		source_field = "project_id",
		target_field = "tag_id"
	)]
	tags: db::associations::ManyToManyField<Project, Tag>,
}

#[model(table_name = "documents")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	title: String,
	#[rel(foreign_key)]
	project: db::associations::ForeignKeyField<Project>,
	#[rel(foreign_key, null = true)]
	corpus_file: db::associations::ForeignKeyField<CorpusFile>,
}

fn main() {
	use db::orm::relations::RelationPathLike;

	assert_eq!(
		Document::rel_corpus_file().steps()[0].target_column,
		"corpus_pk"
	);
	let _forward = Document::rel_project().into_typed().field_name().eq("alpha");
	let _forward_low_level = Document::rel_project()
		.field(Project::field_name())
		.icontains("alpha");
	let _optional = Document::rel_corpus_file()
		.into_typed()
		.optional()
		.field_normalized_path()
		.is_null();
	let _reverse = Project::rel_documents().into_typed().field_title().icontains("draft");
	let _m2m = Project::rel_tags().into_typed().field_label().eq("source");
	let _nested_m2m = Document::rel_project()
		.into_typed()
		.rel_tags()
		.into_typed()
		.field_label()
		.eq("source");
	let _nested_reverse = Project::rel_documents()
		.into_typed()
		.rel_corpus_file()
		.into_typed()
		.field_normalized_path()
		.icontains("notes");
}
