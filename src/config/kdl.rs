use std::fmt::Write;

use documented::DocumentedFields;
use knus::{
    DecodeScalar,
    ast::{Literal, TypeName},
    decode::Context,
    errors::DecodeError,
    span::Spanned,
    traits::ErrorSpan,
};
use pango::FontDescription;
use struct_patch::Patch;

use crate::{
    config::{
        BarTheme, ButtonTheme, Config, Insets, NotificationTheme, SpreadConfig, StackConfig,
        SurfaceTheme, TextTheme, ThemeConfig,
    },
    ui::renderables::Color,
};

/// The patch of a float, which can also be written as an integer.
pub struct Float(f64);

impl<S: ErrorSpan> DecodeScalar<S> for Float {
    fn type_check(type_name: &Option<Spanned<TypeName, S>>, ctx: &mut Context<S>) {
        f64::type_check(type_name, ctx);
    }

    fn raw_decode(
        value: &Spanned<Literal, S>,
        ctx: &mut Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        match &**value {
            Literal::Int(int) => i64::try_from(int)
                .map(|int| Self(int as f64))
                .map_err(|err| DecodeError::conversion(value, err)),
            _ => f64::raw_decode(value, ctx).map(Self),
        }
    }
}

/// The patch of a font, written as a pango font description string.
pub struct Font(FontDescription);

impl<S: ErrorSpan> DecodeScalar<S> for Font {
    fn type_check(type_name: &Option<Spanned<TypeName, S>>, ctx: &mut Context<S>) {
        String::type_check(type_name, ctx);
    }

    fn raw_decode(
        value: &Spanned<Literal, S>,
        ctx: &mut Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        String::raw_decode(value, ctx).map(|font| Self(FontDescription::from_string(&font)))
    }
}

// Implements the patch of a value that is always replaced as a whole.
macro_rules! replaced_by {
    ($($ty:ty => $patch:ident;)*) => {$(
        impl Patch<$patch> for $ty {
            fn apply(&mut self, patch: $patch) {
                *self = patch.0;
            }

            fn into_patch(self) -> $patch {
                $patch(self)
            }

            fn into_patch_by_diff(self, _: Self) -> $patch {
                $patch(self)
            }

            fn new_empty_patch() -> $patch {
                $patch(Default::default())
            }
        }
    )*};
}

replaced_by! {
    f64 => Float;
    FontDescription => Font;
}

impl<S: ErrorSpan> DecodeScalar<S> for Color {
    fn type_check(type_name: &Option<Spanned<TypeName, S>>, ctx: &mut Context<S>) {
        String::type_check(type_name, ctx);
    }

    fn raw_decode(
        value: &Spanned<Literal, S>,
        ctx: &mut Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        let css = String::raw_decode(value, ctx)?;
        Color::parse(&css)
            .map_err(|err| DecodeError::unexpected(value, "value", format!("invalid color, {err}")))
    }
}

/// Writes a value as the rest of a kdl node, after its name.
pub trait Node {
    fn write(&self, out: &mut String, depth: usize);
}

impl Node for f64 {
    fn write(&self, out: &mut String, _: usize) {
        // Debug always keeps the decimal point.
        let _ = write!(out, " {self:?}");
    }
}

impl Node for usize {
    fn write(&self, out: &mut String, _: usize) {
        let _ = write!(out, " {self}");
    }
}

impl Node for Color {
    fn write(&self, out: &mut String, _: usize) {
        let _ = write!(out, " {:?}", self.to_hex());
    }
}

impl Node for FontDescription {
    fn write(&self, out: &mut String, _: usize) {
        let _ = write!(out, " {:?}", self.to_str().as_str());
    }
}

/// Writes a config struct as kdl nodes, with the docs of each field as
/// comments above its node.
pub trait Fields: DocumentedFields {
    fn fields(&self) -> Vec<(&'static str, &dyn Node)>;

    fn write_fields(&self, out: &mut String, depth: usize) {
        let indent = "    ".repeat(depth);
        for (index, (name, value)) in self.fields().into_iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }

            for line in Self::get_field_docs(name).unwrap_or_default().lines() {
                let _ = writeln!(out, "{}", format!("{indent}// {line}").trim_end());
            }

            // The decoder expects the field names in kebab case.
            let _ = write!(out, "{indent}{}", name.replace('_', "-"));
            value.write(out, depth);
            out.push('\n');
        }
    }
}

impl<T: Fields> Node for T {
    fn write(&self, out: &mut String, depth: usize) {
        out.push_str(" {\n");
        self.write_fields(out, depth + 1);
        let _ = write!(out, "{}}}", "    ".repeat(depth));
    }
}

macro_rules! fields {
    ($($ty:ty { $($field:ident),* };)*) => {$(
        impl Fields for $ty {
            fn fields(&self) -> Vec<(&'static str, &dyn Node)> {
                vec![$((stringify!($field), &self.$field)),*]
            }
        }
    )*};
}

fields! {
    Config { width, margin, stack, spread, theme };
    Insets { x, y };
    StackConfig { peek, inset, max_count };
    SpreadConfig { gap, max_count };
    ThemeConfig { notification, button, bar };
    NotificationTheme { surface, title, body, meta, image_radius, image_gap, title_gap, meta_gap };
    ButtonTheme { surface, label, gap };
    BarTheme { surface, label };
    SurfaceTheme { background, border_width, border_color, border_radius, padding };
    TextTheme { font, color };
}
