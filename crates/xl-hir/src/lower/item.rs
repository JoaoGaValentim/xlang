//! Item definition collection, signature lowering, and body lowering.
//!
//! Written against the documented allocation order (module scan → item
//! definitions; signatures; bodies) and the child-index schema in
//! [`crate::origin`].

use xl_ast::{FnDecl, Generics, ImplItem, Item, ItemKind, Param, Type};

use super::{FnSkeleton, ItemSkeleton, Lowerer, SigSkeleton};
use crate::fate::Aborted;
use crate::hir::{
    BodyId, DefId, DefKind, HirItemKind, ModuleBinding, ModuleBindingTarget, Visibility,
};
use crate::origin::{AnchorId, AstOrigin};

impl Lowerer {
    // ------------------------------------------------------------------
    // Pass 1: definition collection
    // ------------------------------------------------------------------

    /// Collects the definitions of one item.
    pub(super) fn item_skeleton(&mut self, item: &Item) -> Result<ItemSkeleton, Aborted> {
        self.with_anchor(AnchorId::Item(item.id), |lowerer| {
            lowerer.item_skeleton_inner(item)
        })
    }

    pub(super) fn item_skeleton_inner(&mut self, item: &Item) -> Result<ItemSkeleton, Aborted> {
        let item_origin = AstOrigin::Item(item.id);
        match &item.kind {
            ItemKind::Fn(decl) => {
                let def = self.reserve_def(
                    decl.sig.name.name.clone(),
                    decl.sig.name.span,
                    decl.vis.clone(),
                    None,
                    item_origin.clone(),
                )?;
                let skeleton = self.reserve_fn_skeleton(decl, def)?;
                self.alloc_item(HirItemKind::Fn, def, item.span, item_origin)?;
                Ok(ItemSkeleton {
                    def,
                    kind: super::SkeletonKind::Fn(skeleton),
                })
            }
            ItemKind::Struct(decl) => {
                let def = self.reserve_def(
                    decl.name.name.clone(),
                    decl.name.span,
                    decl.vis.clone(),
                    None,
                    item_origin.clone(),
                )?;
                // child 0: generics
                self.push_child(0);
                let generics = self.reserve_generics(&decl.generics, def)?;
                self.pop_child();
                let mut fields = Vec::with_capacity(decl.fields.len());
                for (index, field) in decl.fields.iter().enumerate() {
                    // child 1 + i: field i
                    self.push_child(1 + index as u32);
                    let field_def = self.reserve_def(
                        field.name.name.clone(),
                        field.name.span,
                        field.vis.clone(),
                        Some(def),
                        self.structural_origin(),
                    )?;
                    self.pop_child();
                    fields.push(field_def);
                }
                self.alloc_item(HirItemKind::Struct, def, item.span, item_origin)?;
                Ok(ItemSkeleton {
                    def,
                    kind: super::SkeletonKind::Struct { generics, fields },
                })
            }
            ItemKind::Enum(decl) => {
                let def = self.reserve_def(
                    decl.name.name.clone(),
                    decl.name.span,
                    decl.vis.clone(),
                    None,
                    item_origin.clone(),
                )?;
                self.push_child(0);
                let generics = self.reserve_generics(&decl.generics, def)?;
                self.pop_child();
                let mut variants = Vec::with_capacity(decl.variants.len());
                for (index, variant) in decl.variants.iter().enumerate() {
                    self.push_child(1 + index as u32);
                    let variant_def = self.reserve_def(
                        variant.name.name.clone(),
                        variant.name.span,
                        Visibility::Private,
                        Some(def),
                        self.structural_origin(),
                    )?;
                    self.pop_child();
                    variants.push(variant_def);
                }
                self.alloc_item(HirItemKind::Enum, def, item.span, item_origin)?;
                Ok(ItemSkeleton {
                    def,
                    kind: super::SkeletonKind::Enum { generics, variants },
                })
            }
            ItemKind::Trait(decl) => {
                let def = self.reserve_def(
                    decl.name.name.clone(),
                    decl.name.span,
                    decl.vis.clone(),
                    None,
                    item_origin.clone(),
                )?;
                self.push_child(0);
                let generics = self.reserve_generics(&decl.generics, def)?;
                self.pop_child();
                let mut items = Vec::with_capacity(decl.items.len());
                for (index, trait_item) in decl.items.iter().enumerate() {
                    self.push_child(1 + index as u32);
                    let item_def = self.reserve_def(
                        trait_item.sig.name.name.clone(),
                        trait_item.sig.name.span,
                        Visibility::Private,
                        Some(def),
                        self.structural_origin(),
                    )?;
                    // child 0: the trait item's signature
                    self.push_child(0);
                    let params = self.reserve_params(&trait_item.sig.params, item_def)?;
                    let sig = self.reserve_sig(item_def, trait_item.sig.span)?;
                    self.pop_child();
                    self.pop_child();
                    self.fill_def(item_def, DefKind::TraitItem { sig });
                    items.push(SigSkeleton {
                        def: item_def,
                        params,
                        sig,
                    });
                }
                self.alloc_item(HirItemKind::Trait, def, item.span, item_origin)?;
                Ok(ItemSkeleton {
                    def,
                    kind: super::SkeletonKind::Trait { generics, items },
                })
            }
            ItemKind::Impl(decl) => {
                let def = self.reserve_def(
                    String::new(),
                    decl.span,
                    Visibility::Private,
                    None,
                    item_origin.clone(),
                )?;
                self.push_child(0);
                let generics = self.reserve_generics(&decl.generics, def)?;
                self.pop_child();
                let mut methods = Vec::with_capacity(decl.items.len());
                for (index, impl_item) in decl.items.iter().enumerate() {
                    let ImplItem::Fn(method) = impl_item;
                    // child 3 + i: impl item i
                    self.push_child(3 + index as u32);
                    let method_def = self.reserve_def(
                        method.sig.name.name.clone(),
                        method.sig.name.span,
                        method.vis.clone(),
                        Some(def),
                        self.structural_origin(),
                    )?;
                    let skeleton = self.reserve_fn_skeleton(method, method_def)?;
                    self.pop_child();
                    methods.push(skeleton);
                }
                self.alloc_item(HirItemKind::Impl, def, item.span, item_origin)?;
                Ok(ItemSkeleton {
                    def,
                    kind: super::SkeletonKind::Impl { generics, methods },
                })
            }
            ItemKind::Import(decl) => {
                let binding = self.lower_import(item, decl, item_origin)?;
                Ok(ItemSkeleton {
                    def: binding,
                    kind: super::SkeletonKind::Import,
                })
            }
        }
    }

