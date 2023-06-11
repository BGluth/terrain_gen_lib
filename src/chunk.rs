//! Core logic for working with chunk coordinates.

use std::hash::Hash;

pub trait ChunkCoord: Eq + Hash + Sized {
    fn get_neighboring_chunk_coords_in_radius(&self, r: usize) -> impl Iterator<Item = Self>;

    // Might end up not using a float...
    fn scale_coord(&self, factor: f32) -> Self;

    fn get_scale(&self) -> usize;
}

/// Get the super chunk coordinate that this sub-chunk belongs to.
///
/// For example, if we have the coord `(3,3)` at 2x and we want to know the 3x coordiate that it is child of, we would get back `(2,2)`.
pub(crate) fn get_super_chunk_coord_of_coord<T: ChunkCoord>(
    _coord: &T,
    coord_scale_factor: usize,
    target_coord_scale_factor: usize,
) -> T {
    debug_assert!(coord_scale_factor <= target_coord_scale_factor);

    todo!()
}

pub(crate) fn get_sub_chunk_coord_of_coord<T: ChunkCoord>(
    _coord: &T,
    coord_scale_factor: usize,
    target_coord_scale_factor: usize,
) -> impl Iterator<Item = T> {
    debug_assert!(coord_scale_factor >= target_coord_scale_factor);

    // TODO
    std::iter::empty()
}
