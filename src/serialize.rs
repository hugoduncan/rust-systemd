// Copyright 2015 Hugo Duncan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Serialization support for data over DBus.
//!
//! Serialize between Rustc Encodable and Decodable types, and
//! Vec<dbus::MessageItem>.

use std::{num,string};
use std::ascii::AsciiExt;

use dbus::MessageItem;
use rustc_serialize::{self, Decodable, Encodable};

use self::DecoderError::*;
use self::EncoderError::*;


/// Error type for serialization
#[derive(Clone, PartialEq, Debug)]
pub enum DecoderError{
    /// Type not implemented by the decoder
    NotImplemented(String),
    /// Expected error
    ExpectedError(string::String, string::String),
    /// Variant type unknown
    UnknownVariantError(string::String),
    /// Other errors
    ApplicationError(string::String)
}

/// Decoder
pub struct Decoder {
    stack: Vec<MessageItem>
}

impl Decoder {
    fn pop(&mut self) -> MessageItem {
        if self.stack.is_empty() {
            assert!(false, "Nothing to pop");
        }
        self.stack.pop().unwrap()
    }

    /// Return a new Decoder instance which will parse the
    /// passed `items`.
    pub fn new(mut items: Vec<MessageItem>) -> Decoder {
        Decoder { stack: {items.reverse(); items} }
    }

}

/// Result type for decoding dbus messages
pub type DecodeResult<T> = Result<T, DecoderError>;

macro_rules! expect {
    ($e:expr, $t:ident) => ({
        match $e {
            MessageItem::$t(v) => Ok(v),
            other => {
                Err(ExpectedError(stringify!($t).to_string(),
                                  format!("{:?}", other)))
            }
        }
    })
}

macro_rules! expect2 {
    ($e:expr, $t:ident) => ({
        match $e {
            MessageItem::$t(v,w) => Ok((v,w)),
            other => {
                Err(ExpectedError(stringify!($t).to_string(),
                                  format!("{:?}", other)))
            }
        }
    })
}

macro_rules! read_int {
    ($name:ident, $ty:ty) => {
        fn $name(&mut self) -> DecodeResult<$ty> {
            match self.pop() {
                MessageItem::Int16(f) => match num::cast(f) {
                    Some(f) => Ok(f),
                    None => Err(ExpectedError("Number".to_string(), format!("{}", f))),
                },
                MessageItem::Int32(f) => match num::cast(f) {
                    Some(f) => Ok(f),
                    None => Err(ExpectedError("Number".to_string(), format!("{}", f))),
                },
                MessageItem::Int64(f) => match num::cast(f) {
                    Some(f) => Ok(f),
                    None => Err(ExpectedError("Number".to_string(), format!("{}", f))),
                },
                MessageItem::UInt16(f) => match num::cast(f) {
                    Some(f) => Ok(f),
                    None => Err(ExpectedError("Number".to_string(), format!("{}", f))),
                },
                MessageItem::UInt32(f) => match num::cast(f) {
                    Some(f) => Ok(f),
                    None => Err(ExpectedError("Number".to_string(), format!("{}", f))),
                },
                MessageItem::UInt64(f) => match num::cast(f) {
                    Some(f) => Ok(f),
                    None => Err(ExpectedError("Number".to_string(), format!("{}", f))),
                },
                MessageItem::Double(f) => Err(ExpectedError("Integer".to_string(), format!("{}", f))),
                value => Err(ExpectedError("Number".to_string(), format!("{:?}", value))),
            }
        }
    }
}

impl rustc_serialize::Decoder for Decoder {
    type Error = DecoderError;

    fn read_nil(&mut self) -> DecodeResult<()> {
        assert!(false, "not implemented");
        Err(NotImplemented("nil".to_string()))
    }

    read_int! { read_usize, usize }
    read_int! { read_u8, u8 }
    read_int! { read_u16, u16 }
    read_int! { read_u32, u32 }
    read_int! { read_u64, u64 }
    read_int! { read_isize, isize }
    read_int! { read_i8, i8 }
    read_int! { read_i16, i16 }
    read_int! { read_i32, i32 }
    read_int! { read_i64, i64 }

    fn read_f32(&mut self) -> DecodeResult<f32> {
        self.read_f64().map(|x| x as f32)
    }

    fn read_f64(&mut self) -> DecodeResult<f64> {
        match self.pop() {
            MessageItem::Int16(f) => Ok(f as f64),
            MessageItem::Int32(f) => Ok(f as f64),
            MessageItem::Int64(f) => Ok(f as f64),
            MessageItem::UInt16(f) => Ok(f as f64),
            MessageItem::UInt32(f) => Ok(f as f64),
            MessageItem::UInt64(f) => Ok(f as f64),
            MessageItem::Double(f) => Ok(f),
            value => Err(ExpectedError("Number".to_string(), format!("{:?}", value)))
        }
    }

