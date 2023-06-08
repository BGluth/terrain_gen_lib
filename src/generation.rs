use std::any::Any;
use std::cell::UnsafeCell;
use std::cmp::min;
use std::collections::{HashMap, VecDeque};
use std::error::Error;

use crossbeam::channel::{Receiver, Sender};
use threadpool::ThreadPool;

use crate::GeneratorState::{process_reg_info, RawRegistrationInfo};
use crate::GeneratorState::{DataId, ParamId, PassId, PassRunFunc, PassRunResult};
use crate::GeneratorState::{
    NonPassData, ParamsAndData, PassCoreData, PassData, PassMappings, PassMetaData, RegisteredInfo,
};

pub type GenerationError = Result<(), String>;

enum DataReqType {
    READ,
    WRITE,
}

struct GenerationState {
    thread_state: PassThreadState,
    sched_state: SchedState,
    pass_data: PassData,
    non_pass_data: NonPassData,
}

impl GenerationState {
    fn new(reg_info: RegisteredInfo) -> GenerationState {
        GenerationState {
            thread_state: PassThreadState::new(),
            sched_state: SchedState::new(&reg_info.pass_data),
            pass_data: reg_info.pass_data,
            non_pass_data: reg_info.non_pass_data,
        }
    }
}

struct PassThreadState {
    t_comp_msgs_rx: Receiver<PassRunResult>,
    t_comp_msgs_tx: Sender<PassRunResult>,
    t_pool: ThreadPool,
    num_threads_avail: usize,
}

impl PassThreadState {
    fn new() -> PassThreadState {
        unimplemented!()
    }

    fn schedule_run_func_on_thread(
        &mut self,
        run_func: &Box<PassRunFunc>,
        p_data: DataRequestedByPass,
    ) {
        let tx = self.t_comp_msgs_tx.clone();

        // TODO: Fix/refactor!
        // self.t_pool.execute(move || {
        //     let pass_res = run_func(p_data);
        //     tx.send(pass_res);
        // });

        self.num_threads_avail -= 1;
    }
}

struct SchedState {
    p_stages: PassStages,
    p_res_state: PassResState,
    num_passes_rem: usize,
}

impl SchedState {
    fn new(p_data: &PassData) -> SchedState {
        SchedState {
            p_stages: PassStages::new(&p_data.pass_meta_data.pass_mappings),
            p_res_state: PassResState::new(p_data),
            num_passes_rem: p_data.pass_meta_data.pass_names.len(),
        }
    }
}

struct PassStages {
    prereqs_of_passes: PrereqsOfPasses,
    passes_waiting_on_data_to_be_created: PassesWaitingOnDataToBeCreated,
    passes_waiting_for_resources: Vec<PassId>,
    passes_ready_for_scheduling: VecDeque<PassId>,
}

impl PassStages {
    fn new(p_mappings: &PassMappings) -> PassStages {
        PassStages {
            prereqs_of_passes: PrereqsOfPasses::new(p_mappings),
            passes_waiting_on_data_to_be_created: PassesWaitingOnDataToBeCreated::new(p_mappings),
            passes_waiting_for_resources: Vec::new(),
            passes_ready_for_scheduling: VecDeque::new(),
        }
    }

    fn move_pass_from_res_waiting_to_sched_ready(&mut self, res_waiting_idx: usize) {
        let ready_pass_id = self
            .passes_waiting_for_resources
            .swap_remove(res_waiting_idx);
        self.passes_ready_for_scheduling.push_back(ready_pass_id);
    }
}

struct PassResState {
    pass_req_res: HashMap<PassId, PassRequiredRes>,
    data_usage_state: DataUsageState,
}

impl PassResState {
    fn new(p_data: &PassData) -> PassResState {
        PassResState {
            pass_req_res: get_data_reqs_for_passes(&p_data.pass_core_data, &p_data.pass_meta_data),
            data_usage_state: DataUsageState::new(&p_data.pass_core_data),
        }
    }
}

struct PrereqsOfPasses {
    passes_that_are_prereqs_of_pass: HashMap<PassId, Vec<PassId>>,
    rem_prereqs_of_passes: HashMap<PassId, usize>,
}

