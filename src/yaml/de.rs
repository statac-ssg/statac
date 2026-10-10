//! The serde `Deserializer` over the tree.
//!
//! A scalar means what YAML's standard reading makes of it, whatever the
//! target type, with one bend: a text target takes a plain scalar read as
//! another type where that reading keeps its characters (`title: 1984`).
//! A true-or-false target takes `true` and `false` spelt only so.
//!
//! A `Sourced<T>` field receives its node's origin with its value. The
//! deserializer recognises it by a private struct name and hands it the
//! place and style as extra fields, as the toml crate does for its spans.

use std::cell::RefCell;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};

use super::schema::{Resolved, keeps_characters};
use super::tree::{Entry, Node, Value};
use crate::origin::{Origin, Sourced, Style};
use crate::source::{Place, SourceId, Span};

/// A value that doesn't fit the type it is read into, at the innermost node
/// the reading reached.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Error {
    pub message: String,
    pub place: Option<Place>,
}

impl Error {
    fn at(message: impl Into<String>, place: Place) -> Self {
        Error {
            message: message.into(),
            place: Some(place),
        }
    }

    /// Places an error raised without a place, such as serde's missing field.
    fn placed(mut self, place: Place) -> Self {
        self.place.get_or_insert(place);
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl de::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Error {
            message: message.to_string(),
            place: None,
        }
    }
}

/// A mapping key no field of the target took, and its value.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Unknown<'de> {
    pub key: &'de Node,
    pub value: &'de Node,
}

/// Reads `node` into `T`. Keys no field takes are passed over.
pub fn from_node<'de, T: Deserialize<'de>>(node: &'de Node) -> Result<T, Error> {
    T::deserialize(Deserializer {
        node,
        key: None,
        unknown: None,
    })
    .map_err(|e| e.placed(node.place))
}

/// Reads `node` into `T`, and gives every key no field took, at any depth,
/// in the order written.
pub fn from_node_noting_unknown<'de, T: Deserialize<'de>>(
    node: &'de Node,
) -> Result<(T, Vec<Unknown<'de>>), Error> {
    let unknown = RefCell::new(Vec::new());
    let value = T::deserialize(Deserializer {
        node,
        key: None,
        unknown: Some(&unknown),
    })
    .map_err(|e| e.placed(node.place))?;
    Ok((value, unknown.into_inner()))
}

#[derive(Copy, Clone)]
struct Deserializer<'a, 'de> {
    node: &'de Node,
    /// The key this node is the value of.
    key: Option<&'de Node>,
    unknown: Option<&'a RefCell<Vec<Unknown<'de>>>>,
}

impl<'a, 'de> Deserializer<'a, 'de> {
    fn child(&self, node: &'de Node, key: Option<&'de Node>) -> Self {
        Deserializer {
            node,
            key,
            unknown: self.unknown,
        }
    }

    fn error(&self, message: impl Into<String>) -> Error {
        Error::at(message, self.node.place)
    }

    fn scalar(&self) -> Result<(&'de str, Resolved), Error> {
        let Value::Scalar(text) = &self.node.value else {
            return Err(self.error(format!("expected a single value, found {}", self.found())));
        };
        match self.node.resolved() {
            Some(resolved) => Ok((text, resolved)),
            None => Err(self.error(format!("`{text}` doesn't fit its tag"))),
        }
    }

    fn found(&self) -> &'static str {
        match &self.node.value {
            Value::Sequence(_) => "a list",
            Value::Mapping(_) => "a mapping",
            Value::Scalar(_) => match self.node.resolved() {
                Some(Resolved::Null) => "no value",
                Some(Resolved::Bool(_)) => "true or false",
                Some(Resolved::Int(_)) => "a whole number",
                Some(Resolved::Float(_)) => "a number",
                Some(Resolved::Str) => "text",
                None => "a value that doesn't fit its tag",
            },
        }
    }

    fn text(&self) -> Result<&'de str, Error> {
        match self.scalar()? {
            (text, Resolved::Str) => Ok(text),
            (text, resolved)
                if self.node.style == Style::Plain && keeps_characters(text, resolved) =>
            {
                Ok(text)
            }
            (_, Resolved::Null) => Err(self.error("expected text, found no value")),
            (text, _) => Err(self.error(format!("`{text}` is {} to YAML, not text", self.found()))),
        }
    }

    fn integer(&self) -> Result<i128, Error> {
        match self.scalar()? {
            (_, Resolved::Int(Some(n))) => Ok(n),
            (text, Resolved::Int(None)) => Err(self.error(format!("`{text}` is too large"))),
            _ => Err(self.error(format!("expected a whole number, found {}", self.found()))),
        }
    }

    fn entries(&self, entries: &'de [Entry]) -> Entries<'a, 'de> {
        Entries {
            de: *self,
            entries: entries.iter(),
            value: None,
        }
    }

    fn items(&self, items: &'de [Node]) -> Items<'a, 'de> {
        Items {
            de: *self,
            items: items.iter(),
        }
    }
}

