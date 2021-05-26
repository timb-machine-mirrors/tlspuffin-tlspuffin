use std::{
    any::{type_name, Any, TypeId},
    collections::hash_map::DefaultHasher,
    fmt,
    fmt::Formatter,
    hash::{Hash, Hasher},
};

use itertools::Itertools;
use serde::{
    de,
    de::{value::I32Deserializer, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

/// Describes the shape of a [`DynamicFunction`]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DynamicFunctionShape {
    pub name: String,
    pub argument_types: Vec<TypeShape>,
    pub return_type: TypeShape,
}

impl DynamicFunctionShape {
    pub fn arity(&self) -> u16 {
        self.argument_types.len() as u16
    }
}

impl fmt::Display for DynamicFunctionShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}({}) -> {}",
            self.name,
            self.argument_types
                .iter()
                .map(|typ| format!("{}", typ.name))
                .join(","),
            self.return_type.name
        )
    }
}

/// Hashes [`TypeId`]s to be more readable
///
pub fn hash_type_id(type_id: &TypeId) -> u64 {
    let mut hasher = DefaultHasher::new();
    type_id.hash(&mut hasher);
    hasher.finish()
}

pub fn format_args<P: 'static + AsRef<dyn Any + Send + Sync>>(anys: &Vec<P>) -> String {
    format!(
        "({})",
        anys.iter()
            .map(|any| {
                let id = any.type_id();
                format!("{:x}", hash_type_id(&id))
            })
            .join(",")
    )
}

// The type of dynamically typed functions is:

/// Cloneable type for dynamic functions. This trait is automatically implemented for arbitrary
/// closures and functions of the form: `Fn(&Vec<Box<dyn Any>>) -> Box<dyn Any>`
///
/// [`Clone`] is implemented for `Box<dyn DynamicFunction>` using this trick:
/// https://users.rust-lang.org/t/how-to-clone-a-boxed-closure/31035/25
///
/// We want to use Any here and not VariableData (which implements Clone). Else all returned types
/// in functions op_impl.rs would need to return a cloneable struct. Message for example is not.
pub trait DynamicFunction: Fn(&Vec<Box<dyn Any + Send + Sync>>) -> Box<dyn Any + Send + Sync> + Send + Sync {
    fn clone_box(&self) -> Box<dyn DynamicFunction>;
}

impl<T> DynamicFunction for T
where
    T: 'static + Fn(&Vec<Box<dyn Any + Send + Sync>>) -> Box<dyn Any + Send + Sync> + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn DynamicFunction> {
        Box::new(self.clone())
    }
}

impl fmt::Debug for Box<dyn DynamicFunction> {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        // todo
        Ok(())
    }
}

impl Clone for Box<dyn DynamicFunction> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

/// This trait is implemented for function traits in order to:
/// * describe their shape during runtime
/// * wrap them into a [`DynamicFunction`] which is callable with arbitrary data
///
/// Adapted from https://jsdw.me/posts/rust-fn-traits/ but using type ids
pub trait DescribableFunction<Types> {
    fn shape() -> DynamicFunctionShape;
    fn make_dynamic(&'static self) -> Box<dyn DynamicFunction>;
}

macro_rules! dynamic_fn {
    ($($arg:ident)* => $res:ident) => (
    impl<F, $res: 'static, $($arg: 'static),*> // 'static missing
        DescribableFunction<($res, $($arg),*)> for F
    where
        F: Fn($(&$arg),*)  -> $res + Send + Sync,
        $res: Send + Sync,
        $($arg: Send + Sync),*
    {
        fn shape() -> DynamicFunctionShape {
            DynamicFunctionShape {
                name: std::any::type_name::<F>().to_string(),
                argument_types: vec![$(TypeShape::of::<$arg>()),*],
                return_type: TypeShape::of::<$res>(),
            }
        }

        fn make_dynamic(&'static self) -> Box<dyn DynamicFunction> {
            #[allow(unused_variables)]
            Box::new(move |args: &Vec<Box<dyn Any + Send + Sync>>| {
                #[allow(unused_mut)]
                let mut index = 0;

                Box::new(self($(
                       #[allow(unused_assignments)]
                       {
                           if let Some(arg_) = args.get(index)
                                    .unwrap_or_else(|| {
                                        let shape = Self::shape();
                                        panic!("Missing argument #{} while calling {}.", index + 1, shape.name)
                                    })
                                    .as_ref().downcast_ref::<$arg>() {
                               index = index + 1;
                               arg_
                           } else {
                               let shape = Self::shape();
                               panic!(
                                    "Passed argument #{} of {} did not match the shape {}. Hashes of passed types are {}.",
                                    index + 1,
                                    shape.name,
                                    shape,
                                    format_args(args)
                               )
                           }
                       }
                ),*))
            })
        }
    }
    )
}