impl PrereqsOfPasses {
    fn new(p_mappings: &PassMappings) -> PrereqsOfPasses {
        let mut rem_prereqs_of_passes = HashMap::new();

        for (pid, prereq_pids) in p_mappings.prereq_passes_of_passes.iter() {
            rem_prereqs_of_passes.insert(*pid, prereq_pids.len());
        }

        PrereqsOfPasses {
            passes_that_are_prereqs_of_pass: p_mappings.prereq_passes_of_passes.clone(),
            rem_prereqs_of_passes,
        }
    }

    fn notify_pass_completed_and_ret_pids_that_can_be_moved_to_waiting_for_created(
        &mut self,
        pass_id_completed: PassId,
    ) -> Vec<PassId> {
        let mut pids_that_have_all_prereqs_met = Vec::new();
        let pid_prereqs_of_comp_pid = self
            .passes_that_are_prereqs_of_pass
            .get_mut(pass_id_completed)
            .into_iter()
            .flatten();

        for pid in pid_prereqs_of_comp_pid {
            *self.rem_prereqs_of_passes.get_mut(pid).unwrap() -= 1;
            if self.rem_prereqs_of_passes[pid] == 0 {
                pids_that_have_all_prereqs_met.push(*pid);
            }
        }

        pids_that_have_all_prereqs_met
    }
}

struct PassesWaitingOnDataToBeCreated {
    pass_ids_waiting_on_data_id: HashMap<DataId, Vec<PassId>>,
    rem_data_waiting_count_per_pass: HashMap<PassId, usize>,
}

// TODO: This impl is very close to PrereqsOfPasses. Might be a good idea to factor out common stuff.
impl PassesWaitingOnDataToBeCreated {
    fn new(p_mappings: &PassMappings) -> PassesWaitingOnDataToBeCreated {
        let mut pass_ids_waiting_on_data_id = HashMap::new();
        let mut rem_data_waiting_count_per_pass = HashMap::new();

        for (pid, dids_waited_on) in p_mappings.pass_data_create_reqs.iter() {
            rem_data_waiting_count_per_pass.insert(*pid, dids_waited_on.len());

            for did in dids_waited_on {
                let waiting_dids = pass_ids_waiting_on_data_id
                    .entry(*did)
                    .or_insert(Vec::new());
                waiting_dids.push(*did);
            }
        }

        PassesWaitingOnDataToBeCreated {
            pass_ids_waiting_on_data_id,
            rem_data_waiting_count_per_pass,
        }
    }

    fn pass_is_waiting(&self, pid: PassId) -> bool {
        match self.rem_data_waiting_count_per_pass.get(pid) {
            Some(num_waiting) => *num_waiting > 0,
            None => false,
        }
    }

    fn notify_data_created_and_ret_pids_that_can_be_moved_to_waiting_for_res(
        &mut self,
        did_created: DataId,
        to_waiting_pids: &mut Vec<PassId>,
    ) {
        let pids_that_are_waiting_on_did = self
            .pass_ids_waiting_on_data_id
            .get(did_created)
            .into_iter()
            .flatten();

        for pid in pids_that_are_waiting_on_did {
            *self.rem_data_waiting_count_per_pass.get_mut(pid).unwrap() -= 1;
            if self.rem_data_waiting_count_per_pass[pid] == 0 {
                to_waiting_pids.push(*pid);
            }
        }
    }
}

struct PassRequiredRes {
    pid: PassId,
    did_and_req_types: Vec<(DataId, DataReqType)>,
}

impl PassRequiredRes {
    fn new(p_id: PassId, d_ids: Vec<(DataId, DataReqType)>) -> PassRequiredRes {
        PassRequiredRes {
            pid: p_id,
            did_and_req_types: d_ids,
        }
    }
}

struct DataUsageState {
    state: HashMap<DataId, DataUseState>,
}

impl DataUsageState {
    fn new(core_data: &PassCoreData) -> DataUsageState {
        let mut state = HashMap::new();
        for (did, _) in core_data.params_and_data.data.iter() {
            state.insert(*did, DataUseState::new());
        }

        DataUsageState { state }
    }

