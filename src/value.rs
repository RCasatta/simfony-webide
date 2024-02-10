use std::fmt;
use std::sync::Arc;

use hex_conservative::DisplayHex;
use simplicity::dag::{Dag, DagLike, NoSharing};
use simplicity::types::Final;
use simplicity::Value;

/// Immutable sequence of bits whose length is a power of two.
///
/// The sequence can be split in half to produce (pointers) to the front and to the rear.
///
/// All methods assume big Endian (of the implied byte sequence).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bits {
    bits: Arc<Vec<bool>>,
    start: usize,
    len: usize,
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0b")?;

        for i in self.start..self.start + self.len {
            let bit = if self.bits[i] { '1' } else { '0' };
            write!(f, "{}", bit)?;
        }

        Ok(())
    }
}

impl Bits {
    pub fn from_bits(bits: Vec<bool>) -> Self {
        assert!(
            bits.len().is_power_of_two(),
            "Length of bit sequence must be a power of two"
        );
        Self {
            len: bits.len(),
            bits: Arc::new(bits),
            start: 0,
        }
    }

    pub fn from_bit(bit: bool) -> Self {
        Self {
            len: 1,
            bits: Arc::new(vec![bit]),
            start: 0,
        }
    }

    pub fn from_byte(byte: u8) -> Self {
        Self {
            bits: Arc::new((0..8).map(|i| byte & (1 << (7 - i)) != 0).collect()),
            start: 0,
            len: 8,
        }
    }

    pub fn split(&self) -> Option<(Self, Self)> {
        debug_assert!(self.len.is_power_of_two());
        if self.len == 1 {
            None
        } else {
            let left = Self {
                bits: self.bits.clone(),
                start: self.start,
                len: self.len / 2,
            };
            let right = Self {
                bits: self.bits.clone(),
                start: self.start + self.len / 2,
                len: self.len / 2,
            };
            Some((left, right))
        }
    }

    pub fn get_bit(&self) -> Option<bool> {
        if self.len == 1 {
            Some(self.bits[self.start])
        } else {
            None
        }
    }

    pub fn bit_length(&self) -> usize {
        self.len
    }

    pub fn iter_bits(&self) -> impl ExactSizeIterator<Item = bool> + '_ {
        self.bits.iter().copied()
    }
}

/// # Error
///
/// Input value is a left or right value that wraps something other than unit.
///
/// Input value is a product of unit.
fn do_each_bit_strict<F>(value: &Value, mut f: F) -> Result<(), String>
where
    F: FnMut(bool),
{
    for data in value.pre_order_iter::<NoSharing>() {
        match data {
            Value::Unit => {}
            Value::SumL(left) => {
                if let Value::Unit = left.as_ref() {
                    f(false);
                } else {
                    return Err(format!("Illegal left value: {data}"));
                }
            }
            Value::SumR(right) => {
                if let Value::Unit = right.as_ref() {
                    f(true);
                } else {
                    return Err(format!("Illegal right value: {data}"));
                }
            }
            Value::Prod(left, right) => {
                if let (Value::Unit, Value::Unit) = (left.as_ref(), right.as_ref()) {
                    return Err(format!("Illegal product value: {data}"));
                }
            }
        }
    }

    Ok(())
}

impl<'a> TryFrom<&'a Value> for Bits {
    type Error = String;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        if !value.len().is_power_of_two() {
            return Err("Length of bit sequence must a be a power of 2".to_string());
        }

        let mut bits = Vec::with_capacity(value.len());
        let add_bit = |bit: bool| {
            bits.push(bit);
        };

        do_each_bit_strict(value, add_bit)?;

        Ok(Bits {
            len: bits.len(),
            bits: Arc::new(bits),
            start: 0,
        })
    }
}

/// Immutable sequence of bytes whose length is a power of two.
///
/// The sequence can be split in half to produce (pointers) to the front and to the rear.
///
/// All methods assume big Endian.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bytes {
    bytes: Arc<Vec<u8>>,
    start: usize,
    len: usize,
}

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "0x{}",
            DisplayHex::as_hex(&self.bytes[self.start..self.start + self.len])
        )
    }
}

