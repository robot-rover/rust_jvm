use class_file::ClassFile;
use class_array::ClassArray;
use class::Class::*;
use std::cell::RefCell;
use std::clone::Clone;
use class::ClassRef::{Symbolic, Static};

#[derive(Debug)]
pub enum Class<'a> {
    File(ClassFile<'a>),
    Array(ClassArray<'a>)
}

#[derive(Debug)]
pub enum ClassRef<'a> {
    /// 0 -> name of the class
    Symbolic(&'a str),
    /// 0-> a reference to the actual class object in memory
    Static(&'a RefCell<Class<'a>>),
}

impl<'a> Clone for ClassRef<'a> {
    fn clone(&self) -> Self {
        match self {
            Symbolic(index) => Symbolic(index.clone()),
            Static(class_ref) => Static(class_ref.clone())
        }
    }
}

impl<'a> Class<'a> {
    pub fn get_name(&self) -> &str {
        match self {
            File(class) => class.get_name(),
            Array(class) => class.get_name()
        }
    }

    pub fn get_access_flags(&self) -> ClassAccessFlag {
        match self {
            File(class) => class.get_access_flags(),
            Array(class) => class.get_access_flags()
        }
    }
}

/// <https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-4.html#jvms-4.1-200-E.1>
bitflags! {
    pub struct ClassAccessFlag: u16 {
        const ACC_PUBLIC      = 0x0001;
        const ACC_FINAL       = 0x0010;
        const ACC_SUPER       = 0x0020;
        const ACC_INTERFACE   = 0x0200;
        const ACC_ABSTRACT    = 0x0400;
        const ACC_SYNTHETIC   = 0x1000;
        const ACC_ANNOTATION  = 0x2000;
        const ACC_ENUM        = 0x4000;
    }
}