    /// Reserves the generics, parameters, and signature of a function-like
    /// declaration (relative to the declaration's child schema).
    pub(super) fn reserve_fn_skeleton(
        &mut self,
        decl: &FnDecl,
        def: DefId,
    ) -> Result<FnSkeleton, Aborted> {
        // child 0: generics
        self.push_child(0);
        let generics = self.reserve_generics(&decl.generics, def)?;
        self.pop_child();
        // child 1: signature
        self.push_child(1);
        let params = self.reserve_params(&decl.sig.params, def)?;
        let sig = self.reserve_sig(def, decl.sig.span)?;
        self.pop_child();
        Ok(FnSkeleton {
            def,
            generics,
            params,
            sig,
        })
    }

    /// Reserves the generic parameter definitions (assumes the caller
    /// pushed the generics slot; params occupy child `i`).
    pub(super) fn reserve_generics(
        &mut self,
        generics: &Option<Generics>,
        parent: DefId,
    ) -> Result<Vec<DefId>, Aborted> {
        let mut defs = Vec::new();
        if let Some(generics) = generics {
            for (index, param) in generics.params.iter().enumerate() {
                self.push_child(index as u32);
                let def = self.reserve_def(
                    param.name.name.clone(),
                    param.name.span,
                    Visibility::Private,
                    Some(parent),
                    self.structural_origin(),
                )?;
                self.pop_child();
                defs.push(def);
            }
        }
        Ok(defs)
    }

    /// Reserves the parameter (and receiver) definitions (assumes the
    /// caller pushed the signature slot; params occupy child `i`).
    pub(super) fn reserve_params(
        &mut self,
        params: &[Param],
        parent: DefId,
    ) -> Result<Vec<DefId>, Aborted> {
        let mut defs = Vec::with_capacity(params.len());
        for (index, param) in params.iter().enumerate() {
            self.push_child(index as u32);
            let def = match param {
                Param::Receiver(span) => self.alloc_def(
                    "self".to_string(),
                    DefKind::Receiver {
                        index: index as u32,
                    },
                    *span,
                    Visibility::Private,
                    Some(parent),
                    self.structural_origin(),
                )?,
                Param::Named { name, .. } => self.reserve_def(
                    name.name.clone(),
                    name.span,
                    Visibility::Private,
                    Some(parent),
                    self.structural_origin(),
                )?,
            };
            self.pop_child();
            defs.push(def);
        }
        Ok(defs)
    }

