use attribute;
use std::io::Read;
use byteorder::ReadBytesExt;
use byteorder::BigEndian;
use constant_pool::ConstantPool;
use class_file::ClassLoadingError;
use class::ClassRef;
use class::ClassRef::Symbolic;
use field::FieldDescriptor::*;
use constant_pool::cp_info::*;
use class_file::ClassFile;

#[derive(Debug)]
/// Raw data contained in a .class file (ClassFile#fields[])
///
///  <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.1>
pub struct field_info {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes_count: u16,
    attributes: Vec<attribute::attribute_info>
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
    Boolean
}

#[derive(Debug)]
/// A named field belonging to a specific class
pub struct FieldInfo<'a> {
    name: &'a str,
    parent_class: ClassRef<'a>,
    descriptor: FieldDescriptor<'a>,
    index: u16
}

#[derive(Debug)]
/// A reference to a field of a specific class
pub enum FieldRef<'a> {
    Symbolic(&'a str),
    Static(&'a FieldInfo<'a>)
}

/// Reads the array of fields from a class file
///
/// self_reference_index -> CONSTANT_Utf8_attribute that is the name of this class
pub fn read_fields<'a, 'b, 'c>(input: &'b mut Read, length: u16, constant_pool: &'c ConstantPool<'a>, self_reference_name: &'a str) -> Result<Vec<FieldInfo<'a>>, ClassLoadingError> {
    let mut vector = Vec::with_capacity(length as usize);
    for index in 0..length {
        let field_meta = field_info::new(input, constant_pool)?;
        let name = constant_pool.get_string_entry(field_meta.name_index);
        let descriptor = parse_field_name(constant_pool.get_string_entry(field_meta.descriptor_index));
        let field_info = FieldInfo {
            name,
            parent_class: Symbolic(self_reference_name),
            descriptor,
            index
        };
        vector.push(field_info);
    }
    Ok(vector)
}

/// Parses a valid field descriptor
///
/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.3.2>
pub fn parse_field_name(name: &str) -> FieldDescriptor {
    let mut chars = name.chars();
    match chars.next().unwrap() {
        'B' => Byte,
        'C' => Character,
        'D' => Double,
        'F' => Float,
        'I' => Integer,
        'J' => Long,
        'L' => Reference(Symbolic(&name[1..(name.len()-1)])),
        'S' => Short,
        'Z' => Boolean,
        '[' => Reference(Symbolic(name)),
        _   => panic!("Illegal Field Descriptor: {}", name)
    }
}

impl field_info {
    fn new(input: &mut Read, constant_pool: &ConstantPool) -> Result<field_info, ClassLoadingError> {
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
            attributes
        })
    }
}

/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.5-200-A.1>
enum FieldAccessFlag {
    ACC_PUBLIC      = 0x0001,
    ACC_PRIVATE     = 0x0002,
    ACC_PROTECTED   = 0x0004,
    ACC_STATIC      = 0x0008,
    ACC_FINAL       = 0x0010,
    ACC_VOLATILE    = 0x0040,
    ACC_TRANSIENT   = 0x0080,
    ACC_SYNTHETIC   = 0x1000,
    ACC_ENUM        = 0x4000,
}