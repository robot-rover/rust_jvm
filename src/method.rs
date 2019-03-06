use attribute;
use std::io::Read;
use byteorder::{BigEndian, ReadBytesExt};
use constant_pool::ConstantPool;
use class_file::ClassLoadingError;
use field::FieldDescriptor;
use field::FieldDescriptor::*;
use class::ClassRef;
use class::ClassRef::Symbolic;
use std::str::Chars;
use std::iter::Enumerate;

#[derive(Debug)]
/// Raw data contained in a .class file
///
/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.6>
pub struct method_info {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes_count: u16,
    attributes: Vec<attribute::attribute_info>
}

#[derive(Debug)]
/// Describes the signature of a method
pub struct MethodDescriptor<'a> {
    parameters: Vec<FieldDescriptor<'a>>,
    return_type: ReturnDescriptor<'a>
}

#[derive(Debug)]
/// Describes the return type of a method
pub enum ReturnDescriptor<'a> {
    Return(FieldDescriptor<'a>),
    Void
}

#[derive(Debug)]
/// A reference to a named method of a specific class
pub enum MethodRef<'a> {
    Symbolic(&'a str),
    Static(&'a MethodInfo<'a>)
}

#[derive(Debug)]
/// A named method beloning to a specific class
pub struct MethodInfo<'a> {
    name: &'a str,
    parent_class: ClassRef<'a>,
    descriptor: MethodDescriptor<'a>,
    code: Vec<u8>
}

impl method_info {
    pub fn new(input: &mut Read, constant_pool: &ConstantPool) -> Result<method_info, ClassLoadingError> {
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
            attributes
        })
    }
}

pub fn read_methods<'a, 'b, 'c>(input: &mut Read, length: u16, constant_pool: &ConstantPool<'a>, self_reference_name: &'a str) -> Result<Vec<MethodInfo<'a>>, ClassLoadingError> {
    let mut vector = Vec::with_capacity(length as usize);
    for _ in 0..length {
        let method_meta = method_info::new(input, constant_pool)?;
        let name = constant_pool.get_string_entry(method_meta.name_index);
        let descriptor = parse_method_descriptor(constant_pool.get_string_entry(method_meta.descriptor_index));
        let method_info = MethodInfo {
            name,
            parent_class: Symbolic(self_reference_name),
            descriptor,
            code: vec![]
        };
        vector.push(method_info);
    }
    Ok(vector)
}

fn parse_method_descriptor(method_name: &str) -> MethodDescriptor {
    let mut parameters = Vec::new();
    let mut chars = method_name.chars().enumerate();
    if !chars.next().map_or(false, |i| i.1.eq(&'(')) {
        panic!("Method Descriptor did not begin with '('", )
    }
    loop {
        let next_char = chars.next().unwrap().1;
        print!("{}", next_char);
        let next_parameter = match next_char {
            'B' => Byte,
            'C' => Character,
            'D' => Double,
            'F' => Float,
            'I' => Integer,
            'J' => Long,
            'L' => parse_descriptor_reference(method_name, &mut chars),
            'S' => Short,
            'Z' => Boolean,
            '[' => parse_descriptor_array(method_name, &mut chars),
            ')' => break,
            _ => panic!("Illegal Field Descriptor: {}, broke on: '{}'", method_name, next_char)
        };

        parameters.push(next_parameter);
    }

    MethodDescriptor { parameters, return_type: ReturnDescriptor::Void }
}

fn parse_descriptor_reference<'a, 'b>(method_name: &'a str, chars: &'b mut Enumerate<Chars>) -> FieldDescriptor<'a> {
    let startIndex = chars.next().unwrap().0;
    let mut endIndex = startIndex;
    loop {
        let next = chars.next();
        if next.unwrap().1 == ';' {
            break
        }
        endIndex += 1;
    }

    Reference(Symbolic(&method_name[startIndex..endIndex]))
}

fn parse_descriptor_array<'a, 'b>(method_name: &'a str, chars: &'b mut Enumerate<Chars>) -> FieldDescriptor<'a> {
    let mut next = chars.next();
    let startIndex = next.unwrap().0 - 1;
    let mut endIndex = startIndex + 1;
    while next.unwrap().1 == '[' {
        next = chars.next();
        endIndex += 1;
    }
    match next.unwrap().1 {
        'B' | 'C' | 'D' | 'F'| 'I' | 'J' | 'S' | 'Z' => {
            endIndex += 1;
            return Reference(Symbolic(&method_name[startIndex..endIndex]))
        },
        'L' => {
            loop {
                let next = chars.next();
                endIndex += 1;
                if next.unwrap().1 == ';' {
                    break
                }
            }
        },
        _ => panic!("Illegal character in descriptor: {} -> '{}'", method_name, next.unwrap().1)
    }

    Reference(Symbolic(&method_name[startIndex..endIndex]))
}

/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.6-200-A.1>
enum MethodAccessFlag {
    ACC_PUBLIC      = 0x0001,
    ACC_PRIVATE     = 0x0002,
    ACC_PROTECTED   = 0x0004,
    ACC_STATIC      = 0x0008,
    ACC_FINAL       = 0x0010,
    ACC_SYNCHRONIZED= 0x0020,
    ACC_BRIDGE      = 0x0040,
    ACC_VARARGS     = 0x0080,
    ACC_NATIVE      = 0x0100,
    ACC_ABSTRACT    = 0x0400,
    ACC_STRICT      = 0x0800,
    ACC_SYNTHETIC   = 0x1000,
}
