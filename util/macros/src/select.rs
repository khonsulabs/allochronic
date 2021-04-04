//! An alternative to other [`select!`](crate::select!) macros that doesn't drop
//! [`Future`](std::future::Future)s when they are resolved but not picked due
//! to priority.

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
	parse::{Parse, ParseStream},
	spanned::Spanned,
	Error, Expr, Pat, Path, Token,
};

/// Destructuring [`TokenStream`] for [`select!`](crate::select!).
struct Items {
	/// Name of the crate allochronic-util.
	package: TokenStream,
	/// Items passed to [`select!`](crate::select!).
	items: Vec<Item>,
	/// Expression to be run on completion of all passed items.
	complete: Option<Expr>,
}

/// Destructuring of a single item of [`select!`](crate::select!).
struct Item {
	/// Pattern passed to [`success`][Item::success].
	var: Pat,
	/// [`Future`](std::future::Future) or
	/// [`Stream`](std::stream::Stream) to be polled.
	future: Expr,
	/// Expression to be run on polling success.
	success: Expr,
}

impl Parse for Items {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let mut package = quote! { allochronic_util };
		let mut items = Vec::new();
		let mut complete = None;

		loop {
			let var = if let Ok(var) = input.parse::<Pat>() {
				var
			} else {
				break Ok(Self {
					package,
					items,
					complete,
				})
			};

			// if the next token isn't a ":" it could be the `complete` or `package` item
			let result = input.parse::<Token![:]>();

			match result {
				// we found a ":" token, continue like intended
				Ok(_) => {
					let future = input.parse()?;
					let _: Token![=>] = input.parse()?;
					let success = input.parse()?;

					items.push(Item {
						var,
						future,
						success,
					});

					// the next token is only allowed to be a "," as a separator between items
					if input.parse::<Option<Token![,]>>()?.is_none() && !input.is_empty() {
						break Err(Error::new(input.span(), "missing a comma between items"))
					}
				}
				// if the next item is `complete`, the token afterwards has to be "=>"
				Err(_)
					if var.to_token_stream().to_string() == "complete"
						&& input.parse::<Token![=>]>().is_ok() =>
				{
					complete = Some(input.parse()?);
					// a trailing "," token is no problem
					let _: Option<Token![,]> = input.parse()?;

					// but nothing is allowed to come afterwards
					break if input.is_empty() {
						Ok(Self {
							package,
							items,
							complete,
						})
					} else {
						Err(Error::new(
							var.span(),
							"`complete` needs to be the last item",
						))
					}
				}
				// if the next item is `package`, the token afterwards has to be "=>"
				Err(_)
					if var.to_token_stream().to_string() == "package"
						&& input.parse::<Token![=>]>().is_ok() =>
				{
					package = input.parse::<Path>()?.into_token_stream();
					// a trailing "," token is no problem
					let _: Option<Token![,]> = input.parse()?;

					// but nothing is allowed to come before
					if !items.is_empty() || complete.is_some() {
						break Err(Error::new(
							var.span(),
							"`package` needs to be the first item",
						))
					}
				}
				// otherwise, it was just the wrong token
				Err(error) => break Err(error),
			}
		}
	}
}

/// See [`select`](crate::select!).
pub(crate) fn select(item: TokenStream1) -> TokenStream1 {
	let Items {
		package,
		items,
		complete,
	} = syn::parse_macro_input!(item);

	if items.is_empty() {
		return crate::error(Span::call_site(), "`select!` can't be empty")
	}

	if let Some(complete) = complete {
		let mut stream = TokenStream::new();

		for Item {
			var,
			future,
			success,
		} in items
		{
			stream.extend(quote! {
				match (#future).__into_poll_fused().await {
					::std::task::Poll::Ready(Some(#var)) => {
						break ::std::option::Option::Some(#success)
					}
					::std::task::Poll::Ready(None) => (),
					::std::task::Poll::Pending => __complete = false,
				}
			});
		}

		(quote! {
			{
				use ::#package::select::{PollFutures as _, PollStreams as _};

				let __result = loop {
					let mut __complete = true;

					#stream

					if __complete {
						break ::std::option::Option::None;
					} else {
						::#package::r#yield().await;
					}
				};

				if let ::std::option::Option::Some(__result) = __result {
					__result
				} else {
					#complete
				}
			}
		})
		.into()
	} else {
		let mut stream = TokenStream::new();

		for Item {
			var,
			future,
			success,
		} in items
		{
			stream.extend(quote! {
				if let ::std::task::Poll::Ready(#var) = (#future).__into_poll().await {
					break #success;
				}
			});
		}

		(quote! {
			loop {
				use ::#package::select::{PollFutures as _, PollStreams as _};

				#stream

				::#package::r#yield().await;
			}
		})
		.into()
	}
}
