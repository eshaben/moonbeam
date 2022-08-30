// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use super::*;

impl Precompile {
	pub fn try_from(_args: syn::AttributeArgs, impl_: &mut syn::ItemImpl) -> syn::Result<Self> {
		let struct_ident = Self::extract_struct_ident(impl_)?;
		let enum_ident = format_ident!("{}Call", struct_ident);

		let mut precompile = Precompile {
			struct_type: impl_.self_ty.as_ref().clone(),
			struct_ident,
			enum_ident,
			generics: impl_.generics.clone(),
			selector_to_variant: BTreeMap::new(),
			variants_content: BTreeMap::new(),
			fallback_to_variant: None,
			precompile_set: None,
		};

		for mut item in &mut impl_.items {
			// We only interact with methods and leave the rest as-is.
			if let syn::ImplItem::Method(ref mut method) = &mut item {
				precompile.process_method(method)?;
			}
		}

		Ok(precompile)
	}

	fn extract_struct_ident(impl_: &syn::ItemImpl) -> syn::Result<syn::Ident> {
		let type_path = match impl_.self_ty.as_ref() {
			syn::Type::Path(p) => p,
			_ => {
				let msg = "The type in the impl block must be a path, like `Precompile` or
				`example::Precompile`";
				return Err(syn::Error::new(impl_.self_ty.span(), msg));
			}
		};

		let final_path = type_path.path.segments.last().ok_or_else(|| {
			let msg = "The type path must be non empty.";
			syn::Error::new(impl_.self_ty.span(), msg)
		})?;

		Ok(final_path.ident.clone())
	}

	fn process_method(&mut self, method: &mut syn::ImplItemMethod) -> syn::Result<()> {
		// Take (remove) all attributes related to this macro.
		let attrs = attr::take_attributes::<attr::MethodAttr>(&mut method.attrs)?;

		// If there are no attributes it is a private function and we ignore it.
		if attrs.is_empty() {
			return Ok(());
		}

		// A method cannot have modifiers if it isn't a fallback and/or doesn't have a selector.
		let mut used = false;

		let method_name = method.sig.ident.clone();
		let mut modifier = Modifier::NonPayable;
		let mut solidity_arguments_type: Option<String> = None;
		let mut arguments = vec![];
		let mut is_fallback = false;
		let mut encode_selector = None;

		for attr in attrs {
			match attr {
				attr::MethodAttr::Fallback(span) => {
					if self.fallback_to_variant.is_some() {
						let msg = "A precompile can only have 1 fallback function";
						return Err(syn::Error::new(span, msg));
					}

					self.fallback_to_variant = Some(method_name.clone());
					used = true;
					is_fallback = true;
				}
				attr::MethodAttr::Payable(span) => {
					if modifier != Modifier::NonPayable {
						let msg =
							"A precompile method can have at most one modifier (payable, view)";
						return Err(syn::Error::new(span, msg));
					}

					modifier = Modifier::Payable;
				}
				attr::MethodAttr::View(span) => {
					if modifier != Modifier::NonPayable {
						let msg =
							"A precompile method can have at most one modifier (payable, view)";
						return Err(syn::Error::new(span, msg));
					}

					modifier = Modifier::View;
				}
				attr::MethodAttr::Public(_span, signature_lit) => {
					used = true;

					let signature = signature_lit.value();

					// Split signature to get arguments type.
					let split: Vec<_> = signature.splitn(2, "(").collect();
					if split.len() != 2 {
						let msg = "Selector must have form \"foo(arg1,arg2,...)\"";
						return Err(syn::Error::new(signature_lit.span(), msg));
					}

					let local_args_type = format!("({}", split[1]); // add back initial parenthesis

					if let Some(ref args_type) = &solidity_arguments_type {
						// If there are multiple public attributes we check that they all have
						// the same type.
						if args_type != &local_args_type {
							let msg = "Method cannot have multiple selectors with different types.";
							return Err(syn::Error::new(signature_lit.span(), msg));
						}
					} else {
						solidity_arguments_type = Some(local_args_type);
					}

					// Compute the 4-bytes selector.
					let digest = Keccak256::digest(signature.as_ref());
					let selector = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);

					if let Some(previous) = self
						.selector_to_variant
						.insert(selector, method_name.clone())
					{
						let msg =
							format!("Selector collision with method {}", previous.to_string());
						return Err(syn::Error::new(signature_lit.span(), msg));
					}

					if encode_selector.is_none() {
						encode_selector = Some(selector);
					}
				}
			}
		}

		if !used {
			let msg =
				"A precompile method cannot have modifiers without being a fallback or having\
			a `public` attribute";
			return Err(syn::Error::new(method.span(), msg));
		}

		// We forbid type parameters.
		if let Some(param) = method.sig.generics.params.first() {
			let msg = "Exposed precompile methods cannot have type parameters";
			return Err(syn::Error::new(param.span(), msg));
		}

		if method.sig.inputs.is_empty() {
			let msg = "Methods must have at least 1 parameter (the PrecompileHandle)";
			return Err(syn::Error::new(method.span(), msg));
		}

		if is_fallback {
			if let Some(input) = method.sig.inputs.iter().skip(1).next() {
				let msg =
					"Fallback methods cannot take any parameter outside of the PrecompileHandle";
				return Err(syn::Error::new(input.span(), msg));
			}
		}

		// We skip the first parameter which will be the PrecompileHandle.
		// Not having it or having a self parameter will produce a compilation error when
		// trying to call the functions with such PrecompileHandle.
		let method_inputs = method.sig.inputs.iter().skip(1);

		// We go through each parameter to collect each name and type that will be used to
		// generate the input enum and parse the call data.
		for input in method_inputs {
			let input = match input {
				syn::FnArg::Typed(t) => t,
				_ => {
					// I don't think it is possible to encounter this error since a self receiver
					// seems to only be possible in the first position which we skipped.
					let msg = "Exposed precompile methods cannot have a `self` parameter";
					return Err(syn::Error::new(input.span(), msg));
				}
			};

			let msg = "Parameter must be of the form `name: Type`, optionally prefixed by `mut`";
			let ident = match input.pat.as_ref() {
				syn::Pat::Ident(pat) => {
					if pat.by_ref.is_some() || pat.subpat.is_some() {
						return Err(syn::Error::new(pat.span(), msg));
					}

					pat.ident.clone()
				}
				_ => {
					return Err(syn::Error::new(input.pat.span(), msg));
				}
			};
			let ty = input.ty.as_ref().clone();

			arguments.push(Argument { ident, ty })
		}

		if let Some(_) = self.variants_content.insert(
			method_name.clone(),
			Variant {
				arguments,
				solidity_arguments_type: solidity_arguments_type.unwrap_or(String::from("()")),
				modifier,
				encode_selector,
			},
		) {
			let msg = "Duplicate method name";
			return Err(syn::Error::new(method_name.span(), msg));
		}

		Ok(())
	}
}
