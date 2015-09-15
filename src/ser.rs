//
// ser.rs
// Copyright (C) 2015 Adrian Perez <aperez@igalia.com>
// Distributed under terms of the MIT license.
//

use std::io::Write;
use serde::ser::{self, Serialize, SeqVisitor, MapVisitor};
use super::error::{Result, Error, ErrorCode};


trait Formatter {
    fn start_compound<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
        where W: Write;
    fn end_compound<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
        where W: Write;
    fn key_separator<W>(&mut self, writer: &mut W) -> Result<()>
        where W: Write;
    fn item_separator<W>(&mut self, writer: &mut W, first: bool) -> Result<()>
        where W: Write;
}


pub struct CompactFormatter;

impl Formatter for CompactFormatter {
    fn start_compound<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
        where W: Write
    {
        writer.write_all(&[ch]).map_err(From::from)
    }

    fn end_compound<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
        where W: Write
    {
        writer.write_all(&[ch]).map_err(From::from)
    }

    fn key_separator<W>(&mut self, writer: &mut W) -> Result<()>
        where W: Write
    {
        writer.write_all(b":").map_err(From::from)
    }

    fn item_separator<W>(&mut self, writer: &mut W, first: bool) -> Result<()>
        where W: Write
    {
        if first {
            Ok(())
        } else {
            writer.write_all(b",").map_err(From::from)
        }
    }
}


pub struct PrettyFormatter {
    indent: usize,
}

impl PrettyFormatter {
    fn new() -> Self {
        PrettyFormatter { indent: 0 }
    }
}


#[inline]
fn indent<W>(writer: &mut W, indent: usize) -> Result<()>
    where W: Write
{
    for _ in 0..indent {
        try!(writer.write_all(b"  "));
    }
    Ok(())
}


impl Formatter for PrettyFormatter {
    fn start_compound<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
        where W: Write
    {
        self.indent += 1;
        try!(writer.write_all(&[ch, b'\n']));
        indent(writer, self.indent)
    }

    fn end_compound<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
        where W: Write
    {
        self.indent -= 1;
        try!(writer.write(b"\n"));
        try!(indent(writer, self.indent));
        writer.write_all(&[ch]).map_err(From::from)
    }

    fn key_separator<W>(&mut self, writer: &mut W) -> Result<()>
        where W: Write
    {
        // TODO: Do not write colon when value is dict/list
        writer.write_all(b": ").map_err(From::from)
    }

    fn item_separator<W>(&mut self, writer: &mut W, first: bool) -> Result<()>
        where W: Write
    {
        if first {
            Ok(())
        } else {
            try!(writer.write(b"\n"));
            indent(writer, self.indent)
        }
    }
}


pub struct Serializer<W: Write, F=PrettyFormatter> {
    writer: W,
    format: F,
    first: bool,
}


impl<W: Write> Serializer<W, CompactFormatter> {
    #[inline]
    pub fn new(writer: W) -> Self {
        Serializer::with_formatter(writer, CompactFormatter)
    }
}


impl<W: Write> Serializer<W> {
    #[inline]
    pub fn pretty(writer: W) -> Self {
        Serializer::with_formatter(writer, PrettyFormatter::new())
    }
}


impl<W: Write, F: Formatter> Serializer<W, F> {
    #[inline]
    fn with_formatter(writer: W, format: F) -> Self {
        Serializer {
            writer: writer,
            format: format,
            first: false,
        }
    }
}


impl<W: Write, F: Formatter> ser::Serializer for Serializer<W, F> {
    type Error = Error;

    fn visit_bool(&mut self, v: bool) -> Result<()> {
        self.writer.write_all(if v { b"True" } else { b"False" }).map_err(From::from)
    }

    // Integers
    fn visit_i64(&mut self, v: i64) -> Result<()> {
        write!(self.writer, "{}", v).map_err(From::from)
    }
    fn visit_u64(&mut self, v: u64) -> Result<()> {
        write!(self.writer, "{}", v).map_err(From::from)
    }

