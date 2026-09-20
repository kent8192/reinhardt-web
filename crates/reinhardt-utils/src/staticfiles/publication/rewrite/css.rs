//! CSS tokenizer-based URL/import discovery, including nested blocks.

use super::{Site, map_reference, reference};
use crate::staticfiles::publication::{AssetBuildError, AssetReference};
use cssparser::{Parser, ParserInput, Token};

pub(super) fn analyze(logical: &str, source: &str) -> Result<Vec<AssetReference>, AssetBuildError> {
	let mut input = ParserInput::new(source);
	let mut parser = Parser::new(&mut input);
	let mut references = Vec::new();
	scan(logical, &mut parser, &mut references)?;
	Ok(references)
}

fn scan<'i, 't>(
	logical: &str,
	parser: &mut Parser<'i, 't>,
	refs: &mut Vec<AssetReference>,
) -> Result<(), AssetBuildError> {
	let mut import = false;
	loop {
		let start = parser.position().byte_index();
		let token = match parser.next_including_whitespace_and_comments() {
			Ok(token) => token.clone(),
			Err(_) => break,
		};
		let end = parser.position().byte_index();
		match token {
			Token::AtKeyword(name) if name.eq_ignore_ascii_case("import") => import = true,
			Token::QuotedString(value) if import => {
				if let Some(reference) = reference(logical, &value, Site::CssString { start, end })?
				{
					refs.push(reference);
				}
				import = false;
			}
			Token::UnquotedUrl(value) => {
				if let Some(reference) = reference(logical, &value, Site::CssUrl { start, end })? {
					refs.push(reference);
				}
				import = false;
			}
			Token::Function(name) if name.eq_ignore_ascii_case("url") => {
				let value = parser
					.parse_nested_block(|nested| {
						let value = nested.expect_string_cloned()?;
						nested.expect_exhausted()?;
						Ok::<_, cssparser::ParseError<'i, ()>>(value.to_string())
					})
					.map_err(|e| {
						AssetBuildError::input(
							logical,
							format!("invalid CSS url at byte {start}: {e:?}"),
						)
					})?;
				let end = parser.position().byte_index();
				if let Some(reference) = reference(logical, &value, Site::CssUrl { start, end })? {
					refs.push(reference);
				}
				import = false;
			}
			Token::Function(_)
			| Token::ParenthesisBlock
			| Token::SquareBracketBlock
			| Token::CurlyBracketBlock => {
				parser
					.parse_nested_block(|nested| {
						scan(logical, nested, refs).map_err(|error| {
							nested.new_custom_error::<String, String>(error.to_string())
						})
					})
					.map_err(|e| {
						AssetBuildError::input(logical, format!("CSS nested reference: {e:?}"))
					})?;
				import = false;
			}
			Token::Comment(comment) => {
				if let Some(reference) = map_reference(logical, comment, start + 2)? {
					refs.push(reference);
				}
			}
			Token::BadUrl(_) | Token::BadString(_) => {
				return Err(AssetBuildError::input(
					logical,
					format!("invalid CSS token at byte {start}"),
				));
			}
			Token::WhiteSpace(_) => {}
			_ => import = false,
		}
	}
	Ok(())
}
