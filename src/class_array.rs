use class::ClassAccessFlag;
use class::ClassRef;
use std::cell::RefCell;
use class::Class;
use field::FieldRef;
use field::FieldDescriptor::Reference;
use field::FieldDescriptor;

#[derive(Debug)]
pub struct ClassArray<'a> {
    dimensions: u8,
    component_type: FieldDescriptor<'a>,
    access_flags: ClassAccessFlag,
    name: String
}

impl<'a> ClassArray<'a> {
    pub fn new(dimensions: u8, component_type: FieldDescriptor<'a>, class_name: &str) -> ClassArray<'a> {
        ClassArray {
            dimensions,
            component_type,
            access_flags: ClassAccessFlag::ACC_PUBLIC,
            name: class_name.to_owned()
        }
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get_access_flags(&self) -> ClassAccessFlag {
        self.access_flags
    }
}