    // Float
    fn visit_f64(&mut self, v: f64) -> Result<()> {
        if v.is_nan() || v.is_infinite() {
            write!(self.writer, "{}", v).map_err(From::from)
        } else {
            let s = format!("{}", v);
            try!(self.writer.write_all(s.as_bytes()));
            if !s.contains(".") {
                try!(self.writer.write_all(b".0"));
            }
            Ok(())
        }
    }
    fn visit_str(&mut self, v: &str) -> Result<()> {
        try!(self.writer.write_all(b"\""));
        for ch in v.bytes() {
            try!(match ch {
                0x09 => self.writer.write_all(b"\\t"),
                0x0A => self.writer.write_all(b"\\n"),
                0x0D => self.writer.write_all(b"\\r"),
                0x22 => self.writer.write_all(b"\\\""),
                0x5C => self.writer.write_all(b"\\\\"),
                ch if ch < 0xF => write!(self.writer, "\\0{:X}", ch),
                ch if ch < 0x20 => write!(self.writer, "\\{:X}", ch),
                ch => self.writer.write_all(&[ch]),
            });
        }
        self.writer.write_all(b"\"").map_err(From::from)
    }
    fn visit_unit(&mut self) -> Result<()> {
        Err(Error::SyntaxError(ErrorCode::UnrepresentableValue, 0, 0, 0))
    }
    fn visit_none(&mut self) -> Result<()> {
        self.visit_unit()
    }
    fn visit_some<V>(&mut self, value: V) -> Result<()> where V: Serialize {
        value.serialize(self)
    }
    fn visit_seq<V>(&mut self, mut visitor: V) -> Result<()> where V: SeqVisitor {
        match visitor.len() {
            Some(len) if len == 0 => self.writer.write_all(b"[]").map_err(From::from),
            _ => {
                try!(self.format.start_compound(&mut self.writer, b'['));
                self.first = true;
                while let Some(()) = try!(visitor.visit(self)) {}
                self.format.end_compound(&mut self.writer, b']')
            }
        }
    }
    fn visit_seq_elt<T>(&mut self, value: T) -> Result<()> where T: Serialize {
        try!(self.format.item_separator(&mut self.writer, self.first));
        try!(value.serialize(self));
        self.first = false;
        Ok(())
    }
    fn visit_map<V>(&mut self, mut visitor: V) -> Result<()> where V: MapVisitor {
        match visitor.len() {
            Some(len) if len == 0 => self.writer.write_all(b"{}").map_err(From::from),
            _ => {
                try!(self.format.start_compound(&mut self.writer, b'{'));
                self.first = true;
                while let Some(()) = try!(visitor.visit(self)) {}
                self.format.end_compound(&mut self.writer, b'}')
            }
        }
    }
    fn visit_map_elt<K, V>(&mut self, key: K, value: V) -> Result<()>
        where K: Serialize, V: Serialize
    {
        try!(self.format.item_separator(&mut self.writer, self.first));
        try!(key.serialize(&mut KeySerializer { serializer: self }));
        try!(self.format.key_separator(&mut self.writer));
        try!(value.serialize(self));
        self.first = false;
        Ok(())
    }
}


struct KeySerializer<'a, W: 'a + Write, F: 'a + Formatter> {
    serializer: &'a mut Serializer<W, F>,
}


impl<'a, W: Write, F: Formatter> ser::Serializer for KeySerializer<'a, W, F>
{
    type Error = Error;

    #[inline]
    fn visit_str(&mut self, value: &str) -> Result<()> {
        // TODO: Check that all characters are valid
        self.serializer.writer.write_all(value.as_bytes()).map_err(From::from)
    }

