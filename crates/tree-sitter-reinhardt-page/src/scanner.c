#include "tree_sitter/parser.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

enum TokenType {
	LINE_COMMENT,
	BLOCK_COMMENT,
	RAW_STRING,
	CLOSURE_ARGS,
	IF_HEAD,
	FOR_HEAD,
	MATCH_HEAD,
	ELSE_HEAD,
	ATTRIBUTE_HEAD,
	EVENT_ATTRIBUTE_HEAD,
	FRAGMENT,
};

void *tree_sitter_reinhardt_page_external_scanner_create(void) { return NULL; }
void tree_sitter_reinhardt_page_external_scanner_destroy(void *payload) { (void)payload; }
void tree_sitter_reinhardt_page_external_scanner_reset(void *payload) { (void)payload; }
unsigned tree_sitter_reinhardt_page_external_scanner_serialize(void *payload, char *buffer) {
	(void)payload;
	(void)buffer;
	return 0;
}
void tree_sitter_reinhardt_page_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
	(void)payload;
	(void)buffer;
	(void)length;
}

static bool is_structural_delimiter(int32_t c) {
	return c == 0 || c == '\n' || c == '\r' || c == '{' || c == '}' || c == '[' || c == ']' ||
		   c == '(' || c == ')' || c == '"' || c == ',' || c == ';';
}

static bool is_ident_start(int32_t c) {
	return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_';
}

static bool is_ident_continue(int32_t c) {
	return is_ident_start(c) || (c >= '0' && c <= '9');
}

static bool is_horizontal_whitespace(int32_t c) { return c == ' ' || c == '\t'; }

static bool is_rust_whitespace(int32_t c) { return c == ' ' || c == '\t' || c == '\n' || c == '\r'; }

static bool has_keyword_prefix(const char *prefix, unsigned prefix_len, const char *keyword, unsigned keyword_len) {
	if (prefix_len <= keyword_len) {
		return false;
	}
	for (unsigned i = 0; i < keyword_len; i++) {
		if (prefix[i] != keyword[i]) {
			return false;
		}
	}
	return is_rust_whitespace(prefix[keyword_len]);
}

static bool prefix_equals_keyword(const char *prefix, unsigned prefix_len, const char *keyword, unsigned keyword_len) {
	if (prefix_len != keyword_len) {
		return false;
	}
	for (unsigned i = 0; i < keyword_len; i++) {
		if (prefix[i] != keyword[i]) {
			return false;
		}
	}
	return true;
}

static bool last_identifier_is(const char *identifier, unsigned identifier_len, const char *keyword,
							   unsigned keyword_len) {
	return prefix_equals_keyword(identifier, identifier_len, keyword, keyword_len);
}

static void record_fragment_char(int32_t c, char *prefix, unsigned *consumed_len, unsigned *content_len,
								 bool *has_content) {
	if (*consumed_len < 16) {
		prefix[*consumed_len] = (char)c;
	}
	(*consumed_len)++;
	if (!is_horizontal_whitespace(c)) {
		*has_content = true;
		*content_len = *consumed_len;
	}
}

static bool scan_line_comment_after_slash(TSLexer *lexer);
static bool scan_block_comment_after_slash(TSLexer *lexer);

static bool scan_string_literal(TSLexer *lexer) {
	lexer->advance(lexer, false);
	bool escaped = false;
	while (lexer->lookahead != 0) {
		int32_t curr = lexer->lookahead;
		lexer->advance(lexer, false);
		if (escaped) {
			escaped = false;
			continue;
		}
		if (curr == '\\') {
			escaped = true;
			continue;
		}
		if (curr == '"') {
			lexer->mark_end(lexer);
			return true;
		}
	}
	return false;
}

static bool scan_lifetime_or_char_literal(TSLexer *lexer) {
	lexer->advance(lexer, false);
	if (is_ident_start(lexer->lookahead)) {
		lexer->advance(lexer, false);
		while (is_ident_continue(lexer->lookahead)) {
			lexer->advance(lexer, false);
		}
		if (lexer->lookahead == '\'') {
			lexer->advance(lexer, false);
		}
		lexer->mark_end(lexer);
		return true;
	}

	bool escaped = false;
	while (lexer->lookahead != 0) {
		int32_t curr = lexer->lookahead;
		lexer->advance(lexer, false);
		if (escaped) {
			escaped = false;
			continue;
		}
		if (curr == '\\') {
			escaped = true;
			continue;
		}
		if (curr == '\'') {
			lexer->mark_end(lexer);
			return true;
		}
	}
	return false;
}

