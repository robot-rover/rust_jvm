use attribute;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use class::ClassRef;
use class::ClassRef::Symbolic;
use class_file::ClassLoadingError;
use constant_pool::ConstantPool;
use field::FieldDescriptor::*;
use std::io::Read;
use std::iter::{Enumerate, Peekable};
use std::str::Chars;

#[derive(Debug)]
/// Raw data contained in a .class file (ClassFile#fields[])
///
///  <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.1>
pub struct field_info {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes_count: u16,
    attributes: Vec<attribute::attribute_info>,
}

#[derive(Debug)]
/// Describes the type of a field
pub enum FieldDescriptor<'a> {
    Byte,
    Character,
    Double,
    Float,
    Integer,
    Long,
    Reference(ClassRef<'a>),
    Short,
    Boolean,
}

#[derive(Debug)]
/// A named field belonging to a specific class
pub struct FieldInfo<'a> {
    name: &'a str,
    parent_class: ClassRef<'a>,
    descriptor: FieldDescriptor<'a>,
    index: u16,
}

#[derive(Debug)]
/// A reference to a field of a specific class
pub enum FieldRef<'a> {
    Symbolic(&'a str),
    Static(&'a FieldInfo<'a>),
}

/// Reads the array of fields from a class file
///
/// self_reference_index -> CONSTANT_Utf8_attribute that is the name of this class
pub fn read_fields<'a, 'b, 'c>(
    input: &'b mut Read,
    length: u16,
    constant_pool: &'c ConstantPool<'a>,
    self_reference_name: &'a str,
) -> Result<Vec<FieldInfo<'a>>, ClassLoadingError> {
    let mut vector = Vec::with_capacity(length as usize);
    for index in 0..length {
        let field_meta = field_info::new(input, constant_pool)?;
        let name = constant_pool.get_string_entry(field_meta.name_index);
        let descriptor_str = constant_pool.get_string_entry(field_meta.descriptor_index);
        let descriptor = parse_field_descriptor(
            &mut descriptor_str.chars().enumerate().peekable(),
            descriptor_str,
        );
        let field_info = FieldInfo {
            name,
            parent_class: Symbolic(self_reference_name),
            descriptor,
            index,
        };
        vector.push(field_info);
    }
    Ok(vector)
}

/// Parse the type of a field from a valid field descriptor
///
/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.3.2>
pub fn parse_field_descriptor<'a, 'b>(
    chars: &mut Peekable<Enumerate<Chars<'a>>>,
    source: &'b str,
) -> FieldDescriptor<'b> {
    parse_field_descriptor_index(chars, source).0
}

/// internal method which gives the parsed field and the index of the last character accessed
fn parse_field_descriptor_index<'a, 'b>(
    chars: &mut Peekable<Enumerate<Chars<'a>>>,
    source: &'b str,
) -> (FieldDescriptor<'b>, usize) {
    let first_char = chars.peek().unwrap().1;
    match first_char {
        'L' => return parse_field_descriptor_reference(chars, source),
        '[' => return parse_field_descriptor_array(chars, source),
        _ => {}
    }
    let first_char = chars.next().unwrap();
    (
        match first_char.1 {
            'B' => Byte,
            'C' => Character,
            'D' => Double,
            'F' => Float,
            'I' => Integer,
            'J' => Long,
            'S' => Short,
            'Z' => Boolean,

            _ => panic!(
                "Illegal character in field descriptor: {} -> '{}'",
                source, first_char.1
            ),
        },
        first_char.0,
    )
}

/// parse a reference type (eg `Ljava/lang/Object;`)
fn parse_field_descriptor_reference<'a, 'b>(
    chars: &mut Peekable<Enumerate<Chars<'a>>>,
    source: &'b str,
) -> (FieldDescriptor<'b>, usize) {
    let first = chars.next().unwrap();
    let second_index = first.0 + 1;
    let mut last_index = second_index;
    while chars.next().unwrap().1 != ';' {
        last_index += 1;
    }
    (
        Reference(Symbolic(&source[second_index..last_index])),
        last_index,
    )
}

/// parse an array type (eg `[[I` or `[java/lang/Object;`)
fn parse_field_descriptor_array<'a, 'b>(
    chars: &mut Peekable<Enumerate<Chars<'a>>>,
    source: &'b str,
) -> (FieldDescriptor<'b>, usize) {
    let first_index = chars.peek().unwrap().0;
    while chars.peek().unwrap().1 == '[' {
        chars.next();
    }
    let component = parse_field_descriptor_index(chars, source);
    let last_index = component.1 + 1;
    (
        Reference(Symbolic(&source[first_index..last_index])),
        last_index,
    )
}

impl field_info {
    fn new(
        input: &mut Read,
        constant_pool: &ConstantPool,
    ) -> Result<field_info, ClassLoadingError> {
        let access_flags = input.read_u16::<BigEndian>()?;
        let name_index = input.read_u16::<BigEndian>()?;
        let descriptor_index = input.read_u16::<BigEndian>()?;
        let attributes_count = input.read_u16::<BigEndian>()?;
        let attributes = attribute::read_attributes(input, attributes_count, constant_pool)?;
        Ok(field_info {
            access_flags,
            name_index,
            descriptor_index,
            attributes_count,
            attributes,
        })
    }
}

/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.5-200-A.1>
enum FieldAccessFlag {
    ACC_PUBLIC = 0x0001,
    ACC_PRIVATE = 0x0002,
    ACC_PROTECTED = 0x0004,
    ACC_STATIC = 0x0008,
    ACC_FINAL = 0x0010,
    ACC_VOLATILE = 0x0040,
    ACC_TRANSIENT = 0x0080,
    ACC_SYNTHETIC = 0x1000,
    ACC_ENUM = 0x4000,
}
