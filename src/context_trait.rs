use duplicate::duplicate_item;
use serde_json::{Number, Value};
use std::borrow::{Borrow, Cow};
use std::collections::BTreeMap;
use std::hash::{BuildHasher, Hash};
use std::io::Write;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use std::{collections::HashMap, fmt::Debug};

pub enum ContextType {
    Object, // Hashmaps and Structs
    Array,
    Number,
    String,
    Bool,
    Null
}


/// A Trait for any type, that can be used as Context
pub trait ContextTrait: Debug {
    /// Marks whether this content is truthy. Used when attempting to render a section.
    #[inline]
    fn is_truthy(&self) -> bool {
        true
    }

    /// How much capacity is _likely_ required for all the data in this `Content`
    #[inline]
    fn render_capacity_hint(&self) -> usize {
        0
    }

    // By default don't render anything. e.g. for Hashmaps
    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        Ok(())
    }

    /// Returns the value by a given dotted pointer.
    // TODO get(&self, key: dyn INTO<String>) -> Option<&dyn ContextTrait>
    #[inline]
    fn pointer(&self, key: &str) -> Option<Arc<&dyn ContextTrait>>
where
        Self: Sized,
    {
        if key == "." || key.is_empty() {
            return Some(Arc::new(self));
        }
        None
    }

    /// Returns an iterator over (key,values) if possible, otherwise Option::None
    #[inline]
    fn context_iter(&self) -> Option<Box<dyn Iterator<Item = (String,&dyn ContextTrait)>>> {
        None
    }

    /// Returns the type of the Context similary to Json types
    /// defaults to ContextType::Object
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::Object
    }

    /// Returns the length
    /// len() for arrays and hashmaps,
    /// the amount of fields in structs
    fn len(&self) -> usize;
}

#[duplicate_item(
  number_type;
  [ u8 ];
  [ u16 ];
  [ u32 ];
  [ u64 ];
  [ u128 ];
  [ usize ];
  [ i8 ] ;
  [ i16 ];
  [ i32 ];
  [ i64 ];
  [ i128 ];
  [ isize ];
  [ f32 ];
  [ f64 ];
)]
impl ContextTrait for number_type {
    #[inline]
    fn is_truthy(&self) -> bool {
        // Floats shoudn't be directly compared to 0
        *self != 0 as number_type
    }

    #[inline]
    fn render_capacity_hint(&self) -> usize {
        5
    }

    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        write!(write, "{}", self)
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::Number
    }
    #[inline]
    fn len(&self) -> usize {
        1
    }
}

impl<T: ContextTrait, U: Debug + Clone> ContextTrait for Result<T, U> {
    #[inline]
    fn is_truthy(&self) -> bool {
        self.is_ok()
    }

    #[inline]
    fn render_capacity_hint(&self) -> usize {
        match self {
            Ok(inner) => inner.render_capacity_hint(),
            _ => 0,
        }
    }
    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        match self {
            Ok(inner) => inner.render(write),
            _ => Ok(()),
        }
    }
   fn context_iter(&self) -> Option<Box<dyn Iterator<Item = (String, &dyn ContextTrait)>>> {
        match self {
            Ok(inner) => inner.context_iter(),
            _ => None,
        }
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        match self {
            Ok(inner) => inner.get_type(),
            _ => ContextType::Null,
        }
    }
    #[inline]
    fn len(&self) -> usize {
        match self {
            Ok(inner) => inner.len(),
            _ => 0,
        }
    }
}

impl ContextTrait for bool {
    #[inline]
    fn is_truthy(&self) -> bool {
        *self
    }

    #[inline]
    fn render_capacity_hint(&self) -> usize {
        5
    }
    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        write!(write, "{}", self)
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::Bool
    }
    #[inline]
    fn len(&self) -> usize {
        1
    }
}

#[duplicate_item(
  str_type;
  [ &'a str ];
  [ String ];
)]
impl<'a> ContextTrait for str_type {
    #[inline]
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }

    #[inline]
    fn render_capacity_hint(&self) -> usize {
        self.len()
    }
    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        write!(write, "{}", self)
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::String
    }
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
}

impl ContextTrait for () {
    #[inline]
    fn is_truthy(&self) -> bool {
        false
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::Null
    }
    #[inline]
    fn len(&self) -> usize {
        0
    }
}