static bool scan_raw_string_literal(TSLexer *lexer) {
	lexer->advance(lexer, false);
	unsigned hashes = 0;
	while (lexer->lookahead == '#') {
		hashes++;
		lexer->advance(lexer, false);
	}
	if (lexer->lookahead != '"') {
		return false;
	}

	lexer->advance(lexer, false);
	unsigned matched_hashes = 0;
	while (lexer->lookahead != 0) {
		if (lexer->lookahead == '"') {
			matched_hashes = 0;
			lexer->advance(lexer, false);
			while (matched_hashes < hashes && lexer->lookahead == '#') {
				matched_hashes++;
				lexer->advance(lexer, false);
			}
			if (matched_hashes == hashes) {
				lexer->mark_end(lexer);
				return true;
			}
		} else {
			lexer->advance(lexer, false);
		}
	}
	return false;
}

static bool scan_balanced_brace_block(TSLexer *lexer) {
	if (lexer->lookahead != '{') {
		return false;
	}

	unsigned brace_depth = 0;
	while (lexer->lookahead != 0) {
		if (lexer->lookahead == '"') {
			if (!scan_string_literal(lexer)) {
				return false;
			}
			continue;
		}
		if (lexer->lookahead == '\'') {
			if (!scan_lifetime_or_char_literal(lexer)) {
				return false;
			}
			continue;
		}
		if (lexer->lookahead == 'r') {
			if (scan_raw_string_literal(lexer)) {
				continue;
			}
			lexer->mark_end(lexer);
			continue;
		}
		if (lexer->lookahead == '/') {
			lexer->advance(lexer, false);
			if (lexer->lookahead == '/') {
				scan_line_comment_after_slash(lexer);
				continue;
			}
			if (lexer->lookahead == '*') {
				if (!scan_block_comment_after_slash(lexer)) {
					return false;
				}
				continue;
			}
			lexer->mark_end(lexer);
			continue;
		}
		if (lexer->lookahead == '{') {
			brace_depth++;
			lexer->advance(lexer, false);
			continue;
		}
		if (lexer->lookahead == '}') {
			lexer->advance(lexer, false);
			brace_depth--;
			if (brace_depth == 0) {
				lexer->mark_end(lexer);
				return true;
			}
			continue;
		}
		lexer->advance(lexer, false);
	}
	return false;
}

static bool control_head_block_starts_expression(enum TokenType token_type, bool has_significant_tail,
												 int32_t last_significant, const char *last_identifier,
												 unsigned last_identifier_len) {
	if (!has_significant_tail) {
		return token_type == IF_HEAD || token_type == MATCH_HEAD;
	}
	if (token_type == FOR_HEAD && last_identifier_is(last_identifier, last_identifier_len, "in", 2)) {
		return true;
	}
	if (last_identifier_is(last_identifier, last_identifier_len, "async", 5) ||
		last_identifier_is(last_identifier, last_identifier_len, "const", 5) ||
		last_identifier_is(last_identifier, last_identifier_len, "try", 3) ||
		last_identifier_is(last_identifier, last_identifier_len, "unsafe", 6)) {
		return true;
	}
	return last_significant == '=' || last_significant == '(' || last_significant == '[' ||
		   last_significant == ',' || last_significant == ':' || last_significant == '!' ||
		   last_significant == '&' || last_significant == '|' || last_significant == '?' ||
		   last_significant == '+' || last_significant == '-' || last_significant == '*' ||
		   last_significant == '/' || last_significant == '%';
}

static void record_control_head_char(int32_t c, bool *has_significant_tail, int32_t *last_significant,
									 char *last_identifier, unsigned *last_identifier_len, bool *identifier_active) {
	if (is_rust_whitespace(c)) {
		*identifier_active = false;
		return;
	}

	*has_significant_tail = true;
	*last_significant = c;

	if (is_ident_continue(c)) {
		if (!*identifier_active) {
			*identifier_active = true;
			*last_identifier_len = 0;
		}
		if (*last_identifier_len < 15) {
			last_identifier[*last_identifier_len] = (char)c;
			(*last_identifier_len)++;
		}
		return;
	}

	*identifier_active = false;
}