    /// Lowers one `import` declaration (records, uses, and binding) and
    /// returns the binding definition.
    pub(super) fn lower_import(
        &mut self,
        item: &Item,
        decl: &xl_ast::ImportDecl,
        item_origin: AstOrigin,
    ) -> Result<DefId, Aborted> {
        let path: Vec<String> = decl
            .path
            .iter()
            .map(|segment| segment.name.clone())
            .collect();
        let alias = decl.alias.as_ref().map(|alias| alias.name.clone());
        let binding_name = match (&alias, decl.path.last()) {
            (Some(alias), _) => alias.clone(),
            (None, Some(segment)) => segment.name.clone(),
            (None, None) => String::new(),
        };
        let binding_span = match (&decl.alias, decl.path.last()) {
            (Some(alias), _) => alias.span,
            (None, Some(segment)) => segment.span,
            (None, None) => decl.span,
        };
        // child i: path segment i; child n: alias.
        let mut path_uses = Vec::with_capacity(decl.path.len());
        for (index, segment) in decl.path.iter().enumerate() {
            self.push_child(index as u32);
            let use_ = self.alloc_source_use(
                segment.name.clone(),
                segment.span,
                self.structural_origin(),
            )?;
            self.pop_child();
            path_uses.push(use_);
        }
        // The binding's origin is its name occurrence: the alias when
        // written, otherwise the last segment. A recovered empty path has
        // no name occurrence, so the item's own origin is recorded.
        let origin = match (decl.alias.is_some(), decl.path.len()) {
            (true, n) => {
                self.push_child(n as u32);
                let origin = self.structural_origin();
                self.pop_child();
                origin
            }
            (false, 0) => item_origin.clone(),
            (false, n) => {
                self.push_child(n as u32 - 1);
                let origin = self.structural_origin();
                self.pop_child();
                origin
            }
        };
        let binding = self.alloc_def(
            binding_name,
            DefKind::ModuleBinding(ModuleBinding {
                path,
                alias,
                span: decl.span,
                target: ModuleBindingTarget::Unresolved,
            }),
            binding_span,
            Visibility::Private,
            None,
            origin,
        )?;
        let hir_item = self.alloc_item(HirItemKind::Import, binding, item.span, item_origin)?;
        self.module.imports.push(crate::hir::ImportRecord {
            item: hir_item,
            binding,
            path: path_uses,
            decl_span: decl.span,
        });
        Ok(binding)
    }

    // ------------------------------------------------------------------
    // Pass 2: signature lowering
    // ------------------------------------------------------------------

    /// Lowers one item's signature (types, generics, headers).
    pub(super) fn item_signature(
        &mut self,
        item: &Item,
        skeleton: &ItemSkeleton,
    ) -> Result<(), Aborted> {
        let in_self = matches!(item.kind, ItemKind::Trait(_) | ItemKind::Impl(_));
        if in_self {
            self.self_ctx = self.self_ctx.saturating_add(1);
        }
        let result = self.with_anchor(AnchorId::Item(item.id), |lowerer| {
            lowerer.item_signature_inner(item, skeleton)
        });
        if in_self {
            self.self_ctx = self.self_ctx.saturating_sub(1);
        }
        result
    }