impl Bytes {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        assert!(
            bytes.len().is_power_of_two(),
            "Length of byte sequence must be a power of 2"
        );
        Self {
            len: bytes.len(),
            bytes: Arc::new(bytes),
            start: 0,
        }
    }

    pub fn from_slice<A: AsRef<[u8]>>(bytes: A) -> Self {
        assert!(
            bytes.as_ref().len().is_power_of_two(),
            "Length of byte sequence must be a power of 2"
        );
        Self::from_bytes(bytes.as_ref().to_vec())
    }

    pub fn byte_length(&self) -> usize {
        self.len
    }

    pub fn iter_bytes(&self) -> impl ExactSizeIterator<Item = u8> + '_ {
        self.bytes.iter().copied()
    }

    pub fn iter_bits(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter_bytes()
            .flat_map(|byte| (0..8).map(move |i| byte & (1 << (7 - i)) != 0))
    }

    pub fn split(&self) -> Result<(Self, Self), (Bits, Bits)> {
        debug_assert!(self.len.is_power_of_two());
        if self.len == 1 {
            let byte = self.bytes[self.start];
            let bits = Bits::from_byte(byte);
            Err(bits.split().unwrap())
        } else {
            let left = Self {
                bytes: self.bytes.clone(),
                start: self.start,
                len: self.len / 2,
            };
            let right = Self {
                bytes: self.bytes.clone(),
                start: self.start + self.len / 2,
                len: self.len / 2,
            };
            Ok((left, right))
        }
    }
}

impl<'a> TryFrom<&'a Value> for Bytes {
    type Error = String;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        if !value.len().is_power_of_two() {
            return Err("Length of byte sequence must be a power of 2".to_string());
        }
        if value.len() % 8 != 0 {
            return Err("Length of bit sequence must be divisible by 8".to_string());
        }

        let mut bytes = Vec::with_capacity(value.len());
        let mut unfinished_byte = Vec::with_capacity(8);

        let add_bit = |bit: bool| {
            if unfinished_byte.len() < 8 {
                bytes.push(
                    unfinished_byte
                        .iter()
                        .fold(0, |acc, &b| acc * 2 + u8::from(b)),
                );
            } else {
                unfinished_byte.push(bit);
            }
        };

        do_each_bit_strict(value, add_bit)?;
        debug_assert!(unfinished_byte.is_empty());

        let bytes = value.try_to_bytes()?;
        Ok(Bytes {
            len: bytes.len(),
            bytes: Arc::new(bytes),
            start: 0,
        })
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum ExtValue {
    Unit,
    Left(Arc<Self>),
    Right(Arc<Self>),
    Product(Arc<Self>, Arc<Self>),
    Bits(Bits),
    Bytes(Bytes),
}

impl fmt::Display for ExtValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for data in self.verbose_pre_order_iter::<NoSharing>() {
            match data.node {
                ExtValue::Unit => f.write_str("●")?,
                ExtValue::Left(..) => {
                    match data.n_children_yielded {
                        0 => f.write_str("L")?,
                        _ => continue,
                    };
                }
                ExtValue::Right(..) => match data.n_children_yielded {
                    0 => {
                        f.write_str("R")?;
                    }
                    _ => continue,
                },
                ExtValue::Product(..) => match data.n_children_yielded {
                    0 => f.write_str("(")?,
                    1 => f.write_str(", ")?,
                    _ => f.write_str(")")?,
                },
                ExtValue::Bits(bits) => write!(f, "{}", bits)?,
                ExtValue::Bytes(bytes) => write!(f, "{}", bytes)?,
            }
        }

        Ok(())
    }
}

impl fmt::Debug for ExtValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl ExtValue {
    pub fn unit() -> Arc<Self> {
        Arc::new(Self::Unit)
    }

