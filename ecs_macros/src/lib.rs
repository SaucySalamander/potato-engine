use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Index, Path, parse_macro_input};

#[proc_macro]
pub fn impl_query(input: TokenStream) -> TokenStream {
    let ecs_path: Path = parse_macro_input!(input as Path);
    let mut tokens = TokenStream2::new();

    for n in 1..=16 {
        let idents: Vec<_> = (0..n).map(|i| format_ident!("T{}", i)).collect();
        let vars: Vec<_> = (0..n).map(|i| format_ident!("v{}", i)).collect();
        let indices: Vec<_> = (0..n).map(Index::from).collect();

        // build zipped iterator: columns.0.iter().zip(columns.1.iter()) ...
        let mut zip_chain = quote! { columns.0.iter() };
        for i in 1..n {
            let idx = Index::from(i);
            let iter = quote! { columns.#idx.iter() };
            zip_chain = quote! { #zip_chain.zip(#iter) };
        }

        // build map pattern to destructure nested zip like (((v0, v1), v2), ...)
        let first_var = &vars[0];
        let mut map_pattern = quote! { #first_var };
        for var in &vars[1..] {
            map_pattern = quote! { (#map_pattern, #var) };
        }

        let map_return = if n == 1 {
            let v = &vars[0];
            quote! { (#v,) }
        } else {
            quote! { (#(#vars),*) }
        };

        tokens.extend(quote! {
            impl<'world, #(#idents: 'static),*> Query<'world> for (#(&'world #idents,)*) {
                type Item = (#(&'world #idents,)*);

                fn query_archetype(
                    archetype: &'world Archetype,
                    registry: &ComponentTypeIndexRegistry,
                ) -> Option<Box<dyn Iterator<Item = Self::Item> + 'world>> {
                    let indices = vec![
                        #(registry.get_index(std::any::TypeId::of::<#idents>())?),*
                    ];
                    use #ecs_path::archetypes::GetColumns;
                    let columns: (#(&'world Vec<#idents>,)*) = archetype.get_columns(&indices)?;
                    Some(Box::new(#zip_chain.map(|#map_pattern| #map_return)))
                }
            }
        });
    }

    eprintln!("{}", tokens.to_string());
    tokens.into()
}

#[proc_macro]
pub fn impl_query_combinations(input: TokenStream) -> TokenStream {
    let ecs_path: Path = parse_macro_input!(input as Path);
    const MAX_ARITY: usize = 4;
    let mut output = TokenStream2::new();

    for n in 1..=MAX_ARITY {
        let type_idents: Vec<_> = (0..n).map(|i| format_ident!("T{}", i)).collect();
        let var_idents: Vec<_> = (0..n).map(|i| format_ident!("v{}", i)).collect();

        let total_combinations = 1 << n;
        for mut_mask in 0..total_combinations {
            let mut_refs: Vec<bool> = (0..n).map(|i| (mut_mask >> i) & 1 == 1).collect();

            let ref_types: Vec<_> = type_idents
                .iter()
                .zip(&mut_refs)
                .map(|(ty, is_mut)| {
                    if *is_mut {
                        quote! { &'world mut #ty }
                    } else {
                        quote! { &'world #ty }
                    }
                })
                .collect();

            let item_type = quote! { (#(#ref_types),*) };

            let get_columns: Vec<_> = type_idents.iter().zip(&mut_refs).enumerate().map(|(i,(ty, is_mut))| {
                let col_indent = format_ident!("col_{}", i);
                let index = Index::from(i);
                if *is_mut {
                    quote! { let #col_indent: &'world mut Vec<#ty> = unsafe{&mut *ptr}.get_column_mut(indices[#index])?; }
                } else {
                    quote! { let #col_indent: &'world Vec<#ty> = unsafe{&mut *ptr}.get_column(indices[#index])?; }
                }
            }).collect();

            let mut zip_chain = {
                if mut_refs[0] {
                    quote! { col_0.iter_mut() }
                } else {
                    quote! { col_0.iter() }
                }
            };

            for (i, is_mut) in mut_refs.iter().enumerate().skip(1) {
                let col = format_ident!("col_{}", i);
                let iter = if *is_mut {
                    quote! { #col.iter_mut() }
                } else {
                    quote! { #col.iter() }
                };
                zip_chain = quote! { #zip_chain.zip(#iter) };
            }

            let first_var = &var_idents[0];
            let mut destructure = quote! { #first_var };
            for v in &var_idents[1..] {
                destructure = quote! { (#destructure, #v) };
            }

            let return_tuple = quote! { (#(#var_idents),*) };

            output.extend(quote! {
                impl<'world, #(#type_idents: 'static),*> Query<'world> for (#(#ref_types,)*) {
                    type Item = #item_type;

                    fn query_archetype(
                        archetype: &'world mut Archetype,
                        registry: &ComponentTypeIndexRegistry,
                    ) -> Option<Box<dyn Iterator<Item = Self::Item> + 'world>> {
                        use #ecs_path::archetypes::GetColumns;

                        let indices = vec![
                            #(registry.get_index(std::any::TypeId::of::<#type_idents>())?),*
                        ];

                        let ptr = archetype as *mut Archetype;

                        #(#get_columns)*

                        Some(Box::new(#zip_chain.map(|#destructure| #return_tuple)))
                    }
                }
            });
        }
    }

    // eprintln!("{}", output.to_string());
    output.into()
}