    pub(super) fn item_signature_inner(
        &mut self,
        item: &Item,
        skeleton: &ItemSkeleton,
    ) -> Result<(), Aborted> {
        match (&item.kind, &skeleton.kind) {
            (ItemKind::Fn(decl), super::SkeletonKind::Fn(fn_skel)) => {
                self.push_child(0);
                self.lower_generic_bounds(&decl.generics, &fn_skel.generics)?;
                self.pop_child();
                self.push_child(1);
                self.lower_signature(
                    &decl.sig.params,
                    &fn_skel.params,
                    &decl.sig.ret,
                    fn_skel.sig,
                )?;
                self.pop_child();
                Ok(())
            }
            (ItemKind::Struct(decl), super::SkeletonKind::Struct { generics, fields }) => {
                self.push_child(0);
                self.lower_generic_bounds(&decl.generics, generics)?;
                self.pop_child();
                for (index, (field, field_def)) in decl.fields.iter().zip(fields).enumerate() {
                    self.push_child(1 + index as u32);
                    // child 0: the field's type
                    self.push_child(0);
                    let ty = self.lower_type(&field.ty)?;
                    self.pop_child();
                    self.pop_child();
                    self.fill_def(*field_def, DefKind::Field { ty });
                }
                self.fill_def(
                    skeleton.def,
                    DefKind::Struct {
                        generics: generics.clone(),
                        fields: fields.clone(),
                    },
                );
                Ok(())
            }
            (ItemKind::Enum(decl), super::SkeletonKind::Enum { generics, variants }) => {
                self.push_child(0);
                self.lower_generic_bounds(&decl.generics, generics)?;
                self.pop_child();
                for (index, (variant, variant_def)) in
                    decl.variants.iter().zip(variants).enumerate()
                {
                    self.push_child(1 + index as u32);
                    let mut payload = Vec::with_capacity(variant.payload.len());
                    for (payload_index, ty) in variant.payload.iter().enumerate() {
                        self.push_child(payload_index as u32);
                        let ty_ref = self.lower_type(ty)?;
                        self.pop_child();
                        payload.push(ty_ref);
                    }
                    self.pop_child();
                    self.fill_def(*variant_def, DefKind::Variant { payload });
                }
                self.fill_def(
                    skeleton.def,
                    DefKind::Enum {
                        generics: generics.clone(),
                        variants: variants.clone(),
                    },
                );
                Ok(())
            }
            (ItemKind::Trait(decl), super::SkeletonKind::Trait { generics, items }) => {
                self.push_child(0);
                self.lower_generic_bounds(&decl.generics, generics)?;
                self.pop_child();
                for (index, (trait_item, sig_skel)) in decl.items.iter().zip(items).enumerate() {
                    self.push_child(1 + index as u32);
                    // child 0: the trait item's signature
                    self.push_child(0);
                    self.lower_signature(
                        &trait_item.sig.params,
                        &sig_skel.params,
                        &trait_item.sig.ret,
                        sig_skel.sig,
                    )?;
                    self.pop_child();
                    self.pop_child();
                }
                self.fill_def(
                    skeleton.def,
                    DefKind::Trait {
                        generics: generics.clone(),
                        items: items.iter().map(|item| item.def).collect(),
                    },
                );
                Ok(())
            }
            (ItemKind::Impl(decl), super::SkeletonKind::Impl { generics, methods }) => {
                self.push_child(0);
                self.lower_generic_bounds(&decl.generics, generics)?;
                self.pop_child();
                // child 1: the implemented trait; child 2: the self type.
                let trait_ref = match &decl.trait_ {
                    Some(trait_path) => {
                        self.push_child(1);
                        let ty = self.lower_path_type(trait_path)?;
                        self.pop_child();
                        Some(ty)
                    }
                    None => None,
                };
                self.push_child(2);
                let self_ty = self.lower_path_type(&decl.self_ty)?;
                self.pop_child();
                for (index, (impl_item, method_skel)) in decl.items.iter().zip(methods).enumerate()
                {
                    let ImplItem::Fn(method) = impl_item;
                    self.push_child(3 + index as u32);
                    self.push_child(0);
                    self.lower_generic_bounds(&method.generics, &method_skel.generics)?;
                    self.pop_child();
                    self.push_child(1);
                    self.lower_signature(
                        &method.sig.params,
                        &method_skel.params,
                        &method.sig.ret,
                        method_skel.sig,
                    )?;
                    self.pop_child();
                    self.pop_child();
                }
                self.fill_def(
                    skeleton.def,
                    DefKind::Impl {
                        generics: generics.clone(),
                        trait_ref,
                        self_ty,
                        is_inherent: decl.trait_.is_none(),
                    },
                );
                Ok(())
            }
            (ItemKind::Import(_), super::SkeletonKind::Import) => Ok(()),
            _ => Ok(()),
        }
    }

    /// Lowers the generic bounds of an item (assumes the caller pushed the
    /// generics slot; params occupy child `i`, the bound child `1`).
    pub(super) fn lower_generic_bounds(
        &mut self,
        generics: &Option<Generics>,
        defs: &[DefId],
    ) -> Result<(), Aborted> {
        if let Some(generics) = generics {
            for (index, (param, def)) in generics.params.iter().zip(defs).enumerate() {
                self.push_child(index as u32);
                // child 1: the bound (child 0 is the name, which receives
                // the parameter's definition).
                let bound = match &param.bound {
                    Some(bound) => {
                        self.push_child(1);
                        let ty = self.lower_path_type(bound)?;
                        self.pop_child();
                        Some(ty)
                    }
                    None => None,
                };
                self.pop_child();
                self.fill_def(*def, DefKind::GenericParam { bound });
            }
        }
        Ok(())
    }