macro_rules! integer {
    ($($method:ident => $visit:ident as $ty:ty),* $(,)?) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            let n = self.integer()?;
            match <$ty>::try_from(n) {
                Ok(n) => visitor.$visit(n),
                Err(_) => Err(self.error(format!("{n} is out of range here"))),
            }
        }
    )*};
}

impl<'de> de::Deserializer<'de> for Deserializer<'_, 'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match &self.node.value {
            Value::Sequence(items) => visitor.visit_seq(self.items(items)),
            Value::Mapping(entries) => visitor.visit_map(self.entries(entries)),
            Value::Scalar(_) => match self.scalar()? {
                (_, Resolved::Null) => visitor.visit_unit(),
                (_, Resolved::Bool(b)) => visitor.visit_bool(b),
                (_, Resolved::Int(Some(n))) => match i64::try_from(n) {
                    Ok(n) => visitor.visit_i64(n),
                    Err(_) => visitor.visit_i128(n),
                },
                (text, Resolved::Int(None)) => Err(self.error(format!("`{text}` is too large"))),
                (_, Resolved::Float(f)) => visitor.visit_f64(f),
                (text, Resolved::Str) => visitor.visit_borrowed_str(text),
            },
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.scalar()? {
            // `True` and `TRUE` are YAML's own spellings, not a second spelling of Statac's.
            (_, Resolved::Bool(b)) => visitor.visit_bool(b),
            _ => Err(self.error(format!("expected true or false, found {}", self.found()))),
        }
    }

    integer! {
        deserialize_i8 => visit_i8 as i8,
        deserialize_i16 => visit_i16 as i16,
        deserialize_i32 => visit_i32 as i32,
        deserialize_i64 => visit_i64 as i64,
        deserialize_i128 => visit_i128 as i128,
        deserialize_u8 => visit_u8 as u8,
        deserialize_u16 => visit_u16 as u16,
        deserialize_u32 => visit_u32 as u32,
        deserialize_u64 => visit_u64 as u64,
        deserialize_u128 => visit_u128 as u128,
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.scalar()? {
            (_, Resolved::Float(f)) => visitor.visit_f64(f),
            (_, Resolved::Int(Some(n))) => visitor.visit_f64(n as f64),
            _ => Err(self.error(format!("expected a number, found {}", self.found()))),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_borrowed_str(self.text()?)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.node.resolved() {
            Some(Resolved::Null) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.scalar()? {
            (_, Resolved::Null) => visitor.visit_unit(),
            _ => Err(self.error(format!("expected no value, found {}", self.found()))),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match &self.node.value {
            Value::Sequence(items) => visitor.visit_seq(self.items(items)),
            _ => Err(self.error(format!("expected a list, found {}", self.found()))),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match &self.node.value {
            Value::Mapping(entries) => visitor.visit_map(self.entries(entries)),
            _ => Err(self.error(format!("expected a mapping, found {}", self.found()))),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        if name == SOURCED {
            return visitor.visit_map(SourcedFields { de: self, field: 0 });
        }
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_enum(UnitVariant(self.text()?))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match &self.node.value {
            Value::Scalar(text) => visitor.visit_borrowed_str(text),
            _ => Err(self.error(format!("expected a name, found {}", self.found()))),
        }
    }

    /// Serde's derived structs read the value of a key no field takes here.
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if let (Some(unknown), Some(key)) = (self.unknown, self.key) {
            unknown.borrow_mut().push(Unknown {
                key,
                value: self.node,
            });
        }
        visitor.visit_unit()
    }
}

struct Items<'a, 'de> {
    de: Deserializer<'a, 'de>,
    items: std::slice::Iter<'de, Node>,
}

impl<'de> SeqAccess<'de> for Items<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        let Some(node) = self.items.next() else {
            return Ok(None);
        };
        seed.deserialize(self.de.child(node, None))
            .map(Some)
            .map_err(|e| e.placed(node.place))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.items.len())
    }
}