    pub fn left(left: Arc<Self>) -> Arc<Self> {
        Arc::new(Self::Left(left))
    }

    pub fn right(right: Arc<Self>) -> Arc<Self> {
        Arc::new(Self::Right(right))
    }

    pub fn product(left: Arc<Self>, right: Arc<Self>) -> Arc<Self> {
        Arc::new(Self::Product(left, right))
    }

    pub fn bits(bits: Bits) -> Arc<Self> {
        Arc::new(Self::Bits(bits))
    }

    pub fn bytes(bytes: Bytes) -> Arc<Self> {
        Arc::new(Self::Bytes(bytes))
    }

    pub fn split_left(&self) -> Option<Arc<Self>> {
        match self {
            Self::Left(left) => Some(left.clone()),
            Self::Bits(bits) => bits.get_bit().and_then(|b| (!b).then(ExtValue::unit)),
            _ => None,
        }
    }

    pub fn split_right(&self) -> Option<Arc<Self>> {
        match self {
            Self::Right(right) => Some(right.clone()),
            Self::Bits(bits) => bits.get_bit().and_then(|b| b.then(ExtValue::unit)),
            _ => None,
        }
    }

    pub fn split_product(&self) -> Option<(Arc<Self>, Arc<Self>)> {
        match self {
            Self::Product(left, right) => Some((left.clone(), right.clone())),
            Self::Bits(bits) => bits
                .split()
                .map(|(left, right)| (Self::bits(left), Self::bits(right))),
            Self::Bytes(bytes) => match bytes.split() {
                Ok((left, right)) => Some((Self::bytes(left), Self::bytes(right))),
                Err((left, right)) => Some((Self::bits(left), Self::bits(right))),
            },
            _ => None,
        }
    }

    pub fn bit_width(&self) -> usize {
        self.pre_order_iter::<NoSharing>()
            .map(|inner| match inner {
                ExtValue::Unit | ExtValue::Product(..) => 0,
                ExtValue::Left(..) | ExtValue::Right(..) => 1,
                ExtValue::Bits(bits) => bits.bit_length(),
                ExtValue::Bytes(bytes) => bytes.byte_length() * 8,
            })
            .sum()
    }

    pub fn iter_bits(&self) -> impl Iterator<Item = bool> + '_ {
        self.pre_order_iter::<NoSharing>()
            .flat_map(|inner| match inner {
                ExtValue::Unit | ExtValue::Product(..) => {
                    Box::new(std::iter::empty()) as Box<dyn Iterator<Item = bool>>
                }
                ExtValue::Left(..) => Box::new(std::iter::once(false)),
                ExtValue::Right(..) => Box::new(std::iter::once(true)),
                ExtValue::Bits(bits) => Box::new(bits.iter_bits()),
                ExtValue::Bytes(bytes) => Box::new(bytes.iter_bits()),
            })
    }

    // FIXME: Take &Final
    // Requires split_{sum,product} method of Final that returns references
    pub fn from_bits<I: Iterator<Item = bool>>(ty: Arc<Final>, it: &mut I) -> Arc<Self> {
        enum Task {
            ReadType(Arc<Final>),
            MakeLeft,
            MakeRight,
            MakeProduct,
        }

        let mut task_stack = vec![Task::ReadType(ty)];
        let mut result_stack = vec![];

        while let Some(task) = task_stack.pop() {
            match task {
                Task::ReadType(ty) => {
                    if ty.is_unit() {
                        result_stack.push(ExtValue::unit());
                    } else if let Some((left, right)) = ty.split_sum() {
                        if !it.next().expect("enough bits") {
                            task_stack.push(Task::MakeLeft);
                            task_stack.push(Task::ReadType(left));
                        } else {
                            task_stack.push(Task::MakeRight);
                            task_stack.push(Task::ReadType(right));
                        }
                    } else if let Some((left, right)) = ty.split_product() {
                        task_stack.push(Task::MakeProduct);
                        task_stack.push(Task::ReadType(right));
                        task_stack.push(Task::ReadType(left));
                    }
                }
                Task::MakeLeft => {
                    let inner = result_stack.pop().unwrap();
                    result_stack.push(ExtValue::left(inner));
                }
                Task::MakeRight => {
                    let inner = result_stack.pop().unwrap();
                    result_stack.push(ExtValue::right(inner));
                }
                Task::MakeProduct => {
                    let right = result_stack.pop().unwrap();
                    let left = result_stack.pop().unwrap();
                    result_stack.push(ExtValue::product(left, right));
                }
            }
        }

        debug_assert!(result_stack.len() == 1);
        result_stack.pop().unwrap()
    }
}

