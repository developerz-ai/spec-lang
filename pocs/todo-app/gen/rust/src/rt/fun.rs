//! A function value: a lambda with the locals it captured, called with the machine.

use super::R;
use crate::machine::Machine;
use std::cmp::Ordering;
use std::rc::Rc;

/// A lambda's code: the machine and the tuple of arguments in, the result out.
type Code<A, B> = dyn Fn(&mut Machine, A) -> R<B>;

/// `A` is the tuple of parameters, `B` the result. Equal only to itself (`==` on functions is never lowered).
pub struct Fun<A, B>(Rc<Code<A, B>>);

impl<A, B> Fun<A, B> {
    pub fn new(f: impl Fn(&mut Machine, A) -> R<B> + 'static) -> Self {
        Fun(Rc::new(f))
    }

    pub fn call(&self, m: &mut Machine, args: A) -> R<B> {
        (self.0)(m, args)
    }

    fn addr(&self) -> usize {
        Rc::as_ptr(&self.0).cast::<()>() as usize
    }
}

impl<A, B> Clone for Fun<A, B> {
    fn clone(&self) -> Self {
        Fun(Rc::clone(&self.0))
    }
}

impl<A, B> std::fmt::Debug for Fun<A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<function>")
    }
}

impl<A, B> PartialEq for Fun<A, B> {
    fn eq(&self, other: &Self) -> bool {
        self.addr() == other.addr()
    }
}

impl<A, B> Eq for Fun<A, B> {}

impl<A, B> PartialOrd for Fun<A, B> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A, B> Ord for Fun<A, B> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.addr().cmp(&other.addr())
    }
}
