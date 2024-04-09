use class::Class;
use std::cell::RefCell;

pub trait LazyResolve<'a, T> {
    fn resolve(&mut self, name: &'a str) -> &'a T;
}

//pub trait ClassResolve<'a> {
//    fn resolve(&mut self, name: &'a str) -> &'a RefCell<Class<'a>>;
//}