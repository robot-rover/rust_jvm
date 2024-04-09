use class::Class::*;
use class_array::ClassArray;
use class_file::ClassFile;
use std::cell::RefCell;
use lazy::LazyResolve;
use class::ClassRef::{Static, Symbolic};

#[derive(Debug)]
pub enum Class<'a> {
    File(ClassFile<'a>),
    Array(ClassArray<'a>),
}

#[derive(Debug)]
pub enum ClassRef<'a> {
    Symbolic(&'a str),
    Static(&'a RefCell<Class<'a>>)
}

impl<'a> ClassRef<'a> {
    pub fn get(&self) -> &'a RefCell<Class<'a>> {
        if let Static(class_ref) = self {
            class_ref
        } else {
            panic!("Accessed ClassRef that isn't resolved")
        }
    }

    pub fn resolve<'b, 'c, T>(&'b mut self, resolver: &'c mut T) -> &'a RefCell<Class<'a>>
        where T: LazyResolve<'a, RefCell<Class<'a>>> {
        let class_name = match self {
            Symbolic(class_name) => *class_name,
            Static(class_ref) => return class_ref
        };

        *self = Static(resolver.resolve(class_name));
        self.get()
    }
}

impl<'a> Class<'a> {
    pub fn get_name(&self) -> &str {
        match self {
            File(class) => class.get_name(),
            Array(class) => class.get_name(),
        }
    }

    pub fn get_access_flags(&self) -> ClassAccessFlag {
        match self {
            File(class) => class.get_access_flags(),
            Array(class) => class.get_access_flags(),
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
