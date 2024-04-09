use attribute::attribute_info;
use attribute::attribute_info_Data::*;
use byteorder::{BigEndian, ReadBytesExt};
use class::ClassRef;
use class_file::ClassLoadingError;
use constant_pool::ConstantPool;
use field::FieldDescriptor;
use method;
use method::ReturnDescriptor::*;
use std::io::Read;
use std::iter::{Enumerate, Peekable};
use std::str::Chars;
use {attribute, field};
use class::ClassRef::Symbolic;

#[derive(Debug)]
/// Raw data contained in a .class file
///
/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.6>
pub struct method_info {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes_count: u16,
    attributes: Vec<attribute::attribute_info>,
}

#[derive(Debug)]
/// Describes the signature of a method
pub struct MethodDescriptor<'a> {
    parameters: Vec<FieldDescriptor<'a>>,
    return_type: ReturnDescriptor<'a>,
}

#[derive(Debug)]
/// Describes the return type of a method
pub enum ReturnDescriptor<'a> {
    Value(FieldDescriptor<'a>),
    Void,
}

#[derive(Debug)]
/// A reference to a named method of a specific class
pub enum MethodRef<'a> {
    Symbolic(&'a str),
    Static(&'a MethodInfo<'a>),
}

#[derive(Debug)]
/// A named method beloning to a specific class
pub struct MethodInfo<'a> {
    name: &'a str,
    parent_class: ClassRef<'a>,
    descriptor: MethodDescriptor<'a>,
    code: Option<Vec<u8>>,
}

impl method_info {
    pub fn new(
        input: &mut Read,
        constant_pool: &ConstantPool,
    ) -> Result<method_info, ClassLoadingError> {
        let access_flags = input.read_u16::<BigEndian>().unwrap();
        let name_index = input.read_u16::<BigEndian>().unwrap();
        let descriptor_index = input.read_u16::<BigEndian>().unwrap();
        let attributes_count = input.read_u16::<BigEndian>().unwrap();
        let attributes = attribute::read_attributes(input, attributes_count, constant_pool)?;
        Ok(method_info {
            access_flags,
            name_index,
            descriptor_index,
            attributes_count,
            attributes,
        })
    }
}

pub fn read_methods<'a, 'b, 'c>(
    input: &mut Read,
    length: u16,
    constant_pool: &ConstantPool<'a>,
    self_reference_name: &'a str,
) -> Result<Vec<MethodInfo<'a>>, ClassLoadingError> {
    let mut vector = Vec::with_capacity(length as usize);
    for _ in 0..length {
        let method_meta = method_info::new(input, constant_pool)?;
        let name = constant_pool.get_string_entry(method_meta.name_index);
        let descriptor_str = constant_pool.get_string_entry(method_meta.descriptor_index);
        let descriptor = parse_method_descriptor(
            &mut descriptor_str.chars().enumerate().peekable(),
            descriptor_str,
        );
        let code = method::get_code(&method_meta.attributes);
        let method_info = MethodInfo {
            name,
            parent_class: Symbolic(self_reference_name),
            descriptor,
            code,
        };
        vector.push(method_info);
    }
    Ok(vector)
}

fn get_code(attributes: &Vec<attribute_info>) -> Option<Vec<u8>> {
    for info in attributes.iter() {
        if let Code_attribute { code, .. } = info.get_data() {
            return Some(code.clone());
        }
    }
    None
}

/// Parse a method signature from a valid method descriptor
///
/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.3.3>
fn parse_method_descriptor<'a, 'b>(
    chars: &mut Peekable<Enumerate<Chars<'a>>>,
    source: &'b str,
) -> MethodDescriptor<'b> {
    if chars.next().unwrap().1 != '(' {
        panic!("Method Descriptor not valid: {}", source);
    }
    let mut parameters = Vec::new();
    while chars.peek().unwrap().1 != ')' {
        parameters.push(field::parse_field_descriptor(chars, source));
    }
    chars.next();
    let return_type = parse_return_descriptor(chars, source);
    MethodDescriptor {
        parameters,
        return_type,
    }
}

/// Parse a return value from the end of a method descriptor
///
/// This will either be a valid field descriptor or void (V)
fn parse_return_descriptor<'a, 'b>(
    chars: &mut Peekable<Enumerate<Chars<'a>>>,
    source: &'b str,
) -> ReturnDescriptor<'b> {
    if chars.peek().unwrap().1 == 'V' {
        Void
    } else {
        Value(field::parse_field_descriptor(chars, source))
    }
}

/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.6-200-A.1>
enum MethodAccessFlag {
    ACC_PUBLIC = 0x0001,
    ACC_PRIVATE = 0x0002,
    ACC_PROTECTED = 0x0004,
    ACC_STATIC = 0x0008,
    ACC_FINAL = 0x0010,
    ACC_SYNCHRONIZED = 0x0020,
    ACC_BRIDGE = 0x0040,
    ACC_VARARGS = 0x0080,
    ACC_NATIVE = 0x0100,
    ACC_ABSTRACT = 0x0400,
    ACC_STRICT = 0x0800,
    ACC_SYNTHETIC = 0x1000,
}
