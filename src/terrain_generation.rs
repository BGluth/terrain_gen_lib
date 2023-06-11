use std::collections::HashMap;

use crate::{
    chunk::ChunkCoord,
    pass_dependencies::{ChunkPassState, DataAccessType, StaticPassDependency},
    utils::IdGenerator,
    GeneratorState::PassId,
};

pub struct GenerationState<C: ChunkCoord> {
    static_pass_deps: HashMap<PassId, Vec<StaticPassDependency>>,
    chunk_pass_state: HashMap<C, ChunkPassState>,

    reged_passes: Vec<PassInfo>,
    reged_data: Vec<DataInfo>,

    chunk_size: usize,
}

pub struct GenerationStateBuilder {
    reged_passes: Vec<PassInfoBuilder>,
    reged_data: Vec<DataInfoBuilder>,

    // ... Enforce to be a power of 2?
    base_chunk_dim: usize,
}

impl GenerationStateBuilder {
    pub fn set_chunk_size(self, c_size: usize) -> Self {
        todo!()
    }

    pub fn build<C: ChunkCoord>(self) -> GenerationState<C> {
        todo!()
    }
}

pub struct PassDataAccessReg {
    d_str: String,
    access_type: DataAccessType,
}

struct PassInfo {
    name: String,
    deps: Vec<StaticPassDependency>,
}

struct DataInfo {
    name: String,
    chunk_factor_scale: usize,
}

pub struct PassInfoBuilder {
    name: String,
}

impl PassInfoBuilder {
    pub fn new(name: String) -> Self {
        todo!()
    }

    pub fn add_data_dep(self, d_reg: PassDataAccessReg) -> Self {
        todo!()
    }

    pub fn add_prereq_pass(self, p_name: String) -> Self {
        todo!()
    }

    fn build(self) -> PassInfo {
        todo!()
    }
}

pub struct DataInfoBuilder {
    name: String,
}

impl DataInfoBuilder {
    pub fn new(name: String) -> Self {
        todo!()
    }

    fn build(self) -> DataInfo {
        todo!()
    }
}