impl ContextTrait for Value {
    #[inline]
    fn is_truthy(&self) -> bool {
        match *self {
            Value::Number(ref i) => {
                if i.is_i64() {
                    return i.as_i64().unwrap() != 0;
                }
                if i.is_u64() {
                    return i.as_u64().unwrap() != 0;
                }
                let f = i.as_f64().unwrap();
                f != 0.0 && !f.is_nan()
            }
            Value::Bool(ref i) => *i,
            Value::Null => false,
            Value::String(ref i) => !i.is_empty(),
            Value::Array(ref i) => !i.is_empty(),
            Value::Object(ref i) => !i.is_empty(),
        }
    }
    #[inline]
    fn render_capacity_hint(&self) -> usize {
        match *self {
            Value::Number(ref i) => i.render_capacity_hint(),
            Value::Bool(ref i) => i.render_capacity_hint(),
            Value::Null => 0,
            Value::String(ref i) => i.render_capacity_hint(),
            Value::Array(ref i) => i.render_capacity_hint(),
            Value::Object(ref i) => 0,
        }
    }
    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        write!(write, "{}", self)

        // match *self {
        //     Value::String(ref s) => write!(write, "{}", s),
        //     Value::Number(ref i) => {
        //         if let Some(v) = i.as_i64() {
        //             write!(write, "{}", v)
        //         } else if let Some(v) = i.as_u64() {
        //             write!(write, "{}", v)
        //         } else if let Some(v) = i.as_f64() {
        //             write!(write, "{}", v)
        //         } else {
        //             unreachable!()
        //         }
        //     }
        //     Value::Bool(i) => write!(write, "{}", i),
        //     Value::Null => Ok(()),
        //     Value::Array(ref a) => {
        //         let mut first = true;
        //         write!(write, "[")?;
        //         for i in a.iter() {
        //             if !first {
        //                 write!(write, ", ")?;
        //             }
        //             first = false;
        //             i.render(write)?;
        //         }
        //         write!(write, "]")?;
        //         Ok(())
        //     }
        //     Value::Object(_) => write!(write, "[object]"),
        // }
    }

    fn context_iter(&self) -> Option<Box<dyn Iterator<Item = (String, &dyn ContextTrait)>>> {
        // if let Some(array) = self.as_array() {
        //     Some(&array.iter().into())
        // } else
        if let Some(object) = self.as_object() {
            Some(Box::new(object.iter().map(|(key,value)| (key.to_string(),value as &dyn ContextTrait))))
        } else {
            None
        }
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        match *self {
            Value::Number(_) => ContextType::Number,
            Value::Bool(_) => ContextType::Bool,
            Value::Null => ContextType::Null,
            Value::String(_) => ContextType::String,
            Value::Array(_) => ContextType::Array,
            Value::Object(_) => ContextType::Object
        }
    }
    #[inline]
    fn len(&self) -> usize {
        match *self {
            Value::Number(ref i) => i.len(),
            Value::Bool(ref i) => i.len(),
            Value::Null => 0,
            Value::String(ref i) => i.len(),
            Value::Array(ref i) => i.len(),
            Value::Object(ref i) => i.len(),
        }
    }
}

impl ContextTrait for Number {
    #[inline]
    fn is_truthy(&self) -> bool {
        if self.is_i64() {
            return self.as_i64().unwrap() != 0;
        }
        if self.is_u64() {
            return self.as_u64().unwrap() != 0;
        }
        let f = self.as_f64().unwrap();
        f != 0.0 && !f.is_nan()
    }
    #[inline]
    fn render_capacity_hint(&self) -> usize {
        if self.is_truthy() {
            5
        } else {
            0
        }
    }
    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        write!(write, "{}", self)
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::Number
    }
    #[inline]
    fn len(&self) -> usize {
        1
    }
}

impl<T: ContextTrait> ContextTrait for Option<T> {
    #[inline]
    fn is_truthy(&self) -> bool {
        self.is_some()
    }
    #[inline]
    fn render_capacity_hint(&self) -> usize {
        match self {
            Some(inner) => inner.render_capacity_hint(),
            _ => 0,
        }
    }
    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        match self {
            Some(inner) => inner.render(write),
            _ => Ok(()),
        }
    }

    #[inline]
    fn context_iter(&self) -> Option<Box<dyn Iterator<Item = (String, &dyn ContextTrait)>>> {
        match self {
            Some(inner) => inner.context_iter(),
            _ => None,
        }
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        match self {
            Some(inner) => inner.get_type(),
            _ => ContextType::Null,
        }
    }
    #[inline]
    fn len(&self) -> usize {
        match self {
            Some(inner) => inner.len(),
            _ => 0,
        }
    }
}

