#![deny(unsafe_code)]
#![warn(
	clippy::cargo,
	clippy::nursery,
	clippy::pedantic,
	clippy::restriction,
	future_incompatible,
	rust_2018_idioms
)]
#![warn(
	macro_use_extern_crate,
	meta_variable_misuse,
	missing_copy_implementations,
	missing_crate_level_docs,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	single_use_lifetimes,
	trivial_casts,
	trivial_numeric_casts,
	unaligned_references,
	unreachable_pub,
	unused_import_braces,
	unused_lifetimes,
	unused_qualifications,
	variant_size_differences
)]
#![allow(
	clippy::blanket_clippy_restriction_lints,
	clippy::else_if_without_else,
	clippy::expect_used,
	clippy::exhaustive_enums,
	clippy::exhaustive_structs,
	clippy::future_not_send,
	clippy::implicit_return,
	clippy::missing_inline_in_public_items,
	clippy::multiple_inherent_impl,
	clippy::non_ascii_literal,
	clippy::pattern_type_mismatch,
	clippy::redundant_pub_crate,
	clippy::shadow_reuse,
	clippy::tabs_in_doc_comments,
	clippy::unreachable,
	clippy::wildcard_enum_match_arm,
	unreachable_pub,
	variant_size_differences
)]
#![cfg_attr(
	doc,
	warn(rustdoc::all),
	allow(rustdoc::missing_doc_code_examples, rustdoc::private_doc_tests)
)]
// TODO: finish documentation
#![allow(
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc,
	missing_docs
)]
#![cfg_attr(doc, allow(rustdoc::all))]

mod error;
mod executor;
mod task;
mod worker;

pub use allochronic_macros::executor;
pub use executor::Executor;
pub use task::Task;
use worker::{Message, Runnables, Worker};