static bool scan_control_head_tail(TSLexer *lexer, const bool *valid_symbols, enum TokenType token_type) {
	unsigned paren_depth = 0;
	unsigned bracket_depth = 0;
	bool has_significant_tail = false;
	int32_t last_significant = 0;
	char last_identifier[16];
	unsigned last_identifier_len = 0;
	bool identifier_active = false;

	while (lexer->lookahead != 0) {
		if (lexer->lookahead == '{' && paren_depth == 0 && bracket_depth == 0) {
			if (control_head_block_starts_expression(token_type, has_significant_tail, last_significant,
													 last_identifier, last_identifier_len)) {
				if (!scan_balanced_brace_block(lexer)) {
					return false;
				}
				has_significant_tail = true;
				last_significant = '}';
				last_identifier_len = 0;
				identifier_active = false;
				continue;
			}
			lexer->result_symbol = token_type;
			return true;
		}
		if (lexer->lookahead == '"') {
			if (!scan_string_literal(lexer)) {
				return false;
			}
			has_significant_tail = true;
			last_significant = '"';
			last_identifier_len = 0;
			identifier_active = false;
			continue;
		}
		if (lexer->lookahead == '\'') {
			if (!scan_lifetime_or_char_literal(lexer)) {
				return false;
			}
			has_significant_tail = true;
			last_significant = '\'';
			last_identifier_len = 0;
			identifier_active = false;
			continue;
		}
		if (lexer->lookahead == 'r') {
			if (scan_raw_string_literal(lexer)) {
				has_significant_tail = true;
				last_significant = '"';
				last_identifier_len = 0;
				identifier_active = false;
				continue;
			}
			record_control_head_char('r', &has_significant_tail, &last_significant, last_identifier,
									 &last_identifier_len, &identifier_active);
			lexer->mark_end(lexer);
			continue;
		}
		if (lexer->lookahead == '/') {
			lexer->advance(lexer, false);
			if (lexer->lookahead == '/') {
				scan_line_comment_after_slash(lexer);
				continue;
			}
			if (lexer->lookahead == '*') {
				if (!scan_block_comment_after_slash(lexer)) {
					return false;
				}
				continue;
			}
			record_control_head_char('/', &has_significant_tail, &last_significant, last_identifier,
									 &last_identifier_len, &identifier_active);
			lexer->mark_end(lexer);
			continue;
		}
		if (lexer->lookahead == '(') {
			paren_depth++;
		} else if (lexer->lookahead == ')' && paren_depth > 0) {
			paren_depth--;
		} else if (lexer->lookahead == '[') {
			bracket_depth++;
		} else if (lexer->lookahead == ']' && bracket_depth > 0) {
			bracket_depth--;
		}
		int32_t c = lexer->lookahead;
		bool is_trailing_whitespace = is_rust_whitespace(c);
		lexer->advance(lexer, false);
		record_control_head_char(c, &has_significant_tail, &last_significant, last_identifier, &last_identifier_len,
								 &identifier_active);
		if (!is_trailing_whitespace) {
			lexer->mark_end(lexer);
		}
	}
	if (valid_symbols[FRAGMENT]) {
		lexer->result_symbol = FRAGMENT;
		return true;
	}
	return false;
}

static bool scan_closure_args(TSLexer *lexer) {
	lexer->advance(lexer, false);
	unsigned paren_depth = 0;
	unsigned bracket_depth = 0;
	unsigned angle_depth = 0;
	while (lexer->lookahead != 0) {
		if (lexer->lookahead == '|' && paren_depth == 0 && bracket_depth == 0 && angle_depth == 0) {
			lexer->advance(lexer, false);
			lexer->mark_end(lexer);
			lexer->result_symbol = CLOSURE_ARGS;
			return true;
		}
		if (lexer->lookahead == '"') {
			if (!scan_string_literal(lexer)) {
				return false;
			}
			continue;
		}
		if (lexer->lookahead == '\'') {
			if (!scan_lifetime_or_char_literal(lexer)) {
				return false;
			}
			continue;
		}
		if (lexer->lookahead == 'r') {
			if (scan_raw_string_literal(lexer)) {
				continue;
			}
			lexer->mark_end(lexer);
			continue;
		}
		if (lexer->lookahead == '(') {
			paren_depth++;
		} else if (lexer->lookahead == ')' && paren_depth > 0) {
			paren_depth--;
		} else if (lexer->lookahead == '[') {
			bracket_depth++;
		} else if (lexer->lookahead == ']' && bracket_depth > 0) {
			bracket_depth--;
		} else if (lexer->lookahead == '<') {
			angle_depth++;
		} else if (lexer->lookahead == '>' && angle_depth > 0) {
			angle_depth--;
		}
		lexer->advance(lexer, false);
		lexer->mark_end(lexer);
	}
	return false;
}