struct Entries<'a, 'de> {
    de: Deserializer<'a, 'de>,
    entries: std::slice::Iter<'de, Entry>,
    value: Option<&'de Entry>,
}

impl<'de> MapAccess<'de> for Entries<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        let Some(entry) = self.entries.next() else {
            return Ok(None);
        };
        self.value = Some(entry);
        seed.deserialize(self.de.child(&entry.key, None))
            .map(Some)
            .map_err(|e| e.placed(entry.key.place))
    }

    fn next_value_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value, Error> {
        let entry = self.value.take().expect("a value follows its key");
        seed.deserialize(self.de.child(&entry.value, Some(&entry.key)))
            .map_err(|e| e.placed(entry.value.place))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}

const SOURCED: &str = "$statac::Sourced";
/// What the deserializer hands `Sourced`, in this order.
const FIELDS: &[&str] = &["$source", "$start", "$end", "$style", "$value"];
/// Styles by their number on the way through serde.
const STYLES: [Style; 6] = [
    Style::Plain,
    Style::SingleQuoted,
    Style::DoubleQuoted,
    Style::Literal,
    Style::Folded,
    Style::None,
];

struct SourcedFields<'a, 'de> {
    de: Deserializer<'a, 'de>,
    field: usize,
}

impl<'de> MapAccess<'de> for SourcedFields<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match FIELDS.get(self.field) {
            Some(field) => seed.deserialize(Name(field)).map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value, Error> {
        let node = self.de.node;
        self.field += 1;
        match self.field {
            1 => seed.deserialize(node.place.source.index().into_deserializer()),
            2 => seed.deserialize(node.place.span.start.into_deserializer()),
            3 => seed.deserialize(node.place.span.end.into_deserializer()),
            4 => {
                let style = STYLES.iter().position(|&s| s == node.style).unwrap_or(0);
                seed.deserialize((style as u8).into_deserializer())
            }
            _ => seed.deserialize(self.de),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Sourced<T> {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SourcedVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for SourcedVisitor<T> {
            type Value = Sourced<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a value read from YAML with its place")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                fn next<'de, A: MapAccess<'de>, V: Deserialize<'de>>(
                    map: &mut A,
                    field: &str,
                ) -> Result<V, A::Error> {
                    match map.next_key::<&str>()? {
                        Some(key) if key == field => map.next_value(),
                        _ => Err(de::Error::custom(
                            "a value with its place can only be read from YAML",
                        )),
                    }
                }
                let source = SourceId::from_index(next(&mut map, FIELDS[0])?);
                let start: u32 = next(&mut map, FIELDS[1])?;
                let end: u32 = next(&mut map, FIELDS[2])?;
                let style: u8 = next(&mut map, FIELDS[3])?;
                let value = next(&mut map, FIELDS[4])?;
                let place = Place {
                    source,
                    span: Span::new(start as usize, end as usize),
                };
                let style = STYLES[usize::from(style)];
                Ok(Sourced::new(value, Origin::written(place, style)))
            }
        }

        deserializer.deserialize_struct(SOURCED, FIELDS, SourcedVisitor(PhantomData))
    }
}

/// One of the names `Sourced` is handed its fields by.
struct Name(&'static str);

impl<'de> de::Deserializer<'de> for Name {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_borrowed_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

/// An enum read from a scalar naming one of its unit variants.
struct UnitVariant<'de>(&'de str);

impl<'de> EnumAccess<'de> for UnitVariant<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self), Error> {
        let variant = seed.deserialize(self.0.into_deserializer())?;
        Ok((variant, self))
    }
}

impl<'de> VariantAccess<'de> for UnitVariant<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _seed: T) -> Result<T::Value, Error> {
        Err(de::Error::custom(format!("`{}` takes a value", self.0)))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value, Error> {
        Err(de::Error::custom(format!("`{}` takes values", self.0)))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Error> {
        Err(de::Error::custom(format!("`{}` takes fields", self.0)))
    }
}
