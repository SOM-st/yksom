// Copyright (c) 2019 King's College London created by the Software Development Team
// <http://soft-dev.org/>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0>, or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, or the UPL-1.0 license <http://opensource.org/licenses/UPL>
// at your option. This file may not be copied, modified, or distributed except according to those
// terms.

#![allow(clippy::new_ret_no_self)]

//! This file contains the core SOM objects. Note that there is a fundamental constraint that
//! *must* be obeyed by the programmer at all times: upon their creation, instances of the `Obj`
//! trait must immediately be passed to `Val::from_obj`. In other words this is safe:
//!
//! ```rust,ignore
//! let x = Val::from_obj(vm, String_{ s: "a".to_owned() });
//! dbg!(x.tobj().as_str());
//! ```
//!
//! but this leads to undefined behaviour:
//!
//! ```rust,ignore
//! let x = String_{ s: "a".to_owned() };
//! dbg!(x.tobj().as_str());
//! ```
//!
//! The reason for this is that methods on `Obj`s can call `Val::restore` which converts an `Obj`
//! reference back into a `Val`.
//!
//! Although this constraint is not enforced through the type system, it is not hard to obey: as
//! soon as you create an `Obj` instance, pass it to `Val::from_obj`.

use std::{cell::UnsafeCell, collections::HashMap, convert::TryFrom, fmt::Debug, path::PathBuf};

use abgc::{self, Gc};
use abgc_derive::GcLayout;
use natrob::narrowable_abgc;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive};

use crate::{
    compiler::{
        cobjects,
        instrs::{Instr, Primitive},
    },
    vm::{
        val::{NotUnboxable, Val, ValResult},
        vm::{Closure, VMError, VM},
    },
};

/// The SOM type of objects.
///
/// It might seem that we should use `TypeId` for this, but that requires types to have a 'static
/// lifetime, which is an annoying restriction.
#[derive(Debug, PartialEq)]
pub enum ObjType {
    ArbInt,
    Block,
    Class,
    Method,
    Inst,
    Int,
    String_,
}

/// The main SOM Object trait. Notice that code should almost never call these functions directly:
/// you should instead call the equivalent function in the `Val` struct.
#[narrowable_abgc(ThinObj)]
pub trait Obj: Debug + abgc::GcLayout {
    /// What `ObjType` does this `Val` represent?
    fn dyn_objtype(&self) -> ObjType;
    /// What class is this object an instance of?
    fn get_class(&self, vm: &VM) -> Val;

    /// Convert this object to a `Val` that represents a SOM string.
    fn to_strval(&self, _: &VM) -> ValResult {
        unimplemented!();
    }

