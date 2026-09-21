extern crate parking_lot;

use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

pub type InstanceInit = Box<dyn Fn() -> Box<dyn Any + Send + Sync + 'static> + 'static>;

#[derive(Default)]
pub struct Container {
    init: RwLock<HashMap<TypeId, InstanceInit>>,
    instances: RwLock<HashMap<TypeId, Arc<Box<dyn Any + Send + Sync + 'static>>>>,
}

impl std::fmt::Debug for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Container")
            .field(
                "init",
                &(*self.init.read())
                    .keys()
                    .map(|k| format!("{:?}", k))
                    .collect::<Vec<String>>(),
            )
            .field(
                "instances",
                &(*self.instances.read())
                    .keys()
                    .map(|k| format!("{:?}", k))
                    .collect::<Vec<String>>(),
            )
            .finish()
    }
}

impl Container {
    pub fn add<T, C>(&self, factory: C) -> &Self
    where
        T: Any + Send + Sync + 'static,
        C: Fn() -> Box<dyn Any + Send + Sync + 'static> + 'static,
    {
        self.init.write().insert(TypeId::of::<T>(), Box::new(factory));

        self
    }

    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Arc<Box<T>> {
        let key = TypeId::of::<T>();
        if let Some(instance) = self.instances.read().get(&key) {
            if !(*instance).is::<T>() {
                panic!("Type mismatch, you must have inserted different types in the map keys and values");
            }

            // The unsafe call is sound here because we check the types above
            unsafe {
                return Arc::clone(&*(instance as *const dyn Any as *const Arc<Box<T>>));
            }
        }

        if let Some(init_func) = self.init.write().remove(&key) {
            self.instances.write().insert(key, Arc::new(init_func()));
            return self.get();
        }

        panic!("This should not happen");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Test {
        test_true: Arc<Box<TestInjectableTrue>>,
        test_false: Arc<Box<TestInjectableFalse>>,
    }

    struct TestInjectableTrue {
        meh_true: bool,
        extra_data: u8,
    }

    impl TestInjectableTrue {
        pub fn new() -> Self {
            TestInjectableTrue {
                meh_true: true,
                extra_data: 10,
            }
        }

        pub fn ret_bool(&self) -> bool {
            self.meh_true
        }

        pub fn ret_data(&self) -> u8 {
            self.extra_data
        }
    }

    struct TestInjectableFalse {
        extra_data: i64,
        meh_false: bool,
    }

    impl TestInjectableFalse {
        pub fn new() -> Self {
            TestInjectableFalse {
                meh_false: false,
                extra_data: 100,
            }
        }

        pub fn ret_bool(&self) -> bool {
            self.meh_false
        }

        pub fn ret_data(&self) -> i64 {
            self.extra_data
        }
    }

    #[test]
    fn it_works() {
        let container = Container::default();
        container.add::<TestInjectableTrue, _>(|| Box::new(TestInjectableTrue::new()));
        container.add::<TestInjectableFalse, _>(|| Box::new(TestInjectableFalse::new()));

        let test = Test {
            test_true: container.get(),
            test_false: container.get(),
        };

        assert_eq!(test.test_true.ret_bool(), true);
        assert_eq!(test.test_true.ret_data(), 10);
        assert_eq!(test.test_false.ret_bool(), false);
        assert_eq!(test.test_false.ret_data(), 100);

        let test2 = Test {
            test_true: container.get(),
            test_false: container.get(),
        };

        assert_eq!(test2.test_true.ret_bool(), true);
        assert_eq!(test2.test_true.ret_data(), 10);
        assert_eq!(test2.test_false.ret_bool(), false);
        assert_eq!(test2.test_false.ret_data(), 100);
    }

    #[test]
    #[should_panic]
    fn it_should_panic_on_type_mismatch() {
        let container = Container::default();
        container.add::<TestInjectableFalse, _>(|| Box::new(TestInjectableTrue::new()));
        container.add::<TestInjectableTrue, _>(|| Box::new(TestInjectableFalse::new()));

        let _ = Test {
            test_true: container.get(),
            test_false: container.get(),
        };
    }
}
