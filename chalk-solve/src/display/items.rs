use std::fmt::{Formatter, Result};

use crate::rust_ir::*;
use crate::split::Split;
use chalk_ir::interner::Interner;
use itertools::Itertools;

use super::{
    display_self_where_clauses_as_bounds, display_trait_with_generics, render_trait::RenderAsRust,
    state::WriterState,
};

impl<I: Interner> RenderAsRust<I> for AdtDatum<I> {
    fn fmt(&self, s: &WriterState<'_, I>, f: &'_ mut Formatter<'_>) -> Result {
        // When support for Self in structs is added, self_binding should be
        // changed to Some(0)
        let s = &s.add_debrujin_index(None);
        let value = self.binders.skip_binders();
        write!(f, "struct {}", self.id.display(s),)?;
        write_joined_non_empty_list!(f, "<{}>", s.binder_var_display(&self.binders.binders), ", ")?;
        if !value.where_clauses.is_empty() {
            let s = &s.add_indent();
            write!(f, "\nwhere\n{}\n", value.where_clauses.display(s))?;
        } else {
            write!(f, " ")?;
        }
        write!(f, "{{")?;
        let s = &s.add_indent();
        write_joined_non_empty_list!(
            f,
            "\n{}\n",
            value.fields.iter().enumerate().map(|(idx, field)| {
                format!("{}field_{}: {}", s.indent(), idx, field.display(s))
            }),
            ",\n"
        )?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl<I: Interner> RenderAsRust<I> for Polarity {
    fn fmt(&self, _s: &WriterState<'_, I>, f: &'_ mut Formatter<'_>) -> Result {
        if !self.is_positive() {
            write!(f, "!")?;
        }
        Ok(())
    }
}

impl<I: Interner> RenderAsRust<I> for TraitDatum<I> {
    fn fmt(&self, s: &WriterState<'_, I>, f: &'_ mut Formatter<'_>) -> Result {
        let s = &s.add_debrujin_index(Some(0));
        let value = self.binders.skip_binders();

        macro_rules! trait_flags {
            ($($n:ident),*) => {
                $(if self.flags.$n {
                    write!(f,"#[{}]\n",stringify!($n))?;
                })*
            }
        }

        trait_flags!(
            auto,
            marker,
            upstream,
            fundamental,
            non_enumerable,
            coinductive
        );
        let binders = s.binder_var_display(&self.binders.binders).skip(1);
        write!(f, "trait {}", self.id.display(s))?;
        write_joined_non_empty_list!(f, "<{}>", binders, ", ")?;
        if !value.where_clauses.is_empty() {
            let s = &s.add_indent();
            write!(f, "\nwhere\n{}\n", value.where_clauses.display(s))?;
        } else {
            write!(f, " ")?;
        }
        write!(f, "{{")?;
        let s = &s.add_indent();
        write_joined_non_empty_list!(
            f,
            "\n{}\n",
            self.associated_ty_ids.iter().map(|assoc_ty_id| {
                let assoc_ty_data = s.db.associated_ty_data(*assoc_ty_id);
                format!("{}{}", s.indent(), (*assoc_ty_data).display(s))
            }),
            "\n"
        )?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl<I: Interner> RenderAsRust<I> for ImplDatum<I> {
    fn fmt(&self, s: &WriterState<'_, I>, f: &'_ mut Formatter<'_>) -> Result {
        let interner = s.db.interner();

        let s = &s.add_debrujin_index(None);
        let binders = s.binder_var_display(&self.binders.binders);

        let value = self.binders.skip_binders();

        let trait_ref = &value.trait_ref;
        // Ignore automatically added Self parameter by skipping first parameter
        let full_trait_name = display_trait_with_generics(
            s,
            trait_ref.trait_id,
            &trait_ref.substitution.parameters(interner)[1..],
        );
        write!(f, "impl")?;
        write_joined_non_empty_list!(f, "<{}>", binders, ", ")?;
        write!(
            f,
            " {}{} for {}",
            self.polarity.display(s),
            full_trait_name,
            trait_ref.self_type_parameter(interner).display(s)
        )?;
        if !value.where_clauses.is_empty() {
            let s = &s.add_indent();
            write!(f, "\nwhere\n{}\n", value.where_clauses.display(s))?;
        } else {
            write!(f, " ")?;
        }
        write!(f, "{{")?;
        {
            let s = &s.add_indent();
            let assoc_ty_values = self.associated_ty_value_ids.iter().map(|assoc_ty_value| {
                s.db.associated_ty_value(*assoc_ty_value)
                    .display(s)
                    .to_string()
            });
            write_joined_non_empty_list!(f, "\n{}\n", assoc_ty_values, "\n")?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl<I: Interner> RenderAsRust<I> for OpaqueTyDatum<I> {
    fn fmt(&self, s: &WriterState<'_, I>, f: &mut Formatter<'_>) -> Result {
        let s = &s.add_debrujin_index(None);
        let bounds = self.bound.skip_binders();
        write!(f, "opaque type {}", self.opaque_ty_id.display(s))?;
        write_joined_non_empty_list!(f, "<{}>", s.binder_var_display(&self.bound.binders), ", ")?;
        {
            let s = &s.add_debrujin_index(Some(0));
            let clauses = bounds.bounds.skip_binders();
            write!(
                f,
                ": {} = ",
                display_self_where_clauses_as_bounds(s, clauses)
            )?;
        }
        write!(f, "{};", bounds.hidden_ty.display(s))?;
        Ok(())
    }
}

impl<I: Interner> RenderAsRust<I> for AssociatedTyDatum<I> {
    fn fmt(&self, s: &WriterState<'_, I>, f: &'_ mut Formatter<'_>) -> Result {
        // In lowering, a completely new empty environment is created for each
        // AssociatedTyDatum, and it's given generic parameters for each generic
        // parameter that its trait had. We want to map the new binders for
        // those generic parameters back into their original names. To do that,
        // first find their original names (trait_binder_names), then the names
        // they have inside the AssociatedTyDatum (assoc_ty_names_for_trait_params),
        // and then add that mapping to the WriterState when writing bounds and
        // where clauses.
        let trait_datum = s.db.trait_datum(self.trait_id);
        // inverted Debrujin indices for the trait's parameters in the trait
        // environment
        let trait_param_names_in_trait_env = s.binder_var_indices(&trait_datum.binders.binders);
        let s = &s.add_debrujin_index(None);
        // inverted Debrujin indices for the trait's parameters in the
        // associated type environment
        let param_names_in_assoc_ty_env = s
            .binder_var_indices(&self.binders.binders)
            .collect::<Vec<_>>();
        // inverted Debrujin indices to render the trait's parameters in the
        // associated type environment
        let (trait_param_names_in_assoc_ty_env, _) =
            s.db.split_associated_ty_parameters(&param_names_in_assoc_ty_env, self);

        let s = &s.add_parameter_mapping(
            trait_param_names_in_assoc_ty_env.iter().copied(),
            trait_param_names_in_trait_env,
        );

        // rendered names for the associated type's generics in the associated
        // type environment
        let binder_display_in_assoc_ty = s
            .binder_var_display(&self.binders.binders)
            .collect::<Vec<_>>();

        let (_, assoc_ty_params) =
            s.db.split_associated_ty_parameters(&binder_display_in_assoc_ty, self);
        write!(f, "type {}", self.id.display(s))?;
        write_joined_non_empty_list!(f, "<{}>", assoc_ty_params, ", ")?;

        let datum_bounds = &self.binders.skip_binders();

        if !datum_bounds.bounds.is_empty() {
            write!(f, ": ")?;
        }

        // bounds is `A: V, B: D, C = E`?
        // type Foo<A: V, B:D, C = E>: X + Y + Z;
        let bounds = datum_bounds
            .bounds
            .iter()
            .map(|bound| bound.display(s).to_string())
            .collect::<Vec<String>>()
            .join(" + ");
        write!(f, "{}", bounds)?;

        // where_clause is 'X: Y, Z: D'
        // type Foo<...>: ... where X: Y, Z: D;

        // note: it's a quantified clause b/c we could have `for<'a> T: Foo<'a>`
        // within 'where'
        if !datum_bounds.where_clauses.is_empty() {
            let where_s = &s.add_indent();
            let where_clauses = datum_bounds.where_clauses.display(where_s);
            write!(f, "\n{}where\n{}", s.indent(), where_clauses)?;
        }
        write!(f, ";")?;
        Ok(())
    }
}

impl<I: Interner> RenderAsRust<I> for AssociatedTyValue<I> {
    fn fmt(&self, s: &WriterState<'_, I>, f: &'_ mut Formatter<'_>) -> Result {
        // see comments for a similar empty env operation in AssociatedTyDatum's
        // impl of RenderAsRust.
        let assoc_ty_data = s.db.associated_ty_data(self.associated_ty_id);
        let impl_datum = s.db.impl_datum(self.impl_id);

        let impl_param_names_in_impl_env = s.binder_var_indices(&impl_datum.binders.binders);

        let s = &s.add_debrujin_index(None);
        let value = self.value.skip_binders();

        let param_names_in_assoc_ty_value_env = s
            .binder_var_indices(&self.value.binders)
            .collect::<Vec<_>>();

        let (impl_params_in_assoc_ty_value_env, _assoc_ty_value_params) =
            s.db.split_associated_ty_value_parameters(&param_names_in_assoc_ty_value_env, self);

        let s = &s.add_parameter_mapping(
            impl_params_in_assoc_ty_value_env.iter().cloned(),
            impl_param_names_in_impl_env,
        );

        // let params = s
        //     .binder_var_display(&self.value.binders)
        //     .collect::<Vec<_>>();
        let display_params = s
            .binder_var_display(&self.value.binders)
            .collect::<Vec<_>>();

        let (_impl_display, assoc_ty_value_display) =
            s.db.split_associated_ty_value_parameters(&display_params, self);

        write!(f, "{}type {}", s.indent(), assoc_ty_data.id.display(s))?;
        write_joined_non_empty_list!(f, "<{}>", &assoc_ty_value_display, ", ")?;
        write!(f, " = {};", value.ty.display(s))?;
        Ok(())
    }
}