    /// Produce a new `Val` which adds `other` to this.
    fn add(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Produce a new `Val` which subtracts `other` from this.
    fn sub(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Produce a new `Val` which multiplies `other` to this.
    fn mul(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Produce a new `Val` which divides `other` from this.
    fn div(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Produce a new `Val` which shifts `self` `other` bits to the left.
    fn shl(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Does this `Val` equal `other`?
    fn equals(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Does this `Val` not equal `other`?
    fn not_equals(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Is this `Val` greater than `other`?
    fn greater_than(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Is this `Val` greater than or equal to `other`?
    fn greater_than_equals(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Is this `Val` less than `other`?
    fn less_than(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }

    /// Is this `Val` less than or equal to `other`?
    fn less_than_equals(&self, _: &VM, _: Val) -> ValResult {
        unimplemented!();
    }
}

pub trait StaticObjType {
    /// Return this trait type's static `ObjType`
    fn static_objtype() -> ObjType;
}

#[derive(Debug, GcLayout)]
pub struct Block {
    // Does this Block represent Block, Block2, or Block3?
    pub blockn_cls: Val,
    pub blockinfo_cls: Val,
    pub blockinfo_off: usize,
    pub parent_closure: Gc<Closure>,
}

impl Obj for Block {
    fn dyn_objtype(&self) -> ObjType {
        ObjType::Block
    }

    fn get_class(&self, _: &VM) -> Val {
        self.blockn_cls.clone()
    }
}

impl NotUnboxable for Block {}

impl StaticObjType for Block {
    fn static_objtype() -> ObjType {
        ObjType::Block
    }
}

impl Block {
    pub fn new(
        vm: &VM,
        blockinfo_cls: Val,
        blockinfo_off: usize,
        parent_closure: Gc<Closure>,
        num_params: usize,
    ) -> Val {
        let blockn_cls = match num_params {
            0 => vm.block_cls.clone(),
            1 => vm.block2_cls.clone(),
            2 => vm.block3_cls.clone(),
            _ => unimplemented!(),
        };
        Val::from_obj(
            vm,
            Block {
                blockn_cls,
                blockinfo_cls,
                blockinfo_off,
                parent_closure,
            },
        )
    }
}

#[derive(Debug, GcLayout)]
pub struct Class {
    pub name: Val,
    pub path: PathBuf,
    pub supercls: Option<Val>,
    pub num_inst_vars: usize,
    pub methods: HashMap<String, Method>,
    pub blockinfos: Vec<BlockInfo>,
    pub instrs: Vec<Instr>,
    pub consts: Vec<Val>,
    pub sends: Vec<(String, usize)>,
}

/// Minimal information about a SOM block.
#[derive(Debug)]
pub struct BlockInfo {
    pub bytecode_off: usize,
    pub bytecode_end: usize,
    pub num_params: usize,
    pub num_vars: usize,
}

impl Obj for Class {
    fn dyn_objtype(&self) -> ObjType {
        ObjType::Class
    }

    fn get_class(&self, vm: &VM) -> Val {
        vm.cls_cls.clone()
    }
}

impl NotUnboxable for Class {}

impl StaticObjType for Class {
    fn static_objtype() -> ObjType {
        ObjType::Class
    }
}

impl Class {
    pub fn from_ccls(vm: &VM, ccls: cobjects::Class) -> ValResult {
        let supercls = match ccls.supercls {
            Some(ref x) => match x.as_str() {
                "Block" => Some(vm.block_cls.clone()),
                "Boolean" => Some(vm.bool_cls.clone()),
                "nil" => None,
                _ => unimplemented!(),
            },
            None => Some(vm.obj_cls.clone()),
        };

        let mut inst_vars = Vec::with_capacity(ccls.num_inst_vars);
        inst_vars.resize(ccls.num_inst_vars, Val::illegal());

        let mut methods = HashMap::with_capacity(ccls.methods.len());
        for cmeth in ccls.methods.into_iter() {
            let body = match cmeth.body {
                cobjects::MethodBody::Primitive(p) => MethodBody::Primitive(p),
                cobjects::MethodBody::User {
                    num_vars,
                    bytecode_off,
                } => MethodBody::User {
                    num_vars,
                    bytecode_off,
                },
            };
            let meth = Method {
                name: cmeth.name.clone(),
                body,
            };
            methods.insert(cmeth.name, meth);
        }

        let blockinfos = ccls
            .blocks
            .into_iter()
            .map(|b| BlockInfo {
                bytecode_off: b.bytecode_off,
                bytecode_end: b.bytecode_end,
                num_params: b.num_params,
                num_vars: b.num_vars,
            })
            .collect();

        let mut consts = Vec::with_capacity(ccls.consts.len());
        for c in ccls.consts {
            consts.push(match c {
                cobjects::Const::String(s) => String_::new(vm, s),
                cobjects::Const::Int(i) => vtry!(Val::from_isize(vm, i)),
            });
        }
        ValResult::from_val(Val::from_obj(
            vm,
            Class {
                name: String_::new(vm, ccls.name),
                path: ccls.path,
                supercls,
                num_inst_vars: ccls.num_inst_vars,
                methods,
                blockinfos,
                instrs: ccls.instrs,
                consts,
                sends: ccls.sends,
            },
        ))
    }

    pub fn name(&self, _: &VM) -> ValResult {
        ValResult::from_val(self.name.clone())
    }

    pub fn get_method(&self, vm: &VM, msg: &str) -> Result<(Val, &Method), Box<VMError>> {
        self.methods
            .get(msg)
            .map(|x| Ok((Val::recover(self), x)))
            .unwrap_or_else(|| match &self.supercls {
                Some(scls) => scls.downcast::<Class>(vm)?.get_method(vm, msg),
                None => Err(Box::new(VMError::UnknownMethod(msg.to_owned()))),
            })
    }

    pub fn blockinfo(&self, blockinfo_off: usize) -> &BlockInfo {
        &self.blockinfos[blockinfo_off]
    }
}

#[derive(Debug, GcLayout)]
pub struct Method {
    pub name: String,
    pub body: MethodBody,
}

#[derive(Debug)]
pub enum MethodBody {
    /// A built-in primitive.
    Primitive(Primitive),
    /// User bytecode.
    User {
        /// How many variables does this method define?
        num_vars: usize,
        /// The offset of this method's bytecode in its parent class.
        bytecode_off: usize,
    },
}

impl Obj for Method {
    fn dyn_objtype(&self) -> ObjType {
        ObjType::Method
    }

    fn get_class(&self, _: &VM) -> Val {
        unimplemented!();
    }
}

impl NotUnboxable for Method {}

impl StaticObjType for Method {
    fn static_objtype() -> ObjType {
        ObjType::Method
    }
}

/// An instance of a user class.
#[derive(Debug, GcLayout)]
pub struct Inst {
    class: Val,
    inst_vars: UnsafeCell<Vec<Val>>,
}

impl Obj for Inst {
    fn dyn_objtype(&self) -> ObjType {
        ObjType::Inst
    }

    fn get_class(&self, _: &VM) -> Val {
        self.class.clone()
    }
}

impl NotUnboxable for Inst {}

impl StaticObjType for Inst {
    fn static_objtype() -> ObjType {
        ObjType::Inst
    }
}

impl Inst {
    pub fn new(vm: &VM, class: Val) -> Val {
        let cls: &Class = class.downcast(vm).unwrap();
        let mut inst_vars = Vec::with_capacity(cls.num_inst_vars);
        inst_vars.resize(cls.num_inst_vars, Val::illegal());
        let inst = Inst {
            class,
            inst_vars: UnsafeCell::new(inst_vars),
        };
        Val::from_obj(vm, inst)
    }

    pub fn inst_var_lookup(&self, n: usize) -> Val {
        let inst_vars = unsafe { &mut *self.inst_vars.get() };
        inst_vars[n].clone()
    }

    pub fn inst_var_set(&self, n: usize, v: Val) {
        let inst_vars = unsafe { &mut *self.inst_vars.get() };
        inst_vars[n] = v;
    }
}

#[derive(Debug, GcLayout)]
/// A boxed `isize`.
pub struct Int {
    val: isize,
}

impl Obj for Int {
    fn dyn_objtype(&self) -> ObjType {
        ObjType::Int
    }

    fn get_class(&self, vm: &VM) -> Val {
        vm.int_cls.clone()
    }

    fn to_strval(&self, vm: &VM) -> ValResult {
        ValResult::from_val(String_::new(vm, self.val.to_string()))
    }
}

impl StaticObjType for Int {
    fn static_objtype() -> ObjType {
        ObjType::Int
    }
}

impl Int {
    /// Create a `Val` representing the `usize` integer `i`. The `Val` is guaranteed to be boxed
    /// internally.
    pub fn boxed_isize(vm: &VM, i: isize) -> ValResult {
        ValResult::from_val(Val::from_obj(vm, Int { val: i }))
    }

    pub fn as_isize(&self) -> isize {
        self.val
    }

    pub fn as_usize(&self) -> Option<usize> {
        if self.val >= 0 {
            Some(self.val as usize)
        } else {
            None
        }
    }
}

#[derive(Debug, GcLayout)]
/// A boxed arbitrary sized `BigInt`.
pub struct ArbInt {
    val: BigInt,
}

impl NotUnboxable for ArbInt {}

impl Obj for ArbInt {
    fn dyn_objtype(&self) -> ObjType {
        ObjType::ArbInt
    }

    fn get_class(&self, vm: &VM) -> Val {
        vm.int_cls.clone()
    }

    fn to_strval(&self, vm: &VM) -> ValResult {
        ValResult::from_val(String_::new(vm, self.val.to_string()))
    }

    fn add(&self, vm: &VM, other: Val) -> ValResult {
        if let Some(rhs) = other.as_isize(vm) {
            ArbInt::new(vm, &self.val + rhs)
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            ArbInt::new(vm, &self.val + &rhs.val)
        } else {
            ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            })
        }
    }

    fn sub(&self, vm: &VM, other: Val) -> ValResult {
        if let Some(rhs) = other.as_isize(vm) {
            ArbInt::new(vm, &self.val - &rhs)
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            ArbInt::new(vm, &self.val - &rhs.val)
        } else {
            ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            })
        }
    }

    fn mul(&self, vm: &VM, other: Val) -> ValResult {
        if let Some(rhs) = other.as_isize(vm) {
            ArbInt::new(vm, &self.val * rhs)
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            ArbInt::new(vm, &self.val * &rhs.val)
        } else {
            ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            })
        }
    }

    fn div(&self, vm: &VM, other: Val) -> ValResult {
        if let Some(rhs) = other.as_isize(vm) {
            if rhs == 0 {
                ValResult::from_vmerror(VMError::DivisionByZero)
            } else {
                ArbInt::new(vm, &self.val / rhs)
            }
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            match self.val.checked_div(&rhs.val) {
                Some(i) => ArbInt::new(vm, i),
                None => ValResult::from_vmerror(VMError::DivisionByZero),
            }
        } else {
            ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            })
        }
    }

    fn shl(&self, vm: &VM, other: Val) -> ValResult {
        if let Some(rhs) = other.as_isize(vm) {
            if rhs < 0 {
                ValResult::from_vmerror(VMError::NegativeShift)
            } else {
                let rhs_i = rtry!(usize::try_from(rhs).map_err(|_| Box::new(VMError::ShiftTooBig)));
                ArbInt::new(vm, &self.val << rhs_i)
            }
        } else if let Some(_) = other.try_downcast::<ArbInt>(vm) {
            ValResult::from_vmerror(VMError::ShiftTooBig)
        } else {
            ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            })
        }
    }

    fn equals(&self, vm: &VM, other: Val) -> ValResult {
        let b = if other.dyn_objtype(vm) == ObjType::Int {
            debug_assert!(self.val != BigInt::from_isize(other.as_isize(vm).unwrap()).unwrap());
            false
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            self.val == rhs.val
        } else {
            false
        };
        ValResult::from_val(Val::from_bool(vm, b))
    }

    fn not_equals(&self, vm: &VM, other: Val) -> ValResult {
        let b = if other.dyn_objtype(vm) == ObjType::Int {
            debug_assert!(self.val != BigInt::from_isize(other.as_isize(vm).unwrap()).unwrap());
            true
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            self.val != rhs.val
        } else {
            true
        };
        ValResult::from_val(Val::from_bool(vm, b))
    }

    fn greater_than(&self, vm: &VM, other: Val) -> ValResult {
        let b = if other.dyn_objtype(vm) == ObjType::Int {
            debug_assert!(self.val > BigInt::from_isize(other.as_isize(vm).unwrap()).unwrap());
            true
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            self.val > rhs.val
        } else {
            return ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            });
        };
        ValResult::from_val(Val::from_bool(vm, b))
    }

    fn greater_than_equals(&self, vm: &VM, other: Val) -> ValResult {
        let b = if other.dyn_objtype(vm) == ObjType::Int {
            debug_assert!(self.val >= BigInt::from_isize(other.as_isize(vm).unwrap()).unwrap());
            true
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            self.val >= rhs.val
        } else {
            return ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            });
        };
        ValResult::from_val(Val::from_bool(vm, b))
    }

    fn less_than(&self, vm: &VM, other: Val) -> ValResult {
        let b = if other.dyn_objtype(vm) == ObjType::Int {
            debug_assert!(self.val < BigInt::from_isize(other.as_isize(vm).unwrap()).unwrap());
            false
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            self.val < rhs.val
        } else {
            return ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            });
        };
        ValResult::from_val(Val::from_bool(vm, b))
    }

    fn less_than_equals(&self, vm: &VM, other: Val) -> ValResult {
        let b = if other.dyn_objtype(vm) == ObjType::Int {
            debug_assert!(self.val <= BigInt::from_isize(other.as_isize(vm).unwrap()).unwrap());
            false
        } else if let Some(rhs) = other.try_downcast::<ArbInt>(vm) {
            self.val <= rhs.val
        } else {
            return ValResult::from_vmerror(VMError::NotANumber {
                got: other.dyn_objtype(vm),
            });
        };
        ValResult::from_val(Val::from_bool(vm, b))
    }
}