static bool scan_fragment_tail(TSLexer *lexer, bool has_content) {
	while (!is_structural_delimiter(lexer->lookahead)) {
		if (lexer->lookahead == '\'') {
			lexer->advance(lexer, false);
			if (is_ident_start(lexer->lookahead)) {
				lexer->advance(lexer, false);
				if (lexer->lookahead == '\'') {
					break;
				}
				has_content = true;
				lexer->mark_end(lexer);
				while (is_ident_continue(lexer->lookahead)) {
					lexer->advance(lexer, false);
					has_content = true;
					lexer->mark_end(lexer);
				}
			} else {
				break;
			}
			continue;
		}
		if (lexer->lookahead == '/') {
			lexer->advance(lexer, false);
			if (lexer->lookahead == '/' || lexer->lookahead == '*') {
				break;
			}
			has_content = true;
			lexer->mark_end(lexer);
			continue;
		}
		bool is_trailing_whitespace = lexer->lookahead == ' ' || lexer->lookahead == '\t';
		lexer->advance(lexer, false);
		if (!is_trailing_whitespace) {
			has_content = true;
			lexer->mark_end(lexer);
		}
	}
	return has_content;
}

static bool scan_line_comment_after_slash(TSLexer *lexer) {
	while (lexer->lookahead != 0 && lexer->lookahead != '\n' && lexer->lookahead != '\r') {
		lexer->advance(lexer, false);
		lexer->mark_end(lexer);
	}
	return true;
}

static bool scan_block_comment_after_slash(TSLexer *lexer) {
	lexer->advance(lexer, false);
	bool saw_star = false;
	while (lexer->lookahead != 0) {
		int32_t curr = lexer->lookahead;
		if (saw_star && curr == '/') {
			lexer->advance(lexer, false);
			lexer->mark_end(lexer);
			return true;
		}
		saw_star = (curr == '*');
		lexer->advance(lexer, false);
	}
	return false;
}