    /// Lowers a signature's parameter types and return type (assumes the
    /// caller pushed the signature slot; params occupy child `i`, the
    /// return type child `n`).
    pub(super) fn lower_signature(
        &mut self,
        params: &[Param],
        defs: &[DefId],
        ret: &Option<Type>,
        sig: crate::hir::FnSigId,
    ) -> Result<(), Aborted> {
        for (index, param) in params.iter().enumerate() {
            if let Param::Named { ty, .. } = param {
                self.push_child(index as u32);
                // child 0: the parameter's type
                self.push_child(0);
                let ty_ref = self.lower_type(ty)?;
                self.pop_child();
                self.pop_child();
                if let Some(def) = defs.get(index) {
                    self.fill_def(
                        *def,
                        DefKind::Param {
                            ty: ty_ref,
                            index: index as u32,
                        },
                    );
                }
            }
        }
        let ret_ref = match ret {
            Some(ret) => {
                self.push_child(params.len() as u32);
                let ty_ref = self.lower_type(ret)?;
                self.pop_child();
                Some(ty_ref)
            }
            None => None,
        };
        self.fill_sig(sig, defs.to_vec(), ret_ref);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 3: body lowering
    // ------------------------------------------------------------------

    /// Lowers one item's bodies.
    pub(super) fn item_body(
        &mut self,
        item: &Item,
        skeleton: &ItemSkeleton,
    ) -> Result<(), Aborted> {
        let in_self = matches!(item.kind, ItemKind::Impl(_));
        if in_self {
            self.self_ctx = self.self_ctx.saturating_add(1);
        }
        let result = self.with_anchor(AnchorId::Item(item.id), |lowerer| {
            lowerer.item_body_inner(item, skeleton)
        });
        if in_self {
            self.self_ctx = self.self_ctx.saturating_sub(1);
        }
        result
    }

    pub(super) fn item_body_inner(
        &mut self,
        item: &Item,
        skeleton: &ItemSkeleton,
    ) -> Result<(), Aborted> {
        match (&item.kind, &skeleton.kind) {
            (ItemKind::Fn(decl), super::SkeletonKind::Fn(fn_skel)) => {
                // child 2: the body block
                self.push_child(2);
                let body = self.lower_function_body(&decl.body);
                self.pop_child();
                let body = body?;
                self.fill_def(
                    fn_skel.def,
                    DefKind::Fn {
                        generics: fn_skel.generics.clone(),
                        sig: fn_skel.sig,
                        body: Some(body),
                    },
                );
                Ok(())
            }
            (ItemKind::Impl(decl), super::SkeletonKind::Impl { methods, .. }) => {
                for (index, (impl_item, method_skel)) in decl.items.iter().zip(methods).enumerate()
                {
                    let ImplItem::Fn(method) = impl_item;
                    self.push_child(3 + index as u32);
                    // child 2: the method's body block
                    self.push_child(2);
                    let body = self.lower_function_body(&method.body);
                    self.pop_child();
                    self.pop_child();
                    let body = body?;
                    self.fill_def(
                        method_skel.def,
                        DefKind::Method {
                            generics: method_skel.generics.clone(),
                            sig: method_skel.sig,
                            body: Some(body),
                        },
                    );
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Lowers a function or method body: the loop stack is scoped to the
    /// body (no `break`/`continue` crosses a function boundary).
    pub(super) fn lower_function_body(&mut self, block: &xl_ast::Block) -> Result<BodyId, Aborted> {
        let saved_loops = std::mem::take(&mut self.loop_stack);
        let result = self.lower_block(block);
        self.loop_stack = saved_loops;
        result
    }

    /// Runs a nested (block-level) item through all three passes at its
    /// source position. Nested items reset the syntactic `Self` context
    /// and the enclosing loop targets.
    pub(super) fn lower_nested_item(&mut self, item: &Item) -> Result<(), Aborted> {
        let saved_self = self.self_ctx;
        let saved_loops = std::mem::take(&mut self.loop_stack);
        self.self_ctx = 0;
        let result = (|| -> Result<(), Aborted> {
            let skeleton = self.item_skeleton(item)?;
            self.item_signature(item, &skeleton)?;
            self.item_body(item, &skeleton)
        })();
        self.self_ctx = saved_self;
        self.loop_stack = saved_loops;
        result
    }
}
