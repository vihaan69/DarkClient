use jni::objects::GlobalRef;
use std::ops::Deref;

#[allow(dead_code)]
pub struct JavaList {
    pub jni_ref: GlobalRef,
}

#[allow(dead_code)]
pub struct JavaSet {
    pub jni_ref: GlobalRef,
}

impl Deref for JavaList {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}

impl Deref for JavaSet {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