static bool scan_classified_fragment(TSLexer *lexer, const bool *valid_symbols) {
	if (lexer->lookahead == ' ' || lexer->lookahead == '\t' || is_structural_delimiter(lexer->lookahead)) {
		return false;
	}

	if (lexer->lookahead == '|' && valid_symbols[CLOSURE_ARGS]) {
		return scan_closure_args(lexer);
	}

	int32_t first = lexer->lookahead;
	char prefix[16];
	unsigned consumed_len = 0;
	unsigned content_len = 0;
	bool has_content = false;

	if (lexer->lookahead == 'r') {
		lexer->advance(lexer, false);
		record_fragment_char('r', prefix, &consumed_len, &content_len, &has_content);
		lexer->mark_end(lexer);
		unsigned hashes = 0;
		while (lexer->lookahead == '#') {
			lexer->advance(lexer, false);
			record_fragment_char('#', prefix, &consumed_len, &content_len, &has_content);
			lexer->mark_end(lexer);
			hashes++;
		}
		if (lexer->lookahead == '"') {
			lexer->advance(lexer, false);
			unsigned matched_hashes = 0;
			while (lexer->lookahead != 0) {
				if (lexer->lookahead == '"') {
					matched_hashes = 0;
					lexer->advance(lexer, false);
					while (matched_hashes < hashes && lexer->lookahead == '#') {
						matched_hashes++;
						lexer->advance(lexer, false);
					}
					if (matched_hashes == hashes) {
						lexer->mark_end(lexer);
						if (valid_symbols[RAW_STRING]) {
							lexer->result_symbol = RAW_STRING;
							return true;
						}
						if (valid_symbols[FRAGMENT]) {
							lexer->result_symbol = FRAGMENT;
							return true;
						}
						return false;
					}
				} else {
					lexer->advance(lexer, false);
				}
			}
			return false;
		}
	}

	while (!is_structural_delimiter(lexer->lookahead)) {
		if (lexer->lookahead == '\'') {
			lexer->advance(lexer, false);
			if (is_ident_start(lexer->lookahead)) {
				int32_t ident = lexer->lookahead;
				lexer->advance(lexer, false);
				if (lexer->lookahead == '\'') {
					break;
				}
				record_fragment_char('\'', prefix, &consumed_len, &content_len, &has_content);
				record_fragment_char(ident, prefix, &consumed_len, &content_len, &has_content);
				lexer->mark_end(lexer);
				while (is_ident_continue(lexer->lookahead)) {
					int32_t c = lexer->lookahead;
					lexer->advance(lexer, false);
					record_fragment_char(c, prefix, &consumed_len, &content_len, &has_content);
					lexer->mark_end(lexer);
				}
			} else {
				break;
			}
			continue;
		}
		if (lexer->lookahead == '/') {
			lexer->advance(lexer, false);
			if (lexer->lookahead == '/' || lexer->lookahead == '*') {
				break;
			}
			record_fragment_char('/', prefix, &consumed_len, &content_len, &has_content);
			lexer->mark_end(lexer);
			continue;
		}
		int32_t c = lexer->lookahead;
		lexer->advance(lexer, false);
		record_fragment_char(c, prefix, &consumed_len, &content_len, &has_content);
		if (!is_horizontal_whitespace(c)) {
			lexer->mark_end(lexer);
		}
		if (c == ':' && first == '@' && valid_symbols[EVENT_ATTRIBUTE_HEAD]) {
			lexer->result_symbol = EVENT_ATTRIBUTE_HEAD;
			return true;
		}
		if (c == ':' && (first == '_' || (first >= 'a' && first <= 'z')) && valid_symbols[ATTRIBUTE_HEAD]) {
			lexer->result_symbol = ATTRIBUTE_HEAD;
			return true;
		}
		if (content_len == 4 && consumed_len >= 4 && prefix[0] == 'e' && prefix[1] == 'l' && prefix[2] == 's' &&
			prefix[3] == 'e' && !is_ident_continue(lexer->lookahead) && valid_symbols[ELSE_HEAD]) {
			lexer->result_symbol = ELSE_HEAD;
			return true;
		}
		if (has_keyword_prefix(prefix, consumed_len, "if", 2) && valid_symbols[IF_HEAD]) {
			return scan_control_head_tail(lexer, valid_symbols, IF_HEAD);
		}
		if (has_keyword_prefix(prefix, consumed_len, "for", 3) && valid_symbols[FOR_HEAD]) {
			return scan_control_head_tail(lexer, valid_symbols, FOR_HEAD);
		}
		if (has_keyword_prefix(prefix, consumed_len, "match", 5) && valid_symbols[MATCH_HEAD]) {
			return scan_control_head_tail(lexer, valid_symbols, MATCH_HEAD);
		}
	}

	if (!has_content) {
		return false;
	}
	if (content_len == 4 && consumed_len >= 4 && prefix[0] == 'e' && prefix[1] == 'l' && prefix[2] == 's' &&
		prefix[3] == 'e' && valid_symbols[ELSE_HEAD]) {
		lexer->result_symbol = ELSE_HEAD;
		return true;
	}
	if (is_rust_whitespace(lexer->lookahead)) {
		if (prefix_equals_keyword(prefix, content_len, "if", 2) && valid_symbols[IF_HEAD]) {
			return scan_control_head_tail(lexer, valid_symbols, IF_HEAD);
		}
		if (prefix_equals_keyword(prefix, content_len, "for", 3) && valid_symbols[FOR_HEAD]) {
			return scan_control_head_tail(lexer, valid_symbols, FOR_HEAD);
		}
		if (prefix_equals_keyword(prefix, content_len, "match", 5) && valid_symbols[MATCH_HEAD]) {
			return scan_control_head_tail(lexer, valid_symbols, MATCH_HEAD);
		}
	}
	if (valid_symbols[FRAGMENT]) {
		lexer->result_symbol = FRAGMENT;
		return true;
	}
	return false;
}

bool tree_sitter_reinhardt_page_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
	(void)payload;

	while (lexer->lookahead == ' ' || lexer->lookahead == '\t' || lexer->lookahead == '\n' || lexer->lookahead == '\r') {
		lexer->advance(lexer, true);
	}

	if (lexer->lookahead == '/') {
		lexer->advance(lexer, false);
		if (lexer->lookahead == '/' && valid_symbols[LINE_COMMENT]) {
			if (scan_line_comment_after_slash(lexer)) {
				lexer->result_symbol = LINE_COMMENT;
				return true;
			}
		}
		if (lexer->lookahead == '*' && valid_symbols[BLOCK_COMMENT]) {
			if (scan_block_comment_after_slash(lexer)) {
				lexer->result_symbol = BLOCK_COMMENT;
				return true;
			}
		}
		if (valid_symbols[FRAGMENT] && scan_fragment_tail(lexer, true)) {
			lexer->result_symbol = FRAGMENT;
			return true;
		}
		return false;
	}

	if (scan_classified_fragment(lexer, valid_symbols)) {
		return true;
	}
	return false;
}
