use std::{cmp::Ordering, fmt, sync::Arc};

use nanbox::{NanBox, NanBoxable};

use crate::array2::Array;

pub struct Value(NanBox);

fn _value_is_small() {
    let _: u64 = unsafe { std::mem::transmute(Value::from(0.0)) };
}

type PartialRef = *mut Partial;
type ArrayRef = *mut Array;
const NUM_TAG: u8 = 0;
const CHAR_TAG: u8 = 1;
const FUNCTION_TAG: u8 = 2;
const PARTIAL_TAG: u8 = 3;
const ARRAY_TAG: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RawType {
    Number,
    Char,
    Function,
    Partial,
    Array,
}

static RAW_TYPES: [RawType; 5] = {
    let mut types = [RawType::Number; 5];
    types[NUM_TAG as usize] = RawType::Number;
    types[CHAR_TAG as usize] = RawType::Char;
    types[FUNCTION_TAG as usize] = RawType::Function;
    types[PARTIAL_TAG as usize] = RawType::Partial;
    types[ARRAY_TAG as usize] = RawType::Array;
    types
};

impl Value {
    pub fn raw_ty(&self) -> RawType {
        RAW_TYPES[self.0.tag() as usize]
    }
    pub fn is_num(&self) -> bool {
        self.0.tag() == NUM_TAG as u32
    }
    pub fn is_char(&self) -> bool {
        self.0.tag() == CHAR_TAG as u32
    }
    pub fn is_function(&self) -> bool {
        self.0.tag() == FUNCTION_TAG as u32
    }
    pub fn is_partial(&self) -> bool {
        self.0.tag() == PARTIAL_TAG as u32
    }
    pub fn is_array(&self) -> bool {
        self.0.tag() == ARRAY_TAG as u32
    }
    pub fn number(&self) -> f64 {
        assert!(self.is_num());
        unsafe { self.0.unpack::<f64>() }
    }
    pub fn char(&self) -> char {
        assert!(self.is_char());
        unsafe { self.0.unpack::<char>() }
    }
    pub fn function(&self) -> Function {
        assert!(self.is_function());
        unsafe { self.0.unpack::<Function>() }
    }
    pub fn partial(&self) -> &Partial {
        assert!(self.is_partial());
        unsafe { &*self.0.unpack::<PartialRef>() }
    }
    pub fn partial_mut(&mut self) -> &mut Partial {
        assert!(self.is_partial());
        unsafe { &mut *self.0.unpack::<PartialRef>() }
    }
    pub fn array(&self) -> &Array {
        assert!(self.is_array());
        unsafe { &*self.0.unpack::<ArrayRef>() }
    }
    pub fn array_mut(&mut self) -> &mut Array {
        assert!(self.is_array());
        unsafe { &mut *self.0.unpack::<ArrayRef>() }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Function {
    pub(crate) start: u32,
    pub(crate) params: u16,
}

impl Default for Function {
    fn default() -> Self {
        Self::nil()
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            write!(f, "nil")
        } else {
            write!(f, "fn({} {})", self.start, self.params)
        }
    }
}

impl Function {
    #[inline]
    pub const fn nil() -> Self {
        Self {
            start: 0,
            params: 1,
        }
    }
    #[inline]
    pub const fn is_nil(&self) -> bool {
        self.start == 0
    }
}

impl NanBoxable for Function {
    unsafe fn from_nan_box(n: NanBox) -> Self {
        let [a, b, c, d, e, f]: [u8; 6] = NanBoxable::from_nan_box(n);
        Self {
            start: u32::from_le_bytes([a, b, c, d]),
            params: u16::from_le_bytes([e, f]),
        }
    }
    fn into_nan_box(self) -> NanBox {
        let [a, b, c, d] = self.start.to_le_bytes();
        let [e, f] = self.params.to_le_bytes();
        NanBoxable::into_nan_box([a, b, c, d, e, f])
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Partial {
    pub(crate) function: Function,
    pub(crate) args: Arc<[Value]>,
}

impl fmt::Debug for Partial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for Partial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "fn({} {}/{})",
            self.function.start,
            self.args.len(),
            self.function.params
        )
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        match self.raw_ty() {
            RawType::Partial => unsafe {
                drop(Box::from_raw(self.0.unpack::<PartialRef>()));
            },
            RawType::Array => unsafe {
                drop(Box::from_raw(self.0.unpack::<ArrayRef>()));
            },
            _ => {}
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self.raw_ty() {
            RawType::Partial => Self(unsafe {
                NanBox::new::<PartialRef>(
                    PARTIAL_TAG,
                    Box::into_raw(Box::new(self.partial().clone())),
                )
            }),
            RawType::Array => Self(unsafe {
                NanBox::new::<ArrayRef>(ARRAY_TAG, Box::into_raw(Box::new(self.array().clone())))
            }),
            _ => Self(self.0),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self.raw_ty(), other.raw_ty()) {
            (RawType::Number, RawType::Number) => {
                let a = self.number();
                let b = other.number();
                a == b || a.is_nan() && b.is_nan()
            }
            (RawType::Char, RawType::Char) => self.char() == other.char(),
            (RawType::Function, RawType::Function) => self.function() == other.function(),
            (RawType::Partial, RawType::Partial) => self.partial() == other.partial(),
            (RawType::Array, RawType::Array) => self.array() == other.array(),
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.raw_ty(), other.raw_ty()) {
            (RawType::Number, RawType::Number) => {
                let a = self.number();
                let b = other.number();
                a.partial_cmp(&b)
                    .unwrap_or_else(|| a.is_nan().cmp(&b.is_nan()))
            }
            (RawType::Char, RawType::Char) => self.char().cmp(&other.char()),
            (RawType::Function, RawType::Function) => self.function().cmp(&other.function()),
            (RawType::Partial, RawType::Partial) => self.partial().cmp(other.partial()),
            (RawType::Array, RawType::Array) => self.array().cmp(other.array()),
            (a, b) => a.cmp(&b),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.raw_ty() {
            RawType::Number => write!(f, "{:?}", self.number()),
            RawType::Char => write!(f, "{:?}", self.char()),
            RawType::Function => write!(f, "{:?}", self.function()),
            RawType::Partial => write!(f, "{:?}", self.partial()),
            RawType::Array => write!(f, "{:?}", self.array()),
        }
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Self(unsafe { NanBox::new(NUM_TAG, n) })
    }
}

impl From<char> for Value {
    fn from(c: char) -> Self {
        Self(unsafe { NanBox::new(CHAR_TAG, c) })
    }
}

impl From<Function> for Value {
    fn from(f: Function) -> Self {
        Self(unsafe { NanBox::new(FUNCTION_TAG, f) })
    }
}

impl From<Partial> for Value {
    fn from(p: Partial) -> Self {
        Self(unsafe { NanBox::new::<PartialRef>(PARTIAL_TAG, Box::into_raw(Box::new(p))) })
    }
}

impl From<Array> for Value {
    fn from(a: Array) -> Self {
        Self(unsafe { NanBox::new::<ArrayRef>(ARRAY_TAG, Box::into_raw(Box::new(a))) })
    }
}
