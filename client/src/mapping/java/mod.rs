use jni::objects::GlobalRef;
use std::ops::Deref;

pub struct JavaList {
    pub jni_ref: GlobalRef,
}

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