    fn read_bool(&mut self) -> DecodeResult<bool> {
        expect!(self.pop(), Bool)
    }

    fn read_char(&mut self) -> DecodeResult<char> {
        let s = try!(self.read_str());
        {
            let mut it = s.chars();
            match (it.next(), it.next()) {
                // exactly one character
                (Some(c), None) => return Ok(c),
                _ => ()
            }
        }
        Err(ExpectedError("single character string".to_string(), format!("{}", s)))
    }

    fn read_str(&mut self) -> DecodeResult<string::String> {
        // println!("decode str");
        match self.pop() {
            MessageItem::Str(v) => Ok(v),
            MessageItem::ObjectPath(v) => Ok(v),
            other => Err(ExpectedError("Str".to_string(),
                                       format!("{:?}", other)))
        }
    }

    fn read_enum<T, F>(&mut self, _name: &str, f: F) -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        f(self)
    }

    fn read_enum_variant<T, F>(&mut self, names: &[&str],
                               mut f: F) -> DecodeResult<T>
        where F: FnMut(&mut Decoder, usize) -> DecodeResult<T>,
    {
        let name = match self.pop() {
            MessageItem::Str(s) => s,
            v => {
                return Err(ExpectedError("String or Object".to_string(), format!("{:?}", v)))
            }
        };
        let idx = match names.iter().position(|n| *n == &name[]) {
            Some(idx) => idx,
            None => return Err(UnknownVariantError(name))
        };
        f(self, idx)
    }

    fn read_enum_variant_arg<T, F>(&mut self, _idx: usize, f: F) -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        f(self)
    }

    fn read_enum_struct_variant<T, F>(&mut self, names: &[&str], f: F) -> DecodeResult<T> where
        F: FnMut(&mut Decoder, usize) -> DecodeResult<T>,
    {
        self.read_enum_variant(names, f)
    }


    fn read_enum_struct_variant_field<T, F>(&mut self,
                                         _name: &str,
                                         idx: usize,
                                         f: F)
                                         -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        self.read_enum_variant_arg(idx, f)
    }

    fn read_struct<T, F>(&mut self, _name: &str, _len: usize, f: F) -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        let s=try!(expect!(self.pop(), Struct));
        let mut decoder = Decoder::new(s);
        let value = try!(f(&mut decoder));
        Ok(value)
    }

    fn read_struct_field<T, F>(&mut self,
                               _: &str, // name
                               _idx: usize,
                               f: F)
                               -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>
    {
        let value = try!(f(self));
        Ok(value)
    }

    fn read_tuple<T, F>(&mut self, tuple_len: usize, f: F) -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        self.read_seq(move |d, len| {
            if len == tuple_len {
                f(d)
            } else {
                Err(ExpectedError(format!("Tuple{}", tuple_len), format!("Tuple{}", len)))
            }
        })
    }

    fn read_tuple_arg<T, F>(&mut self, idx: usize, f: F) -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        self.read_seq_elt(idx, f)
    }

    fn read_tuple_struct<T, F>(&mut self,
                               _name: &str,
                               len: usize,
                               f: F)
                               -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        self.read_tuple(len, f)
    }

    fn read_tuple_struct_arg<T, F>(&mut self,
                                   idx: usize,
                                   f: F)
                                   -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        self.read_tuple_arg(idx, f)
    }

    fn read_option<T, F>(&mut self, mut f: F) -> DecodeResult<T> where
        F: FnMut(&mut Decoder, bool) -> DecodeResult<T>,
    {
        match self.pop() {
            value => { self.stack.push(value); f(self, true) }
        }
    }

    fn read_seq<T, F>(&mut self, f: F) -> DecodeResult<T> where
        F: FnOnce(&mut Decoder, usize) -> DecodeResult<T>,
    {
        let (array, len) = try!(expect2!(self.pop(), Array));
        for v in array.into_iter().rev() {
            self.stack.push(v);
        }
        f(self, num::cast(len).unwrap())
    }

    fn read_seq_elt<T, F>(&mut self, _idx: usize, f: F) -> DecodeResult<T> where
        F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        f(self)
    }

    fn read_map<T, F>(&mut self, _: F) -> DecodeResult<T> where
        F: FnOnce(&mut Decoder, usize) -> DecodeResult<T>,
    {
        assert!(false,"not implemented");
        Err(NotImplemented("map".to_string()))
    }

    fn read_map_elt_key<T, F>(&mut self, _idx: usize, f: F) -> DecodeResult<T> where
       F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        f(self)
    }

    fn read_map_elt_val<T, F>(&mut self, _idx: usize, f: F) -> DecodeResult<T> where
       F: FnOnce(&mut Decoder) -> DecodeResult<T>,
    {
        f(self)
    }

    fn error(&mut self, err: &str) -> DecoderError {
        ApplicationError(err.to_string())
    }
}

