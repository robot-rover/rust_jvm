/*
ClassFile {
    u4 magic;
    u2 minor_version;
    u2 major_version;
    u2 constant_pool_count;
    cp_info constant_pool[constant_pool_count-1];
    u2 access_flags;
    u2 this_class;
    u2 super_class;
    u2 interfaces_count;
    u2 interfaces[interfaces_count];
    u2 fields_count;
    field_info fields[fields_count];
    u2 methods_count;
    method_info methods[methods_count];
    u2 attributes_count;
    attribute_info attributes[attributes_count];
}*/

use attribute;
use byteorder::{BigEndian, ReadBytesExt};
use class::ClassAccessFlag;
use class::ClassRef;
use class::ClassRef::Symbolic;
use class_file::ClassLoadingError::*;
use constant_pool::cp_info;
use constant_pool::cp_info::*;
use constant_pool::read_constant_pool;
use constant_pool::ConstantPool;
use field;
use field::FieldInfo;
use method;
use std;
use std::convert::From;
use std::io::ErrorKind;
use std::io::Read;
use typed_arena::Arena;

#[derive(Debug)]
pub struct ClassFile<'a> {
    magic: u32,
    minor_version: u16,
    major_version: u16,
    constant_pool_count: u16,
    constant_pool: ConstantPool<'a>,
    access_flags: ClassAccessFlag,
    this_class: &'a str,
    super_class: Option<ClassRef<'a>>,
    interfaces_count: u16,
    interfaces: Vec<ClassRef<'a>>,
    fields_count: u16,
    fields: Vec<FieldInfo<'a>>,
    methods_count: u16,
    methods: Vec<method::MethodInfo<'a>>,
    attributes_count: u16,
    attributes: Vec<attribute::attribute_info>,
}

impl<'a> ClassFile<'a> {
    const CURRENT_VERSION: u16 = 52;

    fn get_constant_entry(&self, index: u16) -> &cp_info {
        self.constant_pool.get_entry(index)
    }

    pub fn get_string_entry(&self, index: u16) -> &str {
        self.constant_pool.get_string_entry(index)
    }

    pub fn has_super_class(&self) -> bool {
        self.super_class.is_some()
    }

    pub fn resolve_super_class(&mut self) -> &mut Option<ClassRef<'a>> {
        &mut self.super_class
    }

    pub fn get_access_flags(&self) -> ClassAccessFlag {
        self.access_flags
    }

    pub fn get_name(&self) -> &str {
        self.this_class
    }

    pub fn get_super_class(&self) -> &Option<ClassRef<'a>> {
        &self.super_class
    }

    pub fn new<'b>(
        input: &'b mut Read,
        string_allocator: &'a Arena<String>,
    ) -> Result<ClassFile<'a>, ClassLoadingError> {
        let magic = input.read_u32::<BigEndian>()?;
        let minor_version = input.read_u16::<BigEndian>()?;
        let major_version = input.read_u16::<BigEndian>()?;
        if major_version > ClassFile::CURRENT_VERSION {
            return Err(UnsupportedClassVersionError);
        }
        let constant_pool_count = input.read_u16::<BigEndian>()?;
        let constant_pool = read_constant_pool(input, constant_pool_count, string_allocator)?;
        let access_flags = ClassAccessFlag::from_bits(input.read_u16::<BigEndian>()?)
            .expect("Couldn't parse Class Access Flags");
        let this_class_index = input.read_u16::<BigEndian>()?;
        let this_class = {
            let this_class_data = constant_pool.get_entry(this_class_index);
            if let CONSTANT_Class_info { name_index } = this_class_data {
                constant_pool.get_string_entry(*name_index)
            } else {
                panic!(
                    "ClassFile#this_class pointed to non CONSTANT_Class_attribute: {:?}",
                    this_class_data
                )
            }
        };
        let super_class_index = input.read_u16::<BigEndian>()?;
        let super_class = if super_class_index == 0 {
            None
        } else {
            let super_class_data = constant_pool.get_entry(super_class_index);
            if let CONSTANT_Class_info { name_index } = super_class_data {
                let super_class_name = constant_pool.get_string_entry(*name_index);
                Some(Symbolic(super_class_name))
            } else {
                panic!(
                    "ClassFile#super_class didn't point to CONSTANT_Class_info, instead: {:?}",
                    super_class_data
                )
            }
        };
        let interfaces_count = input.read_u16::<BigEndian>()?;
        let interfaces = read_interfaces(input, interfaces_count)?;
        let interfaces = interfaces.iter().map(|i| {
            let class_info = constant_pool.get_entry(*i);
            let string_index = if let CONSTANT_Class_info { name_index } = class_info {
                name_index
            } else {
                panic!("ClassFile#interfaces index {} didn't contain CONSTANT_Class_info, instead: {:?}", i, class_info)
            };
            Symbolic(constant_pool.get_string_entry(*string_index))
        }).collect();
        let fields_count = input.read_u16::<BigEndian>()?;
        let fields = field::read_fields(input, fields_count, &constant_pool, this_class)?;
        let methods_count = input.read_u16::<BigEndian>()?;
        let methods = method::read_methods(input, methods_count, &constant_pool, this_class)?;
        let attributes_count = input.read_u16::<BigEndian>()?;
        let attributes = attribute::read_attributes(input, attributes_count, &constant_pool)?;
        Ok(ClassFile {
            magic,
            minor_version,
            major_version,
            constant_pool_count,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces_count,
            interfaces,
            fields_count,
            fields,
            methods_count,
            methods,
            attributes_count,
            attributes,
        })
    }
}

#[derive(Debug)]
pub enum ClassLoadingError {
    LinkageError,
    ClassFormatError(String),
    UnsupportedClassVersionError,
    NoClassDefFoundError,
    IncompatibleClassChangeError,
    ClassCircularityError,
}

impl From<zip::result::ZipError> for ClassLoadingError {
    fn from(error: zip::result::ZipError) -> Self {
        panic!("Error reading zip file")
    }
}

impl From<std::io::Error> for ClassLoadingError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == ErrorKind::UnexpectedEof {
            return ClassFormatError(String::from("Parsing reached end of Class File"));
        }
        panic!("Unknown error parsing class file: {}", error);
    }
}

impl From<cesu8::Cesu8DecodingError> for ClassLoadingError {
    fn from(error: cesu8::Cesu8DecodingError) -> Self {
        panic!("Error decoding Modified UTF8: {}", error)
    }
}

fn read_interfaces(input: &mut Read, length: u16) -> Result<Vec<u16>, ClassLoadingError> {
    let mut vector = Vec::with_capacity(length as usize);
    for _ in 0..length {
        vector.push(input.read_u16::<BigEndian>()?);
    }
    Ok(vector)
}