    fn visit_bool(&mut self, _value: bool) -> Result<()> {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_i64(&mut self, _value: i64) -> Result<()> {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_u64(&mut self, _value: u64) -> Result<()> {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_f64(&mut self, _value: f64) -> Result<()> {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_none(&mut self) -> Result<()> {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_unit(&mut self) -> Result<()> {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_some<V>(&mut self, _value: V) -> Result<()> where V: Serialize {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_seq<V>(&mut self, _visitor: V) -> Result<()> where V: SeqVisitor {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_seq_elt<T>(&mut self, _value: T) -> Result<()> where T: Serialize {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_map<V>(&mut self, _visitor: V) -> Result<()> where V: MapVisitor {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }
    fn visit_map_elt<K, V>(&mut self, _key: K, _value: V) -> Result<()>
        where K: Serialize, V: Serialize
    {
        Err(Error::SyntaxError(ErrorCode::InvalidKey, 0, 0, 0))
    }

    fn format() -> &'static str {
        "hipack"
    }
}


#[inline]
pub fn to_writer<W, T>(writer: &mut W, value: &T) -> Result<()>
    where W: Write, T: Serialize
{
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)
}

#[inline]
pub fn to_writer_pretty<W, T>(writer: &mut W, value: &T) -> Result<()>
    where W: Write, T: Serialize
{
    let mut serializer = Serializer::pretty(writer);
    value.serialize(&mut serializer)
}

#[inline]
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
    where T: Serialize
{
    let mut writer = Vec::new();
    try!(to_writer(&mut writer, value));
    Ok(writer)
}

#[inline]
pub fn to_vec_pretty<T>(value: &T) -> Result<Vec<u8>>
    where T: Serialize
{
    let mut writer = Vec::new();
    try!(to_writer_pretty(&mut writer, value));
    Ok(writer)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, BTreeMap};
    use std::f64::{NAN, INFINITY};

    #[test]
    fn test_empty_object() {
        let obj : HashMap<String, i32> = HashMap::new();
        let enc = String::from_utf8(to_vec(&obj).unwrap()).unwrap();
        assert_eq!("{}", enc);
    }

    #[test]
    fn test_one_item_object() {
        let mut obj = HashMap::new();
        obj.insert("k", "v");
        let enc = String::from_utf8(to_vec(&obj).unwrap()).unwrap();
        assert_eq!("{k:\"v\"}", enc);
    }

    #[test]
    fn test_two_item_object() {
        let mut obj = BTreeMap::new();
        obj.insert("pi", 3.14);
        obj.insert("phi", 1.67);
        let enc = String::from_utf8(to_vec(&obj).unwrap()).unwrap();
        assert_eq!("{phi:1.67,pi:3.14}", enc);
    }

    macro_rules! make_write_test {
        ($name:ident, $value:expr, $pretty:expr, $compact:expr) => {
            #[test]
            fn $name() {
                let value = $value;
                {
                    println!("Pretty");
                    let enc = String::from_utf8(to_vec_pretty(&value).unwrap()).unwrap();
                    assert_eq!($pretty, enc);
                }
                {
                    println!("Compact");
                    let enc = String::from_utf8(to_vec(&value).unwrap()).unwrap();
                    assert_eq!($compact, enc);
                }
            }
        }
    }

    make_write_test!(bool_true,  true,  "True",  "True");
    make_write_test!(bool_false, false, "False", "False");

    make_write_test!(list_empty, Vec::<u8>::new(), "[]", "[]");
    make_write_test!(list_one, vec![true], "[\n  True\n]", "[True]");
    make_write_test!(list_two, vec![true, false],
                     "[\n  True\n  False\n]", "[True,False]");

    // TODO: Add list_nested test

    make_write_test!(dict_empty, HashMap::<String, bool>::new(), "{}", "{}");
    make_write_test!(dict_one, {
            let mut b = BTreeMap::new();
            b.insert("item", true);
            b
        }, "{\n  item: True\n}", "{item:True}");
    make_write_test!(dict_two, {
            let mut b = BTreeMap::new();
            b.insert("~t", true);
            b.insert("~f", false);
            b
        }, "{\n  ~f: False\n  ~t: True\n}", "{~f:False,~t:True}");


    macro_rules! make_write_string_tests {
        ($($name:ident, $value:expr, $expected:expr),+) => {
            $( make_write_test!($name, $value, $expected, $expected); )*
        }
    }

    make_write_string_tests!(string_empty, "", "\"\"",
                             string_non_empty, "foo bar", "\"foo bar\"",
                             string_unicode, "☺", "\"☺\"",
                             string_escapes, "\n\r\t\\\"", "\"\\n\\r\\t\\\\\\\"\"",
                             string_hexcode, "\0", "\"\\00\"");

    macro_rules! make_write_number_tests {
        ($($name:ident, $value:expr, $expected:expr),+) => {
            $( make_write_test!($name, $value, $expected, $expected); )*
        }
    }

    make_write_number_tests!(integer_zero, 0, "0",
                             integer_negative, -34, "-34");
    make_write_number_tests!(float_zero, 0.0, "0.0",
                             float_suffix, 1f64, "1.0",
                             float_positive, 4.5, "4.5",
                             float_negative, -3.2, "-3.2",
                             float_nan, NAN, "NaN",
                             float_infinite, INFINITY, "inf");
}
