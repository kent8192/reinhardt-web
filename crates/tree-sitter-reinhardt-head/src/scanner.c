#include "tree_sitter/parser.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

enum TokenType {
	LINE_COMMENT,
	BLOCK_COMMENT,
	RAW_STRING,
	FRAGMENT,
};

void *tree_sitter_reinhardt_head_external_scanner_create(void) { return NULL; }
void tree_sitter_reinhardt_head_external_scanner_destroy(void *payload) { (void)payload; }
void tree_sitter_reinhardt_head_external_scanner_reset(void *payload) { (void)payload; }
unsigned tree_sitter_reinhardt_head_external_scanner_serialize(void *payload, char *buffer) {
	(void)payload;
	(void)buffer;
	return 0;
}
void tree_sitter_reinhardt_head_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
	(void)payload;
	(void)buffer;
	(void)length;
}

static bool is_structural_delimiter(int32_t c) {
	return c == 0 || c == '\n' || c == '\r' || c == '{' || c == '}' || c == '[' || c == ']' ||
		   c == '(' || c == ')' || c == '"' || c == '\'' || c == ',' || c == ';';
}

static bool scan_fragment_tail(TSLexer *lexer, bool has_content) {
	while (!is_structural_delimiter(lexer->lookahead)) {
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

static bool scan_r_prefixed(TSLexer *lexer, bool *is_raw_string) {
	*is_raw_string = false;
	lexer->advance(lexer, false);
	lexer->mark_end(lexer);
	unsigned hashes = 0;
	while (lexer->lookahead == '#') {
		hashes++;
		lexer->advance(lexer, false);
		lexer->mark_end(lexer);
	}
	if (lexer->lookahead != '"') {
		return scan_fragment_tail(lexer, true);
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
				*is_raw_string = true;
				return true;
			}
		} else {
			lexer->advance(lexer, false);
		}
	}
	return false;
}

static bool scan_fragment(TSLexer *lexer) {
	if (lexer->lookahead == ' ' || lexer->lookahead == '\t' || is_structural_delimiter(lexer->lookahead)) {
		return false;
	}
	return scan_fragment_tail(lexer, false);
}

bool tree_sitter_reinhardt_head_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
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

	if (lexer->lookahead == 'r' && (valid_symbols[RAW_STRING] || valid_symbols[FRAGMENT])) {
		bool is_raw_string = false;
		if (scan_r_prefixed(lexer, &is_raw_string)) {
			if (is_raw_string && valid_symbols[RAW_STRING]) {
				lexer->result_symbol = RAW_STRING;
			} else if (valid_symbols[FRAGMENT]) {
				lexer->result_symbol = FRAGMENT;
			} else {
				return false;
			}
			return true;
		}
		return false;
	}

	if (valid_symbols[FRAGMENT] && scan_fragment(lexer)) {
		lexer->result_symbol = FRAGMENT;
		return true;
	}
	return false;
}