/// Convenience function to decode a vector of MessageItem
pub fn decode<T: Decodable>(items: Vec<MessageItem>) -> DecodeResult<T> {
    let mut decoder = Decoder::new(items);
    Decodable::decode(&mut decoder)
}

/// Encoder Error Type
#[derive(Clone, PartialEq, Debug)]
pub enum EncoderError{
    /// Unimplemented type in the encoder
    EncodeNotImplemented(String),
    /// Other encoder error
    InternalEncodeError(String)
}

/// Result type for encoder functions.
pub type EncodeResult<T> = Result<T, EncoderError>;


#[derive(Clone, PartialEq, Debug)]
enum EncoderValue {
    Scalar(Option<MessageItem>),
    Array(Vec<MessageItem>),
    Struct(Vec<MessageItem>)
}

/// Encoder object, to encode to Vec<MessageItem>
#[derive(Clone, PartialEq, Debug)]
pub struct Encoder {
    r: EncoderValue
}

impl Encoder {
    fn new() -> Encoder {
        Encoder{r: EncoderValue::Scalar(None)}
    }

    fn emit(&mut self, v: MessageItem) -> EncodeResult<()> {
        match &mut self.r {
            &mut EncoderValue::Scalar(ref mut x) => {
                if x.is_none() {
                    *x = Some(v);
                    Ok(())
                }  else {
                    Err(InternalEncodeError("Item already has a value".to_string()))
                }
            },
            &mut EncoderValue::Array(ref mut x) => {
                x.push(v);
                Ok(())
            },
            &mut EncoderValue::Struct(ref mut x) => {
                x.push(v);
                Ok(())
            }
        }
    }

    fn value(&mut self) -> EncodeResult<MessageItem> {
        match self.r {
            EncoderValue::Scalar(ref mut v) =>
                match v {
                    &mut None =>
                        Err(InternalEncodeError("no value set".to_string())),
                    &mut Some(ref v) =>
                        Ok(v.clone())},
            EncoderValue::Struct(ref mut v) =>
                Ok(MessageItem::Struct(v.clone())),
            EncoderValue::Array(ref mut v) =>
                Ok(MessageItem::Array(v.clone(), v.len() as i32))
        }

    }
}

/// Utility function to encode an Encodable instance to a MessageItem
pub fn encode<T: Encodable>(x: T) -> EncodeResult<MessageItem> {
    let mut encoder = Encoder::new();
    {
        try!(x.encode(&mut encoder));
    }
    encoder.value()
}

impl rustc_serialize::Encoder for Encoder {
    type Error = EncoderError;

    fn emit_nil(&mut self) -> EncodeResult<()> {
        Err(EncodeNotImplemented("Encoding not implemented for nil".to_string()))
    }

    fn emit_usize(&mut self, v: usize) -> EncodeResult<()> {
        self.emit(MessageItem::UInt32(v as u32))
    }

    fn emit_u64(&mut self, v: u64) -> EncodeResult<()> {
        self.emit(MessageItem::UInt64(v))
    }

    fn emit_u32(&mut self, v: u32) -> EncodeResult<()> {
        self.emit(MessageItem::UInt32(v))
    }

    fn emit_u16(&mut self, v: u16) -> EncodeResult<()> {
        self.emit(MessageItem::UInt16(v))
    }

    fn emit_u8(&mut self, v: u8) -> EncodeResult<()> {
        self.emit(MessageItem::Byte(v))
    }

    fn emit_isize(&mut self, v: isize) -> EncodeResult<()> {
        self.emit(MessageItem::Int32(v as i32))
    }

    fn emit_i64(&mut self, v: i64) -> EncodeResult<()> {
        self.emit(MessageItem::Int64(v))
    }

    fn emit_i32(&mut self, v: i32) -> EncodeResult<()> {
        self.emit(MessageItem::Int32(v))
    }

    fn emit_i16(&mut self, v: i16) -> EncodeResult<()> {
        self.emit(MessageItem::Int16(v))
    }

    fn emit_i8(&mut self, v: i8) -> EncodeResult<()> {
        self.emit(MessageItem::Byte(v as u8))
    }

    fn emit_bool(&mut self, v: bool) -> EncodeResult<()> {
        self.emit(MessageItem::Bool(v))
    }

    fn emit_f64(&mut self, v: f64) -> EncodeResult<()> {
        self.emit(MessageItem::Double(v))
    }

    fn emit_f32(&mut self, v: f32) -> EncodeResult<()> {
        self.emit(MessageItem::Double(v as f64))
    }

    fn emit_char(&mut self, _: char) -> EncodeResult<()> {
        Err(EncodeNotImplemented("Encode not implemented for char".to_string()))
    }

