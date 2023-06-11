use std::{any::Any, cell::UnsafeCell, collections::HashMap};

use crate::{
    chunk::ChunkCoord,
    GeneratorState::{DataId, PassId},
};

type PassDepLookup = HashMap<PassId, Vec<StaticPassDependency>>;

#[derive(Debug)]
pub enum DataAccessType {
    Read,
    Write,
}

enum DataAccessTracker {
    Read(usize),
    Write,
}

pub(crate) enum StaticPassDependency {
    DataAccess(StaticPassRunPrereq),
    PrereqPass(PassId),
}

/// Pass dep info that is updated as a chunk is generated. Created from `StaticPassDependency`.
enum ActivePassDependency {
    DataAccess(ActivePassRunPrereq),
    PrereqPass(PassId),
}

struct PassDataReq {
    d_id: DataId,
    access_type: DataAccessType,
}

pub(crate) struct ChunkPassState {
    raw_pass_data: HashMap<DataId, UnsafeCell<Box<dyn Any>>>,
    active_data_locks: HashMap<DataId, DataAccessTracker>,
    pending_pass_ids: Vec<PassId>,
    inactive_pass_ids: Vec<PassId>,
}

impl ChunkPassState {
    /// Release resources and update chunk state from the pass completing.
    pub(crate) fn handle_pass_completed(&mut self, _p_id: PassId) {
        todo!()
    }

    /// Schedules all passes that are able to run. This causes the chunk state to lock resources.
    pub(crate) fn schedule_all_passes_that_are_able_to_run<C: ChunkCoord>(
        &mut self,
    ) -> impl Iterator<Item = ScheduledPassInfo<C>> {
        // TODO
        std::iter::empty()
    }
}

pub(crate) struct ScheduledPassInfo<C: ChunkCoord> {
    raw_data: *mut Box<dyn Any>,
    coord: C,
}

pub(crate) struct StaticPassRunPrereq {
    common: PassRunPrereqCommon,
}

struct ActivePassRunPrereq {
    common: PassRunPrereqCommon,
}

struct PassRunPrereqCommon {
    d_id: DataId,
    access_type: DataAccessType,
    coord_scale: usize,
}