impl<'a> DagLike for &'a ExtValue {
    type Node = Self;

    fn data(&self) -> &Self::Node {
        self
    }

    fn as_dag_node(&self) -> Dag<Self> {
        match self {
            ExtValue::Unit | ExtValue::Bits(..) | ExtValue::Bytes(..) => Dag::Nullary,
            ExtValue::Left(child) | ExtValue::Right(child) => Dag::Unary(child),
            ExtValue::Product(left, right) => Dag::Binary(left, right),
        }
    }
}

fn bits_to_byte<A: AsRef<[bool]>>(bits: A) -> u8 {
    assert_eq!(
        bits.as_ref().len(),
        8,
        "Length of bit sequence must be exactly 8"
    );

    let mut byte: u8 = 0;

    for bit in bits.as_ref().iter().copied() {
        byte = byte << 1 | if bit { 1 } else { 0 };
    }

    byte
}

// I would like to implement for Arc<ExtValue> but I can't
impl<'a> From<&'a Value> for ExtValue {
    fn from(value: &'a Value) -> Self {
        enum Item {
            Value(ExtValue),
            Bits(Vec<bool>),
            Bytes(Vec<u8>),
        }

        impl Item {
            fn into_extvalue(self) -> ExtValue {
                match self {
                    Item::Value(left) => left,
                    Item::Bits(left) => ExtValue::Bits(Bits::from_bits(left)),
                    Item::Bytes(left) => ExtValue::Bytes(Bytes::from_bytes(left)),
                }
            }
        }

        if let Ok(bytes) = Bytes::try_from(value) {
            ExtValue::Bytes(bytes)
        } else if let Ok(bits) = Bits::try_from(value) {
            ExtValue::Bits(bits)
        } else {
            let mut stack = vec![];
            for data in value.post_order_iter::<NoSharing>() {
                match data.node {
                    Value::Unit => stack.push(Item::Value(ExtValue::Unit)),
                    Value::SumL(..) => match stack.pop().unwrap() {
                        Item::Value(ExtValue::Unit) => {
                            stack.push(Item::Bits(vec![false]));
                        }
                        top => {
                            let child = Arc::new(top.into_extvalue());
                            stack.push(Item::Value(ExtValue::Left(child)));
                        }
                    },
                    Value::SumR(..) => match stack.pop().unwrap() {
                        Item::Value(ExtValue::Unit) => {
                            stack.push(Item::Bits(vec![true]));
                        }
                        top => {
                            let child = Arc::new(top.into_extvalue());
                            stack.push(Item::Value(ExtValue::Right(child)));
                        }
                    },
                    Value::Prod(..) => match (stack.pop().unwrap(), stack.pop().unwrap()) {
                        (Item::Bits(right), Item::Bits(mut left)) => {
                            debug_assert!(right.len() == left.len()); // FIXME: Doesn't always hold
                            debug_assert!(right.len() == 1 || right.len() == 2 || right.len() == 4);
                            left.extend(right);
                            if left.len() == 8 {
                                stack.push(Item::Bytes(vec![bits_to_byte(left)]));
                            } else {
                                stack.push(Item::Bits(left));
                            }
                        }
                        (Item::Bytes(right), Item::Bytes(mut left)) => {
                            debug_assert!(right.len() == left.len()); // FIXME: Doesn't always hold
                            debug_assert!(!right.is_empty());
                            left.extend(right);
                            stack.push(Item::Bytes(left));
                        }
                        (right, left) => {
                            let left = Arc::new(left.into_extvalue());
                            let right = Arc::new(right.into_extvalue());
                            stack.push(Item::Value(ExtValue::Product(left, right)));
                        }
                    },
                }
            }

            debug_assert!(stack.len() == 1);
            stack.pop().unwrap().into_extvalue()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simplicity::Cmr;

    #[test]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn split_bits() {
        let bits = Bits::from_byte(0b01101111);
        assert_eq!("0b01101111", bits.to_string().as_str());

        let (a, b) = bits.split().unwrap();
        assert_eq!("0b0110", a.to_string().as_str());
        assert_eq!("0b1111", b.to_string().as_str());

        let (c, d) = a.split().unwrap();
        assert_eq!("0b01", c.to_string().as_str());
        assert_eq!("0b10", d.to_string().as_str());

        let (e, f) = c.split().unwrap();
        assert_eq!("0b0", e.to_string().as_str());
        assert_eq!("0b1", f.to_string().as_str());

        assert!(e.split().is_none());
    }

    #[test]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn get_bit() {
        assert_eq!(Some(false), Bits::from_bit(false).get_bit());
        assert_eq!(Some(true), Bits::from_bit(true).get_bit());
        assert_eq!(None, Bits::from_bits(vec![false, false]).get_bit());
    }