#[duplicate_item(
  array_type;
  [ Vec<T> ];
  [ [T] ];
)]
impl<T: ContextTrait> ContextTrait for array_type {
    #[inline]
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }
    #[inline]
    fn render_capacity_hint(&self) -> usize {
        self.iter().map(|item| item.render_capacity_hint()).sum()
    }
    #[inline]
    fn context_iter(&self) -> Option<Box<dyn Iterator<Item = (String, &dyn ContextTrait)>>> {
        Some(Box::new(self.into_iter().enumerate().map(|(index,item)| (index.to_string(), item as &dyn ContextTrait))))
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::Array
    }
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
}

// impl<K, V> ContextTrait for (K, V)
// where
//     K: Borrow<str> + Debug,
//     V: ContextTrait,
// {
//     #[inline]
//     fn is_truthy(&self) -> bool {
//         self.1.is_truthy()
//     }
//     #[inline]
//     fn render_capacity_hint(&self) -> usize {
//         self.1.render_capacity_hint()
//     }
// }


impl<K, V> ContextTrait for (K, V)
where
    K: Borrow<str> + Debug,
    V: ContextTrait,
{
    #[inline]
    fn is_truthy(&self) -> bool {
        self.1.is_truthy()
    }
    #[inline]
    fn render_capacity_hint(&self) -> usize {
        self.1.render_capacity_hint()
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        self.1.get_type()
    }
    #[inline]
    fn len(&self) -> usize {
        1
    }
}

impl<K, V, S> ContextTrait for HashMap<K, V, S>
where
    K: Borrow<str> + Hash + Eq + Debug + Into<String>,
    V: ContextTrait,
    S: BuildHasher,
{
    #[inline]
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }

    #[inline]
    fn render_capacity_hint(&self) -> usize {
        self.iter().map(|(_key, value)| value.render_capacity_hint()).sum()
    }
    #[inline]
    fn context_iter(&self) -> Option<Box<dyn Iterator<Item = (String, &dyn ContextTrait)>>> {
        Some(Box::new(self.iter().map(|(key,value)| (key.into(), value as &dyn ContextTrait))))
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::Object
    }
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
}

impl<K, V> ContextTrait for BTreeMap<K, V>
where
    K: Borrow<str> + Ord + Debug + Into<String>,
    V: ContextTrait,
{
    #[inline]
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }

    #[inline]
    fn render_capacity_hint(&self) -> usize {
        self.iter().map(|(_key, value)| value.render_capacity_hint()).sum()
    }

    #[inline]
    fn pointer(&self, key: &str) -> Option<Arc<&dyn ContextTrait>>
    where
        Self: Sized,
    {
        self.get(key).map(|value| Arc::new(value as &dyn ContextTrait))
    }

    #[inline]
    fn context_iter(&self) -> Option<Box<dyn Iterator<Item = (String, &dyn ContextTrait)>>> {
        Some(Box::new(self.iter().map(|(key,value)|{
            let key_string: String = *key.into();
            (key_string, value as &dyn ContextTrait)
        })))
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        ContextType::Object
    }
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
}

#[duplicate_item(
  pointer_type;
  [ &'a T ];
  [ Box<T> ];
  [ Rc<T> ];
  [ Arc<T> ];
)]
impl<'a, T> ContextTrait for pointer_type
where
    T: ContextTrait,
{
    #[inline]
    fn is_truthy(&self) -> bool {
        self.deref().is_truthy()
    }

    #[inline]
    fn render_capacity_hint(&self) -> usize {
        self.deref().render_capacity_hint()
    }
    #[inline]
    fn render(&self, write: &mut dyn Write) -> std::io::Result<()> {
        self.deref().render(write)
    }
    #[inline]
    fn context_iter(&self) -> Option<Box<dyn Iterator<Item = (String, &dyn ContextTrait)>>> {
        self.deref().context_iter()
    }
    #[inline]
    fn get_type(&self) -> ContextType {
        self.deref().get_type()
    }
    #[inline]
    fn len(&self) -> usize {
        self.deref().len()
    }
}