impl StaticObjType for ArbInt {
    fn static_objtype() -> ObjType {
        ObjType::ArbInt
    }
}

impl ArbInt {
    /// Create a `Val` representing the `BigInt` integer `val`. Note that this will create the most
    /// efficient integer representation that can represent `val` (i.e. this might create a tagged
    /// `isize`, a boxed `isize`, or a boxed `BigInt`) -- the VM relies, in various places, on this
    /// property (e.g. an `ArbInt` with a value `1` would cause odd errors elsewhere).
    pub fn new(vm: &VM, val: BigInt) -> ValResult {
        if let Some(i) = val.to_isize() {
            Val::from_isize(vm, i)
        } else {
            ValResult::from_val(Val::from_obj(vm, ArbInt { val }))
        }
    }

    pub fn bigint(&self) -> &BigInt {
        &self.val
    }
}

#[derive(Debug, GcLayout)]
pub struct String_ {
    s: String,
}

impl Obj for String_ {
    fn dyn_objtype(&self) -> ObjType {
        ObjType::String_
    }

    fn get_class(&self, vm: &VM) -> Val {
        vm.str_cls.clone()
    }
}

impl NotUnboxable for String_ {}

impl StaticObjType for String_ {
    fn static_objtype() -> ObjType {
        ObjType::String_
    }
}

