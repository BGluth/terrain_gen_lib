use std::any::Any;
use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};

use crate::generation::{DataRequestedByPass, GenerationError};

// TODO: Make concrete/trait versions of these aliases?
pub type PassId = &'static str;
pub type DataId = &'static str;
pub type ParamId = &'static str;

pub type PassRunFunc = dyn Fn(DataRequestedByPass) -> PassRunResult;
pub type PassRunResult = Result<PassId, String>;

const ERR_BULLET_SPACES: usize = 4;

pub struct RawRegistrationInfo {
    parameters: HashMap<&'static str, Box<dyn Any>>,
    info_for_passes: Vec<RawPassInfo>,
    max_threads: usize,
}

impl RawRegistrationInfo {
    pub fn new(max_threads: usize) -> RawRegistrationInfo {
        RawRegistrationInfo {
            parameters: HashMap::new(),
            info_for_passes: Vec::new(),
            max_threads,
        }
    }

    pub fn add_param(mut self, p_key: &'static str, p_val: Box<dyn Any>) -> Self {
        self.parameters.insert(p_key, p_val);
        self
    }

    pub fn add_pass_reg_info(mut self, pass_info: RawPassInfo) -> Self {
        self.info_for_passes.push(pass_info);
        self
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum PassInfoDataKeys {
    PassDesc,
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum PassInfoGroupedDataKeys {
    ReqParm,
    ReadAcc,
    WriteAcc,
    CreateAcc,
    PrereqPass,
}

pub struct RawPassInfo {
    name: &'static str,
    run_func: Box<PassRunFunc>,
    data: HashMap<PassInfoDataKeys, Box<dyn Any>>,
    grouped_data: HashMap<PassInfoGroupedDataKeys, Vec<&'static str>>,
}

impl RawPassInfo {
    pub fn new(name: &'static str, run_func: Box<PassRunFunc>) -> RawPassInfo {
        RawPassInfo {
            name,
            run_func,
            data: HashMap::new(),
            grouped_data: HashMap::new(),
        }
    }

    pub fn set_pass_desc(self, pass_desc: String) -> Self {
        self.add_data(PassInfoDataKeys::PassDesc, Box::new(pass_desc))
    }

    pub fn add_req_param(self, param_key: ParamId) -> Self {
        self.add_grouped_data(PassInfoGroupedDataKeys::ReqParm, param_key)
    }

    pub fn add_data_read_access(self, data_key: DataId) -> Self {
        self.add_grouped_data(PassInfoGroupedDataKeys::ReadAcc, data_key)
    }

    pub fn add_data_write_access(self, data_key: DataId) -> Self {
        self.add_grouped_data(PassInfoGroupedDataKeys::WriteAcc, data_key)
    }

    pub fn add_data_create_access(self, data_key: DataId) -> Self {
        self.add_grouped_data(PassInfoGroupedDataKeys::CreateAcc, data_key)
    }

    pub fn add_prereq_pass(self, prereq_pass_name: PassId) -> Self {
        self.add_grouped_data(PassInfoGroupedDataKeys::PrereqPass, prereq_pass_name)
    }

    fn add_data(mut self, info_type: PassInfoDataKeys, data: Box<dyn Any>) -> Self {
        self.data.insert(info_type, data);
        self
    }

    fn add_grouped_data(mut self, info_type: PassInfoGroupedDataKeys, data: &'static str) -> Self {
        self.grouped_data.entry(info_type).or_default().push(data);
        self
    }
}

pub struct RegisteredInfo {
    pub pass_data: PassData,
    pub non_pass_data: NonPassData,
}

impl RegisteredInfo {
    fn new(non_pass_data: NonPassData, params: HashMap<ParamId, Box<dyn Any>>) -> RegisteredInfo {
        unimplemented!()
    }
}

pub struct NonPassData {
    pub max_threads: usize,
}

impl NonPassData {
    fn new(max_threads: usize) -> NonPassData {
        unimplemented!()
    }
}

pub struct PassData {
    pub pass_core_data: PassCoreData,
    pub pass_meta_data: PassMetaData,
}

impl PassData {
    fn new() -> PassData {
        unimplemented!()
    }
}

pub struct PassCoreData {
    pub params_and_data: ParamsAndData,
    pub run_funcs: HashMap<PassId, Box<PassRunFunc>>,
}

pub struct ParamsAndData {
    pub params: HashMap<ParamId, Box<dyn Any>>,
    pub data: HashMap<DataId, UnsafeCell<Box<dyn Any>>>,
}

pub struct PassMetaData {
    pub pass_names: Vec<&'static str>,
    pub pass_descs: HashMap<&'static str, String>,
    pub pass_mappings: PassMappings,
}

impl PassMetaData {
    pub fn get_pass_ids<'a>(&'a self) -> impl Iterator<Item = PassId> + 'a {
        self.pass_names.iter().cloned()
    }
}

pub struct PassMappings {
    pub pass_data_req_params: HashMap<PassId, Vec<ParamId>>,
    pub pass_data_read_reqs: HashMap<PassId, Vec<DataId>>,
    pub pass_data_write_reqs: HashMap<PassId, Vec<DataId>>,
    pub pass_data_create_reqs: HashMap<PassId, Vec<DataId>>,
    pub prereq_passes_of_passes: HashMap<PassId, Vec<PassId>>,
}

pub fn process_reg_info(raw_reg_info: RawRegistrationInfo) -> Result<RegisteredInfo, String> {
    validate_that_no_passes_have_duplicate_names(&raw_reg_info)?;

    let reg_info = extract_data_from_raw_reg_info_into_gen_state(raw_reg_info);

    validate_that_all_requested_data_is_written_by_a_pass(&reg_info)?;
    validate_that_no_passes_have_both_read_write_on_any_data(
        &reg_info.pass_data.pass_meta_data.pass_mappings,
    )?;
    validate_that_all_prereq_passes_of_passes_exist(&reg_info.pass_data.pass_meta_data)?;

    Ok(reg_info)
}

fn validate_that_no_passes_have_duplicate_names(reg_info: &RawRegistrationInfo) -> GenerationError {
    let mut seen_names = HashSet::new();
    let mut duplicate_names = Vec::new();

    for pass_info in reg_info.info_for_passes.iter() {
        let already_exists = !seen_names.insert(pass_info.name);
        if already_exists {
            duplicate_names.push(pass_info.name);
        }
    }

    if duplicate_names.len() > 0 {
        let mut err_str = String::from("The following names we're given to one or more passes:\n");
        write_strs_as_bullets(duplicate_names, &mut err_str);
        return Err(err_str);
    }

    Ok(())
}

fn validate_that_all_passes_have_key(
    reg_info: &RawRegistrationInfo,
    key: PassInfoDataKeys,
    missing_data_str: &str,
) -> GenerationError {
    let mut other_data_of_passes_missing_key = Vec::new();

    for pass_info in reg_info.info_for_passes.iter() {
        if !pass_info.data.contains_key(&key) {
            other_data_of_passes_missing_key.push((&pass_info.data, &pass_info.grouped_data));
        }
    }

    if other_data_of_passes_missing_key.len() > 0 {
        let mut err_str = format!(
            "The following registered pass info do not have a corresponding registered {}:\n",
            missing_data_str
        );

        for (data, grouped_data) in other_data_of_passes_missing_key {
            let reg_info_str = format!("data: {:#?}\ngrouped data: {:#?}\n\n", data, grouped_data);
            err_str.push_str(&reg_info_str);
        }

        return Err(err_str);
    }

    Ok(())
}

fn validate_that_no_passes_have_both_read_write_on_any_data(
    p_mappings: &PassMappings,
) -> GenerationError {
    let mut data_with_read_and_write = Vec::new();
    let mut common_keys_buf: Vec<&'static str> = Vec::new();

    for (pass_name, data_read_by_pass) in p_mappings.pass_data_read_reqs.iter() {
        match p_mappings.pass_data_write_reqs.get(pass_name) {
            None => continue,
            Some(write_data_keys) => {
                let keys_in_both_iter = write_data_keys
                    .iter()
                    .filter(|w_key| data_read_by_pass.contains(w_key));
                common_keys_buf.extend(keys_in_both_iter);

                if common_keys_buf.len() > 0 {
                    data_with_read_and_write.push((*pass_name, common_keys_buf.clone()));
                }
                common_keys_buf.clear();
            }
        }
    }

    if data_with_read_and_write.len() > 0 {
        let mut err_str = String::from(
            "The following passes have the same datatypes registered as both read and write:\n",
        );
        append_parent_and_child_elements_to_string(
            data_with_read_and_write.into_iter(),
            &mut err_str,
        );

        return Err(err_str);
    }

    Ok(())
}

fn validate_that_all_prereq_passes_of_passes_exist(p_meta: &PassMetaData) -> GenerationError {
    let mut undefined_prereqs = Vec::new();
    let mut buf = Vec::new();

    for (pass_name, prereqs) in p_meta.pass_mappings.prereq_passes_of_passes.iter() {
        let missing_prereqs = prereqs
            .iter()
            .filter(|prereq| !p_meta.pass_names.contains(prereq));
        buf.extend(missing_prereqs);

        if buf.len() > 0 {
            undefined_prereqs.push((*pass_name, buf.clone()));
        }
        buf.clear();
    }

    if undefined_prereqs.len() > 0 {
        let mut err_str =
            String::from("The following passes have prerequisite passes that are not defined:");
        append_parent_and_child_elements_to_string(undefined_prereqs.into_iter(), &mut err_str);
    }

    Ok(())
}

fn validate_that_all_requested_data_is_written_by_a_pass(
    gen_state: &RegisteredInfo,
) -> GenerationError {
    unimplemented!();
}

fn extract_data_from_raw_reg_info_into_gen_state(
    raw_reg_info: RawRegistrationInfo,
) -> RegisteredInfo {
    let non_pass_data = NonPassData::new(raw_reg_info.max_threads);
    let mut reg_info = RegisteredInfo::new(non_pass_data, raw_reg_info.parameters);

    for pass_info in raw_reg_info.info_for_passes {
        extract_data_from_pass_info_into_reg_info(pass_info, &mut reg_info.pass_data);
    }

    reg_info
}

fn extract_data_from_pass_info_into_reg_info(
    mut raw_pass_info: RawPassInfo,
    p_data: &mut PassData,
) {
    p_data.pass_meta_data.pass_names.push(raw_pass_info.name);
    p_data
        .pass_core_data
        .run_funcs
        .insert(raw_pass_info.name, raw_pass_info.run_func);

    let data_drain = raw_pass_info.data.drain();
    let grouped_data_drain = raw_pass_info.grouped_data.drain();

    for (key, val) in data_drain {
        match key {
            PassInfoDataKeys::PassDesc => {
                let desc = val.downcast_ref::<Box<String>>().unwrap().to_string();
                p_data
                    .pass_meta_data
                    .pass_descs
                    .insert(&raw_pass_info.name, desc);
            }
        }
    }

    for (key, val) in grouped_data_drain {
        match key {
            PassInfoGroupedDataKeys::ReqParm => {
                p_data
                    .pass_meta_data
                    .pass_mappings
                    .pass_data_req_params
                    .insert(&raw_pass_info.name, val);
            }

            PassInfoGroupedDataKeys::ReadAcc => {
                p_data
                    .pass_meta_data
                    .pass_mappings
                    .pass_data_read_reqs
                    .insert(raw_pass_info.name, val);
            }

            PassInfoGroupedDataKeys::WriteAcc => {
                p_data
                    .pass_meta_data
                    .pass_mappings
                    .pass_data_write_reqs
                    .insert(raw_pass_info.name, val);
            }

            PassInfoGroupedDataKeys::CreateAcc => {
                p_data
                    .pass_meta_data
                    .pass_mappings
                    .pass_data_create_reqs
                    .insert(raw_pass_info.name, val);
            }

            PassInfoGroupedDataKeys::PrereqPass => {
                p_data
                    .pass_meta_data
                    .pass_mappings
                    .prereq_passes_of_passes
                    .insert(raw_pass_info.name, val);
            }
        }
    }
}

fn append_parent_and_child_elements_to_string<'a, I>(
    par_str_with_bullets: I,
    str_to_append_to: &mut String,
) where
    I: Iterator<Item = (&'a str, Vec<&'a str>)>,
{
    for (par_string, bullet_strs) in par_str_with_bullets {
        str_to_append_to.push_str(&par_string);
        write_strs_as_bullets(bullet_strs, str_to_append_to);
        str_to_append_to.push_str("\n");
    }
}

fn write_strs_as_bullets(bullet_strs: Vec<&str>, str_to_append_to: &mut String) {
    for bullet in bullet_strs {
        str_to_append_to.push_str(&" ".repeat(ERR_BULLET_SPACES));
        str_to_append_to.push_str(&format!("- {}\n", bullet));
    }
}