    fn pass_req_res_are_avail(&self, p_req_res: &PassRequiredRes) -> bool {
        for (did, req_type) in p_req_res.did_and_req_types.iter() {
            match req_type {
                DataReqType::READ => {
                    if self.state[did].write_active == true {
                        return false;
                    }
                }

                DataReqType::WRITE => {
                    if self.state[did].num_readers > 0 {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn acquire_req_res_for_pass(&mut self, p_req_res: &PassRequiredRes) {
        for (did, req_type) in p_req_res.did_and_req_types.iter() {
            match req_type {
                DataReqType::READ => {
                    self.state.get_mut(did).unwrap().num_readers += 1;
                }

                DataReqType::WRITE => {
                    self.state.get_mut(did).unwrap().write_active = true;
                }
            }
        }
    }

    fn release_resources_of_completed_pass(&mut self, res_used_by_pass: &PassRequiredRes) {
        unimplemented!();
    }
}

struct DataReadWriteStates {
    data_states: Vec<DataUseState>,
}

impl DataReadWriteStates {
    fn aquire_req_resources_if_avail(&mut self, data_requested: &Vec<(DataId, DataReqType)>) {
        unimplemented!();
    }
}

struct DataUseState {
    num_readers: usize,
    write_active: bool,
}

impl DataUseState {
    fn new() -> DataUseState {
        DataUseState {
            num_readers: 0,
            write_active: false,
        }
    }
}

struct ThreadInfo {
    t_id: usize,
}

pub fn generate_map(reg_data: RawRegistrationInfo) -> GenerationError {
    let reg_info = process_reg_info(reg_data)?;

    run_generation_passes(reg_info)?;

    Ok(())
}

fn run_generation_passes(reg_info: RegisteredInfo) -> GenerationError {
    let mut gen_state = GenerationState::new(reg_info);

    while gen_state.sched_state.num_passes_rem > 0 {
        process_thread_completed_msgs(&mut gen_state);

        block_and_processes_comp_msgs_if_no_threads_avail(&mut gen_state);
        block_and_processes_comp_msgs_if_no_passes_avail(&mut gen_state);

        upgrade_any_passes_from_waiting_for_res_to_ready_for_sched(
            &mut gen_state.sched_state.p_stages,
            &mut gen_state.sched_state.p_res_state,
        );
        schedule_ready_passes_on_avail_threads(
            &mut gen_state.sched_state.p_stages,
            &gen_state.sched_state.p_res_state,
            &gen_state.pass_data.pass_core_data.run_funcs,
            &mut gen_state.pass_data.pass_core_data.params_and_data,
            &gen_state.pass_data.pass_meta_data.pass_mappings,
            &mut gen_state.thread_state,
        );
    }

    Ok(())
}

fn block_and_processes_comp_msgs_if_no_threads_avail(
    gen_state: &mut GenerationState,
) -> GenerationError {
    if gen_state.thread_state.num_threads_avail > 0 {
        return Ok(());
    }

    block_and_process_thread_completed_msgs(gen_state)
}

fn block_and_processes_comp_msgs_if_no_passes_avail(
    gen_state: &mut GenerationState,
) -> GenerationError {
    if gen_state
        .sched_state
        .p_stages
        .passes_ready_for_scheduling
        .len()
        > 0
    {
        return Ok(());
    }

    block_and_process_thread_completed_msgs(gen_state)
}

fn block_and_process_thread_completed_msgs(gen_state: &mut GenerationState) -> GenerationError {
    let pass_id = gen_state.thread_state.t_comp_msgs_rx.recv().map_err(|e| {
        conv_err_to_str_with_when(
            Box::new(e),
            "processing a pass thread complete msg with blocking",
        )
    })??;

    handle_pass_completed(pass_id, gen_state);

    Ok(())
}

fn process_thread_completed_msgs(gen_state: &mut GenerationState) -> GenerationError {
    while !gen_state.thread_state.t_comp_msgs_rx.is_empty() {
        let pass_id = gen_state
            .thread_state
            .t_comp_msgs_rx
            .try_recv()
            .map_err(|e| {
                conv_err_to_str_with_when(
                    Box::new(e),
                    "processing a pass thread complete msg with no blocking",
                )
            })??;

        handle_pass_completed(pass_id, gen_state);
    }

    Ok(())
}

fn handle_pass_completed(comp_pass_id: PassId, gen_state: &mut GenerationState) {
    let new_pass_ids_with_comp_prereqs = gen_state
        .sched_state
        .p_stages
        .prereqs_of_passes
        .notify_pass_completed_and_ret_pids_that_can_be_moved_to_waiting_for_created(comp_pass_id);

    for pid in new_pass_ids_with_comp_prereqs {
        if !gen_state
            .sched_state
            .p_stages
            .passes_waiting_on_data_to_be_created
            .pass_is_waiting(pid)
        {
            // All data needed already exists. Go straight to waiting for resources.
            add_pass_to_res_waiting(pid, &mut gen_state.sched_state.p_stages);
        }
    }

    update_passes_waiting_on_non_existing_data_with_completed_pass(
        comp_pass_id,
        &mut gen_state.sched_state.p_stages,
        &gen_state.pass_data.pass_meta_data.pass_mappings,
    );

    let pass_req_res = &gen_state.sched_state.p_res_state.pass_req_res[comp_pass_id];
    gen_state
        .sched_state
        .p_res_state
        .data_usage_state
        .release_resources_of_completed_pass(pass_req_res);

    gen_state.sched_state.num_passes_rem -= 1;
    gen_state.thread_state.num_threads_avail += 1;
}

fn add_pass_to_res_waiting(pid: PassId, p_stages: &mut PassStages) {
    p_stages.passes_waiting_for_resources.push(pid);
}

fn aquire_data_needed_for_pass() {}

fn get_data_reqs_for_passes(
    p_core_data: &PassCoreData,
    p_meta: &PassMetaData,
) -> HashMap<PassId, PassRequiredRes> {
    let mut req_data_for_passes = HashMap::new();

    for pid in p_meta.get_pass_ids() {
        let mut data_req = Vec::new();

        // A bit messy, but code duplication is very low this way.
        data_req.extend(
            p_meta
                .pass_mappings
                .pass_data_read_reqs
                .get(pid)
                .into_iter()
                .flat_map(|d_ids| d_ids.into_iter().map(|d_id| (*d_id, DataReqType::READ))),
        );
        data_req.extend(
            p_meta
                .pass_mappings
                .pass_data_write_reqs
                .get(pid)
                .into_iter()
                .flat_map(|d_ids| d_ids.into_iter().map(|d_id| (*d_id, DataReqType::WRITE))),
        );

        req_data_for_passes.insert(pid, PassRequiredRes::new(pid, data_req));
    }

    req_data_for_passes
}

fn update_passes_waiting_on_non_existing_data_with_completed_pass(
    comp_pid: PassId,
    p_stages: &mut PassStages,
    p_mappings: &PassMappings,
) {
    let dids_created_by_pass = p_mappings
        .pass_data_create_reqs
        .get(comp_pid)
        .into_iter()
        .flatten();

    let mut pids_that_can_go_to_res_waiting = Vec::new();
    for did in dids_created_by_pass {
        p_stages
            .passes_waiting_on_data_to_be_created
            .notify_data_created_and_ret_pids_that_can_be_moved_to_waiting_for_res(
                did,
                &mut pids_that_can_go_to_res_waiting,
            );
    }

    p_stages
        .passes_waiting_for_resources
        .extend(pids_that_can_go_to_res_waiting);
}

fn schedule_ready_passes_on_avail_threads(
    p_stages: &mut PassStages,
    p_res_state: &PassResState,
    run_funcs: &HashMap<PassId, Box<PassRunFunc>>,
    params_and_data: &mut ParamsAndData,
    p_mappings: &PassMappings,
    t_state: &mut PassThreadState,
) {
    let num_passes_to_schedule = min(
        t_state.num_threads_avail,
        p_stages.passes_ready_for_scheduling.len(),
    );
    for _ in 0..num_passes_to_schedule {
        let pid_to_sched = p_stages.passes_ready_for_scheduling.pop_back().unwrap();
        let req_res_opt = p_res_state.pass_req_res.get(pid_to_sched);
        let run_func = &run_funcs[pid_to_sched];
        let p_data = get_data_req_by_pass(pid_to_sched, req_res_opt, p_mappings, params_and_data);

        t_state.schedule_run_func_on_thread(run_func, p_data);
    }
}

fn get_data_req_by_pass<'a>(
    pid: PassId,
    p_req_res_opt: Option<&'a PassRequiredRes>,
    p_mappings: &'a PassMappings,
    params_and_data: &'a mut ParamsAndData,
) -> DataRequestedByPass<'a> {
    // This module is going to see heavy refactoring, so just get it to compile for now.
    todo!();

    let mut collected_data = DataRequestedByPass::new();

    add_params_and_setup_req_pass_data_for_any_created_data(
        pid,
        &mut collected_data,
        params_and_data,
        p_mappings,
    );

    match p_req_res_opt {
        Some(p_req_res) => {
            add_read_write_data_to_req_pass_data(p_req_res, &mut collected_data, params_and_data)
        }
        None => (),
    }

    collected_data
}

fn add_params_and_setup_req_pass_data_for_any_created_data<'a>(
    pid: PassId,
    coll_data: &'a mut DataRequestedByPass<'a>,
    params_and_data: &'a ParamsAndData,
    p_mappings: &PassMappings,
) {
    for param_id in p_mappings
        .pass_data_req_params
        .get(pid)
        .into_iter()
        .flatten()
    {
        let param = &params_and_data.params[param_id];
        coll_data.add_req_param(param_id, param)
    }

    for did in p_mappings
        .pass_data_create_reqs
        .get(pid)
        .into_iter()
        .flatten()
    {
        coll_data.add_req_create_data(did);
    }
}

fn add_read_write_data_to_req_pass_data<'a>(
    p_req_res: &'a PassRequiredRes,
    coll_data: &'a mut DataRequestedByPass<'a>,
    params_and_data: &'a ParamsAndData,
) {
    for (did, req_type) in p_req_res.did_and_req_types.iter() {
        match req_type {
            DataReqType::READ => {
                let data = &params_and_data.data[did];
                coll_data.add_req_read_data(did, data);
            }

            DataReqType::WRITE => {
                let data = &params_and_data.data[did];
                coll_data.add_req_write_data(did, data);
            }
        }
    }
}

fn upgrade_any_passes_from_waiting_for_res_to_ready_for_sched(
    p_stages: &mut PassStages,
    p_res_state: &mut PassResState,
) {
    let mut passes_moved_to_runnable_on_last_iter = true;
    while passes_moved_to_runnable_on_last_iter {
        passes_moved_to_runnable_on_last_iter = false;

        for i in p_stages.passes_waiting_for_resources.len()..0 {
            let p_req_res = &p_res_state.pass_req_res[p_stages.passes_waiting_for_resources[i]];
            if p_res_state
                .data_usage_state
                .pass_req_res_are_avail(p_req_res)
            {
                p_res_state
                    .data_usage_state
                    .acquire_req_res_for_pass(p_req_res);
                p_stages.move_pass_from_res_waiting_to_sched_ready(i);
                passes_moved_to_runnable_on_last_iter = true;
            }
        }
    }
}

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
            None => panic!(Self::gen_downcast_err_str(type_str, key)),
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
            None => panic!(Self::gen_downcast_err_str(type_str, key)),
            Some(param) => param,
        }
    }

    fn gen_downcast_err_str(type_str: &str, key: &str) -> String {
        format!("Tried to cast {} of key {} but failed! (Don't know how to print name of downcast type...)", type_str, key)
    }
}

fn conv_err_to_str_with_when(err: Box<dyn Error>, when_str: &str) -> String {
    format!("When:{}\n{}", when_str, conv_err_to_str(err))
}

fn conv_err_to_str(err: Box<dyn Error>) -> String {
    format!("Description: {}\nSource: {:?}", err, err.source())
}
