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
	box_pointers,
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
	unused_results,
	variant_size_differences
)]
#![allow(
	clippy::blanket_clippy_restriction_lints,
	clippy::else_if_without_else,
	clippy::exhaustive_enums,
	clippy::future_not_send,
	clippy::implicit_return,
	clippy::missing_inline_in_public_items,
	clippy::non_ascii_literal,
	clippy::pattern_type_mismatch,
	clippy::redundant_pub_crate,
	clippy::shadow_reuse,
	clippy::tabs_in_doc_comments,
	clippy::unreachable,
	clippy::wildcard_enum_match_arm,
	variant_size_differences
)]
#![allow(clippy::cargo_common_metadata)]
#![cfg_attr(
	doc,
	warn(rustdoc::all),
	allow(rustdoc::missing_doc_code_examples, rustdoc::private_doc_tests)
)]

//! Proc-Macros for the `allochronic-util` crate.

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::Error;

mod select;

/// Easy generation of errors in proc macros.
fn error(span: Span, message: &str) -> TokenStream {
	Error::new(span, message).into_compile_error().into()
}

/// Polls all passed [`Future`](std::future::Future)s or
/// [`Stream`](std::stream::Stream)s until one is ready and returns
/// it. Prioritizes by order.
///
/// # Raw-mode
/// If the `complete` item is unspecified, [`Future`](std::future::Future)s and
/// [`Stream`](std::stream::Stream)s will be pulled beyond exhaustion. If a high
/// priority item is exhausted and [`select!`](crate::select!) is being used in
/// a loop, it will continuously return the same result, which might be
/// unintended.
///
/// # Fused-mode
/// If the `complete` item is specified, all polled items have to implement
/// `FusedFuture` or `FusedStream`, for [`Stream`](std::stream::Stream)s this
/// will unwrap the [`Option`]. The `complete` item will be executed once all
/// items have been exhausted. Items will not be pulled beyond exhaustion.
///
/// # Examples
/// Raw-mode:
/// ```
/// # futures_executor::block_on(async {
/// use futures_util::stream;
/// use allochronic_util::select;
///
/// let mut stream1 = stream::iter(1..=3);
/// let mut stream2 = stream::iter(4..=6);
/// let mut stream3 = stream::iter(7..=9);
///
/// let mut counter = 0;
///
/// while let Some(result) = select![
/// 	result: &mut stream1 => result,
/// 	result: &mut stream2 => result,
/// 	result: &mut stream3 => result,
/// ] {
/// 	counter += 1;
/// 	assert_eq!(counter, result);
/// }
///
/// // this loop will only reach the number of three, because it will abort after the first stream
/// // is exhausted
/// assert_eq!(counter, 3);
/// # });
/// ```
/// Fused-mode:
/// ```
/// # futures_executor::block_on(async {
/// use futures_util::stream::{self, StreamExt};
/// use allochronic_util::select;
///
/// let mut stream1 = stream::iter(1..=3).fuse();
/// let mut stream2 = stream::iter(4..=6).fuse();
/// let mut stream3 = stream::iter(7..=9).fuse();
///
/// let mut counter = 0;
///
/// while let Some(result) = select![
/// 	result: &mut stream1 => Some(result),
/// 	result: &mut stream2 => Some(result),
/// 	result: &mut stream3 => Some(result),
/// 	complete => None,
/// ] {
/// 	counter += 1;
/// 	assert_eq!(counter, result);
/// }
///
/// assert_eq!(counter, 9);
/// # });
/// ```
#[proc_macro]
pub fn select(item: TokenStream) -> TokenStream {
	select::select(item)
}
