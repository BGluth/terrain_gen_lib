use std::{any::Any, cell::UnsafeCell, collections::HashMap};

use crate::GeneratorState::{DataId, ParamId};

// Actual data provided by the scheduler to the pass
pub struct DataRequestedByPass<'a> {
    req_params: HashMap<ParamId, &'a Box<dyn Any>>,
    req_data_read: HashMap<DataId, &'a UnsafeCell<Box<dyn Any>>>,
    req_data_write: HashMap<DataId, &'a UnsafeCell<Box<dyn Any>>>,
    new_data_created_by_pass: HashMap<DataId, Option<Box<dyn Any>>>,
}

impl<'a> DataRequestedByPass<'a> {
    pub fn get_param<T: 'static>(&'a self, p_key: ParamId) -> &'a T {
        // TODO: Consider if all passes should always have access to all parameters...
        Self::debug_assert_key_exists_in_table(&self.req_params, p_key, "a parameter");
        Self::get_any_param_from_table_and_downcast(&self.req_params, "a parameter", p_key)
    }

    pub fn get_data_ref<T: 'static>(&self, d_key: DataId) -> &'a T {
        Self::debug_assert_key_exists_in_table(&self.req_data_read, d_key, "data with read access");

        unsafe {
            return Self::get_any_data_from_table_and_downcast(
                &self.req_data_read,
                "write data",
                d_key,
            );
        }
    }

    pub fn get_data_mut<T: 'static>(&'a mut self, d_key: DataId) -> &'a mut T {
        Self::debug_assert_key_exists_in_table(
            &self.req_data_write,
            d_key,
            "data with write access",
        );

        unsafe {
            return Self::get_any_data_from_table_and_downcast(
                &self.req_data_read,
                "read data",
                d_key,
            );
        }
    }

    pub fn write_data<T: 'static>(&mut self, d_key: DataId, data: T) {
        debug_assert!(
            !self.new_data_created_by_pass.contains_key(d_key),
            "A pass tried writing data ({}) that it did not declare it might write!",
            d_key
        );

        self.new_data_created_by_pass
            .insert(d_key, Some(Box::new(data)));
    }

    fn new() -> DataRequestedByPass<'a> {
        DataRequestedByPass {
            req_params: HashMap::new(),
            req_data_read: HashMap::new(),
            req_data_write: HashMap::new(),
            new_data_created_by_pass: HashMap::new(),
        }
    }

    fn add_req_param(&mut self, p_key: ParamId, param: &'a Box<dyn Any>) {
        self.req_params.insert(p_key, param);
    }

    fn add_req_read_data(&mut self, d_key: DataId, data: &'a UnsafeCell<Box<dyn Any>>) {
        self.req_data_read.insert(d_key, data);
    }

    fn add_req_write_data(&mut self, d_key: DataId, data: &'a UnsafeCell<Box<dyn Any>>) {
        self.req_data_write.insert(d_key, data);
    }

    fn add_req_create_data(&mut self, d_key: DataId) {
        self.new_data_created_by_pass.insert(d_key, None);
    }

    fn debug_assert_key_exists_in_table<T>(
        table: &HashMap<&str, T>,
        key: &str,
        access_type_str: &str,
    ) {
        debug_assert!(
            table.contains_key(key),
            "A pass tried to access {} ({}) that it did not request! Params requested: {:#?}",
            key,
            access_type_str,
            table.keys()
        );
    }

    unsafe fn get_any_data_from_table_and_downcast<T: 'static>(
        table: &HashMap<&str, &'a UnsafeCell<Box<dyn Any>>>,
        type_str: &str,
        key: &str,
    ) -> &'a mut Box<T> {
        let data_any = table.get(key).unwrap().get().as_mut().unwrap();
        match data_any.downcast_mut::<Box<T>>() {
            None => panic!("{}", Self::gen_downcast_err_str(type_str, key)),
            Some(param) => param,
        }
    }

    // TODO: Condense this into get_any_data...
    fn get_any_param_from_table_and_downcast<T: 'static>(
        table: &HashMap<ParamId, &'a Box<dyn Any>>,
        type_str: &str,
        key: &str,
    ) -> &'a T {
        let data_any = *table.get(key).unwrap();
        match data_any.downcast_ref::<Box<T>>() {
            None => panic!("{}", Self::gen_downcast_err_str(type_str, key)),
            Some(param) => param,
        }
    }

    fn gen_downcast_err_str(type_str: &str, key: &str) -> String {
        format!("Tried to cast {} of key {} but failed! (Don't know how to print name of downcast type...)", type_str, key)
    }
}