impl String_ {
    pub fn new(vm: &VM, s: String) -> Val {
        Val::from_obj(vm, String_ { s })
    }

    pub fn as_str(&self) -> &str {
        &self.s
    }

    /// Concatenate this string with another string and return the result.
    pub fn concatenate(&self, vm: &VM, other: Val) -> ValResult {
        let other_str: &String_ = rtry!(other.downcast(vm));

        // Since strings are immutable, concatenating an empty string means we don't need to
        // make a new string.
        if self.s.is_empty() {
            return ValResult::from_val(other);
        } else if other_str.s.is_empty() {
            return ValResult::from_val(Val::recover(self));
        }

        let mut new = String::with_capacity(self.s.len() + other_str.s.len());
        new.push_str(&self.s);
        new.push_str(&other_str.s);
        ValResult::from_val(String_::new(vm, new))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::val::{ValKind, BITSIZE, TAG_BITSIZE};
    use std::str::FromStr;

    #[test]
    fn test_boxed_int() {
        let vm = VM::new_no_bootstrap();

        assert_eq!(Val::from_isize(&vm, 12345).unwrap().valkind(), ValKind::INT);
        assert_eq!(
            Int::boxed_isize(&vm, 12345).unwrap().valkind(),
            ValKind::GCBOX
        );

        let v = Val::from_isize(&vm, 12345).unwrap();
        assert_eq!(
            v.tobj(&vm)
                .unwrap()
                .downcast::<Int>()
                .unwrap()
                .as_usize()
                .unwrap(),
            v.as_usize(&vm).unwrap()
        );
    }

    #[test]
    fn test_bint() {
        let vm = VM::new_no_bootstrap();

        assert_eq!(Val::from_isize(&vm, 0).unwrap().valkind(), ValKind::INT);
        assert_eq!(
            Val::from_isize(&vm, 1 << (BITSIZE - 1 - TAG_BITSIZE))
                .unwrap()
                .valkind(),
            ValKind::INT
        );
        assert_eq!(
            Val::from_isize(&vm, -1 - 1 << (BITSIZE - 1 - TAG_BITSIZE))
                .unwrap()
                .valkind(),
            ValKind::INT
        );
        assert_eq!(
            Val::from_isize(&vm, 1 << (BITSIZE - 1)).unwrap().valkind(),
            ValKind::GCBOX
        );
        assert_eq!(
            Val::from_isize(&vm, isize::min_value())
                .unwrap()
                .add(&vm, Val::from_isize(&vm, isize::min_value()).unwrap())
                .unwrap()
                .downcast::<ArbInt>(&vm)
                .unwrap()
                .val,
            BigInt::from_str("-18446744073709551616").unwrap()
        );
        // Check that sizes "downsize" from more expensive to cheaper types.
        assert_eq!(
            Val::from_isize(&vm, isize::max_value())
                .unwrap()
                .sub(&vm, Val::from_isize(&vm, isize::max_value()).unwrap())
                .unwrap()
                .valkind(),
            ValKind::INT
        );
        let bi = Val::from_isize(&vm, isize::max_value())
            .unwrap()
            .add(&vm, Val::from_isize(&vm, 10).unwrap())
            .unwrap();
        assert!(bi.downcast::<ArbInt>(&vm).is_ok());
        assert_eq!(
            bi.tobj(&vm)
                .unwrap()
                .sub(&vm, Val::from_isize(&vm, 1 << (TAG_BITSIZE + 2)).unwrap())
                .unwrap()
                .valkind(),
            ValKind::GCBOX
        );
        assert_eq!(
            bi.tobj(&vm)
                .unwrap()
                .sub(&vm, Val::from_isize(&vm, isize::max_value()).unwrap())
                .unwrap()
                .valkind(),
            ValKind::INT
        );
        // Different LHS and RHS types
        assert!(Val::from_isize(&vm, 1)
            .unwrap()
            .add(&vm, bi.clone())
            .unwrap()
            .downcast::<ArbInt>(&vm)
            .is_ok());
        assert!(Val::from_isize(&vm, 1)
            .unwrap()
            .sub(&vm, bi.clone())
            .unwrap()
            .downcast::<ArbInt>(&vm)
            .is_ok());
        assert!(Val::from_isize(&vm, 1)
            .unwrap()
            .mul(&vm, bi.clone())
            .unwrap()
            .downcast::<ArbInt>(&vm)
            .is_ok());
        assert_eq!(
            Val::from_isize(&vm, 1)
                .unwrap()
                .div(&vm, bi.clone())
                .unwrap()
                .valkind(),
            ValKind::INT
        );

        assert!(bi
            .clone()
            .add(&vm, Val::from_isize(&vm, 1).unwrap())
            .unwrap()
            .downcast::<ArbInt>(&vm)
            .is_ok());
        assert!(bi
            .clone()
            .sub(&vm, Val::from_isize(&vm, 1).unwrap())
            .unwrap()
            .downcast::<ArbInt>(&vm)
            .is_ok());
        assert!(bi
            .clone()
            .mul(&vm, Val::from_isize(&vm, 1).unwrap())
            .unwrap()
            .downcast::<ArbInt>(&vm)
            .is_ok());
        assert_eq!(
            bi.clone()
                .div(&vm, Val::from_isize(&vm, 99999999).unwrap())
                .unwrap()
                .valkind(),
            ValKind::INT
        );
    }
}
