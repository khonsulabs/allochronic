#![deny(unsafe_code)]
#![warn(
	clippy::cargo,
	clippy::nursery,
	clippy::pedantic,
	clippy::restriction,
	future_incompatible,
	rust_2018_idioms,
	rustdoc::all
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
	unused_crate_dependencies,
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
	clippy::multiple_inherent_impl,
	clippy::non_ascii_literal,
	clippy::pattern_type_mismatch,
	clippy::redundant_pub_crate,
	clippy::shadow_reuse,
	clippy::unreachable,
	clippy::wildcard_enum_match_arm,
	rustdoc::private_doc_tests,
	variant_size_differences
)]
#![allow(clippy::missing_docs_in_private_items, missing_docs)]

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
	parse::{Parse, ParseStream},
	punctuated::Punctuated,
	spanned::Spanned,
	Error, Expr, Ident, ItemFn, Path, Token,
};

fn error(span: Span, message: &str) -> TokenStream1 {
	Error::new(span, message).into_compile_error().into()
}

fn select_task_internal(notified: Option<Expr>) -> TokenStream {
	let (notified, shutdown, finished) = if let Some(notified) = notified {
		let notified = quote::quote! {_ = #notified => Message::Nested,};

		(vec![notified], vec![], vec![])
	} else {
		let shutdown = quote::quote! {
			message = worker.shutdown.select_next_some() => Message::Management(message),
		};
		let finished =
			quote::quote! {message = finished.select_next_some() => Message::Management(message),};

		(vec![], vec![shutdown], vec![finished])
	};

	quote::quote! {
		{
			use ::futures_lite::future::block_on;
			use ::futures_util::{select_biased, StreamExt};
			use ::std::ops::DerefMut;
			use crate::{message::Message, worker::{Runtime, Worker}};

			Worker::with_mut(|mut worker| {
				let worker = worker.deref_mut();

				// TODO: fix in Clippy
				#[allow(clippy::mut_mut)]
				#[allow(clippy::panic, unused_variables)]
				block_on(async {
					match &mut worker.runtime {
						Runtime::Async {
							management,
							local_prio,
							global_prio,
							steal_prio,
							local_normal,
							global_normal,
							steal_normal,
						} => {
							select_biased![
								#(#shutdown)*
								#(#notified)*
								message = management.select_next_some() => Message::Task(message),
								message = local_prio.select_next_some() => Message::Task(message),
								message = global_prio.select_next_some() => Message::Task(message),
								message = steal_prio.select_next_some() => Message::Task(message),
								message = local_normal.select_next_some() => Message::Task(message),
								message = global_normal.select_next_some() => {
									Message::Task(message)
								}
								message = steal_normal.select_next_some() => Message::Task(message),
							]
						}
						Runtime::Blocking { local_prio, local_normal, finished, .. } => {
							select_biased![
								#(#shutdown)*
								#(#notified)*
								message = local_prio.select_next_some() => Message::Task(message),
								message = local_normal.select_next_some() => Message::Task(message),
								#(#finished)*
							]
						}
						Runtime::Alien { .. } => {
							unreachable!("`Worker` started in an alien runtime")
						}
					}
				})
			})
		}
	}
}

#[proc_macro]
pub fn select_task(_item: TokenStream1) -> TokenStream1 {
	select_task_internal(None).into()
}

struct NestedArgs {
	fun: Expr,
}

impl Parse for NestedArgs {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let args = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;
		let mut args = args.iter();

		let fun = args
			.next()
			.ok_or_else(|| Error::new(input.span(), "one argument expected"))?;

		args.next().map_or_else(
			|| Ok(Self { fun: fun.clone() }),
			|arg| Err(Error::new(arg.span(), "only one argument expected")),
		)
	}
}

#[proc_macro]
pub fn select_task_nested(args: TokenStream1) -> TokenStream1 {
	let args: NestedArgs = syn::parse_macro_input!(args);
	select_task_internal(Some(args.fun)).into()
}

struct ExecutorArgs {
	path: TokenStream,
}

impl Parse for ExecutorArgs {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		if input.is_empty() {
			Ok(Self {
				path: quote! {::allochronic},
			})
		} else {
			let name = input.parse::<Ident>()?;
			let _: Token![=] = input.parse()?;
			let value: Path = input.parse()?;

			// skip next token if it's a comma
			let _: Option<Token![,]> = input.parse()?;

			if name == "package" {
				Ok(Self {
					path: quote! {#value},
				})
			} else {
				Err(Error::new(name.span(), "expected package"))
			}
		}
	}
}

#[proc_macro_attribute]
pub fn executor(args: TokenStream1, item: TokenStream1) -> TokenStream1 {
	let args: ExecutorArgs = syn::parse_macro_input!(args);
	let item: ItemFn = syn::parse_macro_input!(item);

	// extract relevant stuff from item
	let fn_attrs = item.attrs;
	let fn_vis = item.vis;
	let mut fn_sig = item.sig;
	#[allow(box_pointers)]
	let fn_body = item.block;

	if fn_sig.asyncness.take().is_none() {
		return error(fn_sig.span(), "missing async");
	}

	let path = args.path;

	(quote! {
		#(#fn_attrs)*
		#fn_vis #fn_sig {
			#path::Executor::start(async move {
				#fn_body
			})
		}
	})
	.into()
}