dynamic_fn!( => R);
dynamic_fn!(T1 => R);
dynamic_fn!(T1 T2 => R);
dynamic_fn!(T1 T2 T3 => R);
dynamic_fn!(T1 T2 T3 T4 => R);
dynamic_fn!(T1 T2 T3 T4 T5 => R);
dynamic_fn!(T1 T2 T3 T4 T5 T6 => R);

pub fn make_dynamic<F: 'static, Types>(
    f: &'static F,
) -> (DynamicFunctionShape, Box<dyn DynamicFunction>)
where
    F: DescribableFunction<Types>,
{
    (F::shape(), f.make_dynamic())
}

#[derive(Copy, Clone, Debug)]
pub struct TypeShape {
    inner_type_id: TypeId,
    pub name: &'static str
}

struct UnknownType;

impl TypeShape {
    pub fn of<T: 'static>() -> TypeShape {
        Self {
            inner_type_id: TypeId::of::<T>(),
            name: type_name::<T>()
        }
    }

    fn default_type_id() -> TypeId {
        TypeId::of::<UnknownType>()
    }
}

impl Into<TypeId> for TypeShape {
    fn into(self) -> TypeId {
        self.inner_type_id
    }
}

impl PartialEq for TypeShape {
    fn eq(&self, other: &Self) -> bool {
        self.inner_type_id == other.inner_type_id
    }
}

// todo serialization

impl Serialize for Box<dyn DynamicFunction> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {

        // todo
        serializer.serialize_str(type_name::<dyn DynamicFunction>())
    }
}

struct StringVisitor;

impl<'de> Visitor<'de> for StringVisitor {
    type Value = &'de str;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string xxx")
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        return Ok(v);
    }
}

impl<'de> Deserialize<'de> for Box<dyn DynamicFunction> {
    fn deserialize<D>(deserializer: D) -> Result<Box<dyn DynamicFunction>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // todo
        deserializer
            .deserialize_str(StringVisitor)
            .map(|_str| make_dynamic(&crate::term::op_impl::op_server_hello).1)
    }
}

impl Serialize for TypeShape {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // todo
        serializer.serialize_u64(1)
    }
}

struct VisitorU64;

impl<'de> Visitor<'de> for VisitorU64 {
    type Value = u64;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a 'de")
    }

    fn visit_i64<E>(self, value: i64) -> Result<u64, E>
    where
        E: serde::de::Error,
    {
        Ok(value as u64)
    }

    fn visit_u64<E>(self, value: u64) -> Result<u64, E>
    where
        E: serde::de::Error,
    {
        self.visit_i64(value as i64)
    }
}

impl<'de> Deserialize<'de> for TypeShape {
    fn deserialize<D>(deserializer: D) -> Result<TypeShape, D::Error>
    where
        D: Deserializer<'de>,
    {
        // todo
        deserializer
            .deserialize_u64(VisitorU64)
            .map(|_i| TypeShape::of::<UnknownType>())
    }
}
