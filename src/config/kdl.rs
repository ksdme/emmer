use std::fmt::Write;

use knus::{
    Decode, DecodeScalar,
    ast::{Literal, SpannedNode, TypeName},
    decode::Context,
    errors::DecodeError,
    span::Spanned,
    traits::ErrorSpan,
};
use pango::FontDescription;

use crate::ui::renderables::Color;

/// A config value that can be patched from, and written as, a kdl node.
pub trait Setting: PartialEq {
    /// The parts of the value that were given in a kdl document.
    type Patch;

    fn apply(&mut self, patch: Self::Patch);

    /// Writes the value as a kdl node, where the values that are the same as
    /// their default are commented out.
    fn write_node(&self, name: &str, default: &Self, out: &mut String, depth: usize);
}

/// A config value that is written as the only argument of a kdl node.
pub trait Scalar: PartialEq + Sized {
    /// The decoded kdl argument.
    type Arg: Into<Self>;

    fn to_arg(&self) -> String;
}

impl<T: Scalar> Setting for T {
    type Patch = Arg<T::Arg>;

    fn apply(&mut self, Arg(arg): Self::Patch) {
        *self = arg.into();
    }

    fn write_node(&self, name: &str, default: &Self, out: &mut String, depth: usize) {
        write_arg(name, Some(self.to_arg()), self == default, out, depth);
    }
}

/// An optional value, which is unset until it is given.
impl<T: Scalar> Setting for Option<T> {
    type Patch = Arg<T::Arg>;

    fn apply(&mut self, Arg(arg): Self::Patch) {
        *self = Some(arg.into());
    }

    fn write_node(&self, name: &str, default: &Self, out: &mut String, depth: usize) {
        let arg = self.as_ref().map(T::to_arg);
        write_arg(name, arg, self.is_none() || self == default, out, depth);
    }
}

fn write_arg(name: &str, arg: Option<String>, commented: bool, out: &mut String, depth: usize) {
    let comment = if commented { "// " } else { "" };
    let arg = arg.map(|arg| format!(" {arg}")).unwrap_or_default();
    let _ = write!(out, "{}{comment}{name}{arg}", indent(depth));
}

impl Scalar for f64 {
    type Arg = Float;

    fn to_arg(&self) -> String {
        // Debug always keeps the decimal point.
        format!("{self:?}")
    }
}

impl Scalar for usize {
    type Arg = usize;

    fn to_arg(&self) -> String {
        self.to_string()
    }
}

impl Scalar for Color {
    type Arg = Color;

    fn to_arg(&self) -> String {
        format!("{:?}", self.to_hex())
    }
}

impl Scalar for FontDescription {
    type Arg = Font;

    fn to_arg(&self) -> String {
        format!("{:?}", self.to_str().as_str())
    }
}

pub fn indent(depth: usize) -> String {
    "    ".repeat(depth)
}

/// Writes the docs of a field as comments, followed by its node.
pub fn write_field<T: Setting>(
    docs: &[&str],
    name: &str,
    value: &T,
    default: &T,
    out: &mut String,
    depth: usize,
) {
    for line in docs {
        let line = line.strip_prefix(' ').unwrap_or(line);
        let _ = writeln!(out, "{}", format!("{}// {line}", indent(depth)).trim_end());
    }

    // The decoder expects the field names in kebab case.
    value.write_node(&name.replace('_', "-"), default, out, depth);
    out.push('\n');
}

/// Declares config structs whose fields are kdl child nodes, along with their
/// patches and writers. The docs of the fields are written as comments.
macro_rules! settings {
    ($(
        $(#[doc = $doc:literal])*
        pub struct $name:ident {
            $(
                $(#[doc = $field_doc:literal])*
                pub $field:ident: $ty:ty,
            )*
        }
    )*) => {$(
        $(#[doc = $doc])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            $(
                $(#[doc = $field_doc])*
                pub $field: $ty,
            )*
        }

        pastey::paste! {
            #[derive(knus::Decode)]
            pub struct [<$name Patch>] {
                $(
                    #[knus(child)]
                    pub $field: Option<<$ty as $crate::config::kdl::Setting>::Patch>,
                )*
            }

            impl $name {
                /// Writes the fields as kdl nodes, separated by blank lines.
                pub fn write_fields(&self, default: &Self, out: &mut String, depth: usize) {
                    let fields = [$({
                        let mut field = String::new();
                        $crate::config::kdl::write_field(
                            &[$($field_doc),*],
                            stringify!($field),
                            &self.$field,
                            &default.$field,
                            &mut field,
                            depth,
                        );
                        field
                    }),*];
                    out.push_str(&fields.join("\n"));
                }
            }

            impl $crate::config::kdl::Setting for $name {
                type Patch = [<$name Patch>];

                fn apply(&mut self, patch: Self::Patch) {
                    $(
                        if let Some(patch) = patch.$field {
                            self.$field.apply(patch);
                        }
                    )*
                }

                fn write_node(&self, name: &str, default: &Self, out: &mut String, depth: usize) {
                    use std::fmt::Write;

                    let indent = $crate::config::kdl::indent(depth);
                    let _ = writeln!(out, "{indent}{name} {{");
                    self.write_fields(default, out, depth + 1);
                    let _ = write!(out, "{indent}}}");
                }
            }
        }
    )*};
}

pub(super) use settings;

/// A kdl node with a single argument and nothing else.
pub struct Arg<T>(T);

impl<S: ErrorSpan, T: DecodeScalar<S>> Decode<S> for Arg<T> {
    fn decode_node(node: &SpannedNode<S>, ctx: &mut Context<S>) -> Result<Self, DecodeError<S>> {
        if let Some(type_name) = &node.type_name {
            ctx.emit_error(DecodeError::unexpected(
                type_name,
                "type name",
                "no type name expected for this node",
            ));
        }

        let mut args = node.arguments.iter();
        let arg = args
            .next()
            .ok_or_else(|| DecodeError::missing(node, "additional argument is required"))?;
        let value = T::decode(arg, ctx)?;
        if let Some(arg) = args.next() {
            return Err(DecodeError::unexpected(
                &arg.literal,
                "argument",
                "unexpected argument",
            ));
        }

        if let Some((name, _)) = node.properties.iter().next() {
            return Err(DecodeError::unexpected(
                name,
                "property",
                format!("unexpected property `{}`", name.escape_default()),
            ));
        }

        for child in node.children.iter().flat_map(|children| children.iter()) {
            ctx.emit_error(DecodeError::unexpected(
                child,
                "node",
                format!("unexpected node `{}`", child.node_name.escape_default()),
            ));
        }

        Ok(Self(value))
    }
}

/// A float, which can also be written as an integer.
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

impl From<Float> for f64 {
    fn from(Float(value): Float) -> Self {
        value
    }
}

/// A font, written as a pango font description string.
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

impl From<Font> for FontDescription {
    fn from(Font(value): Font) -> Self {
        value
    }
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