    #[test]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn split_bytes() {
        let bytes = Bytes::from_bytes(vec![0b01101111, 0xff, 0xff, 0xff]);
        assert_eq!("0x6fffffff", bytes.to_string().as_str());

        let (a, b) = bytes.split().unwrap();
        assert_eq!("0x6fff", a.to_string().as_str());
        assert_eq!("0xffff", b.to_string().as_str());

        let (c, d) = a.split().unwrap();
        assert_eq!("0x6f", c.to_string().as_str());
        assert_eq!("0xff", d.to_string().as_str());

        let (e, f) = c.split().unwrap_err();
        assert_eq!("0b0110", e.to_string().as_str());
        assert_eq!("0b1111", f.to_string().as_str());
    }

    #[test]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn display_value() {
        let value = ExtValue::product(
            ExtValue::left(ExtValue::right(ExtValue::unit())),
            ExtValue::product(
                ExtValue::bits(Bits::from_byte(0b01101111)),
                ExtValue::bytes(Bytes::from_bytes(vec![0xde, 0xad, 0xbe, 0xef])),
            ),
        );
        assert_eq!(
            "(LR●, (0b01101111, 0xdeadbeef))",
            value.to_string().as_str()
        );
    }

    #[test]
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn extvalue_from_value() {
        let output_input = vec![
            (ExtValue::unit(), Value::unit()),
            (
                ExtValue::bits(Bits::from_bit(false)),
                Value::sum_l(Value::unit()),
            ),
            (
                ExtValue::bits(Bits::from_bit(true)),
                Value::sum_r(Value::unit()),
            ),
            (
                ExtValue::left(ExtValue::bits(Bits::from_bit(true))),
                Value::sum_l(Value::sum_r(Value::unit())),
            ),
            (
                ExtValue::product(ExtValue::unit(), ExtValue::unit()),
                Value::prod(Value::unit(), Value::unit()),
            ),
            (
                ExtValue::bytes(Bytes::from_slice(Cmr::unit())),
                Value::u256_from_slice(Cmr::unit().as_ref()),
            ),
            (
                ExtValue::bytes(Bytes::from_bytes(vec![0b01010101])),
                Value::u8(0b01010101),
            ),
            (
                ExtValue::bytes(Bytes::from_bytes(vec![0xab, 0xcd])),
                Value::prod(Value::u8(0xab), Value::u8(0xcd)),
            ),
        ];

        for (expected_output, input) in output_input {
            assert_eq!(expected_output.as_ref(), &ExtValue::from(input.as_ref()));
        }
    }
}