    fn emit_str(&mut self, v: &str) -> EncodeResult<()> {
        self.emit(MessageItem::Str(v.to_string()))
    }

    fn emit_enum<F>(&mut self, _name: &str, f: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        println!("Emit enum {}", _name);
        f(self).unwrap();
        Ok(())
    }

    fn emit_enum_variant<F>(&mut self,
                            name: &str, // name
                            _id: usize,
                            _cnt: usize, // cnt
                            _f: F) //f
                            -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        println!("Emit enum variant {}", name);
        self.emit(MessageItem::Str(
            name.to_ascii_lowercase().to_string())).unwrap();
        Ok(())
    }

    fn emit_enum_variant_arg<F>(&mut self, _: usize, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        println!("Emit enum variant arg");
        Err(EncodeNotImplemented("Encode not implemented for enum variant arg".to_string()))
    }

    fn emit_enum_struct_variant<F>(&mut self,
                                   _name: &str, //name
                                   _: usize, // id
                                   _: usize, // cnt
                                   f: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        println!("Emit enum struct variant {}", _name);
        let mut encoder=Encoder{r: EncoderValue::Struct(vec![])};
        try!(f(&mut encoder));
        self.emit(try!(encoder.value()))
    }

    fn emit_enum_struct_variant_field<F>(&mut self,
                                         _name: &str,
                                         _: usize,
                                         f: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        println!("Emit enum struct variant field {}", _name);
        f(self).unwrap();
        Ok(())
    }


    fn emit_struct<F>(&mut self, _: &str, _: usize, f: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        let mut encoder=Encoder{r: EncoderValue::Struct(vec![])};
        try!(f(&mut encoder));
        self.emit(try!(encoder.value()))
    }

    fn emit_struct_field<F>(&mut self, _: &str, _: usize, f: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>
    {
        f(self).unwrap();
        Ok(())
    }

    fn emit_tuple<F>(&mut self, _: usize, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>
    {
        Err(EncodeNotImplemented("Encode not implemented for tuple".to_string()))
    }

    fn emit_tuple_arg<F>(&mut self, _: usize, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>
    {
        Err(EncodeNotImplemented("Encode not implemented for tuple arg".to_string()))
    }

    fn emit_tuple_struct<F>(&mut self, _: &str, _: usize, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>
    {
        Err(EncodeNotImplemented("Encode not implemented for tuple struct".to_string()))
    }

    fn emit_tuple_struct_arg<F>(&mut self, _: usize, f: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>
    {
        f(self)
    }

    fn emit_option<F>(&mut self, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>
    {
        Err(EncodeNotImplemented("Encode not implemented for Option".to_string()))
    }

    fn emit_option_none(&mut self) -> EncodeResult<()> {
        Err(EncodeNotImplemented("Encode not implemented for Option::None".to_string()))
    }

    fn emit_option_some<F>(&mut self, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        Err(EncodeNotImplemented("Encode not implemented for Option::Some".to_string()))
    }

    fn emit_seq<F>(&mut self, _: usize, f: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        let mut encoder=Encoder{r: EncoderValue::Array(vec![])};
        try!(f(&mut encoder));
        self.emit(try!(encoder.value()))
    }

    fn emit_seq_elt<F>(&mut self, _: usize, f: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        f(self)
    }

    fn emit_map<F>(&mut self, _: usize, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        Err(EncodeNotImplemented("Encode not implemented for map".to_string()))
    }

    fn emit_map_elt_key<F>(&mut self, _: usize, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        Err(EncodeNotImplemented("Encode not implemented for map key".to_string()))
    }

    fn emit_map_elt_val<F>(&mut self, _idx: usize, _: F) -> EncodeResult<()> where
        F: FnOnce(&mut Encoder) -> EncodeResult<()>,
    {
        Err(EncodeNotImplemented("Encode not implemented for map value".to_string()))
    }
}






#[cfg(test)]
mod tests {
    use super::*;
    use dbus::MessageItem;

    #[derive(RustcDecodable, RustcEncodable)]
    struct IntField {i: i64}

    // #[test]
    // fn test_no_fields() {
    //     let v = decode::<IntField>(vec![MessageItem::Int16(0)]);
    //     assert!(v.is_ok(), "Returned no input");
    // }

    #[test]
    fn encode_struct_with_int_field() {
        let ifield = IntField{i: 42};
        let v = encode(&ifield).unwrap();
        assert_eq!(MessageItem::Struct(vec![MessageItem::Int64(42)]), v);
    }

    #[derive(RustcDecodable, RustcEncodable)]
    enum TestEnum{
        A,
        B
    }

    #[test]
    fn encode_enum() {
        let e = TestEnum::A;
        let v = encode(&e).unwrap();
        assert_eq!(MessageItem::Str("A".to_string()), v);
    }

}
