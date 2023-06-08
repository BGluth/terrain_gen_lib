# Overview

A general purpose terrain generation library. (Will write a better description once it's more developed).

Essentially this is a scheduler that runs user defined generation passes over all "chunks" of the terrain. Each pass either creates or modifies existing data associated with a chunk, and passes may have dependencies one another before they can be run. The end result is a dataset that can be used to generate terrain for a specific game/simulation/etc.

The type used as a chunk coordinate is able to be defined by the library user, which gives a lot of flexibility over what type of terrain can be generated (eg. Vector3 for traditional 3D terrain, spherical coordinates for spherical planets, etc.).

# License

Dual licensed under either:

    Apache License, Version 2.0
    MIT license

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in time by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
