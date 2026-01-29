//! DML Integration Tests for reinhardt-query
//!
//! This is the entry point for all DML operation tests.

mod fixtures;

mod common_assertions;
mod delete_combinations;
mod delete_edge_cases;
mod delete_error_path;
mod delete_happy_path;
mod insert_combinations;
mod insert_edge_cases;
mod insert_error_path;
mod insert_happy_path;
mod select_aggregations;
mod select_combinations;
mod select_edge_cases;
mod select_error_path;
mod select_happy_path;
mod select_joins;
mod update_combinations;
mod update_edge_cases;
mod update_error_path;
mod update_happy_path;
