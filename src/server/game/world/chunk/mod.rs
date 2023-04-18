pub mod generator;
pub mod light;

use self::light::ChunkAreaLight;
use super::{
    action::BlockAction,
    block::{Block, BlockArea},
    World,
};
use crate::{
    client::game::world::BlockVertex,
    shared::{
        bound::{Aabb, BoundingSphere},
        utils,
    },
};
use bitvec::BitArr;
use nalgebra::{point, vector, Point3, Vector3};
use std::{
    array, mem,
    ops::{Index, IndexMut, Range},
};

#[repr(align(16))]
#[derive(Clone, Default)]
pub struct Chunk([[[Block; Self::DIM]; Self::DIM]; Self::DIM]);

impl Chunk {
    pub const DIM: usize = 16;

    pub fn from_fn<F: FnMut(Point3<u8>) -> Block>(mut f: F) -> Self {
        Self(array::from_fn(|x| {
            array::from_fn(|y| array::from_fn(|z| f(point![x, y, z].cast())))
        }))
    }

    pub fn vertices<'a>(
        &'a self,
        area: &'a ChunkArea,
        area_light: &'a ChunkAreaLight,
    ) -> impl Iterator<Item = BlockVertex> + 'a {
        self.blocks().flat_map(|(coords, block)| {
            block
                .vertices(
                    coords,
                    area.block_area(coords),
                    area_light.block_area_light(coords),
                )
                .into_iter()
                .flatten()
        })
    }

    pub fn apply(&mut self, coords: Point3<u8>, action: &BlockAction) -> bool {
        self[coords].apply(action)
    }

    pub fn is_empty(&self) -> bool {
        let expected = unsafe { mem::transmute([Block::Air; Self::DIM]) };
        self.0
            .iter()
            .flatten()
            .all(|blocks| *unsafe { mem::transmute::<_, &u128>(blocks) } == expected)
    }

    pub fn bounding_box(coords: Point3<i32>) -> Aabb {
        Aabb::new(
            World::coords(coords, Default::default()).cast(),
            Vector3::from_element(Self::DIM).cast(),
        )
    }

    pub fn bounding_sphere(coords: Point3<i32>) -> BoundingSphere {
        Self::bounding_box(coords).into()
    }

    fn blocks(&self) -> impl Iterator<Item = (Point3<u8>, &Block)> + '_ {
        self.0.iter().zip(0..).flat_map(move |(blocks, x)| {
            blocks.iter().zip(0..).flat_map(move |(blocks, y)| {
                blocks
                    .iter()
                    .zip(0..)
                    .map(move |(block, z)| (point![x, y, z], block))
            })
        })
    }
}

impl Index<Point3<u8>> for Chunk {
    type Output = Block;

    fn index(&self, coords: Point3<u8>) -> &Self::Output {
        &self.0[coords.x as usize][coords.y as usize][coords.z as usize]
    }
}

impl IndexMut<Point3<u8>> for Chunk {
    fn index_mut(&mut self, coords: Point3<u8>) -> &mut Self::Output {
        &mut self.0[coords.x as usize][coords.y as usize][coords.z as usize]
    }
}

#[derive(Default)]
pub struct ChunkArea(BitArr!(for Self::DIM * Self::DIM * Self::DIM, in usize));

impl ChunkArea {
    pub const DIM: usize = Chunk::DIM + Self::PADDING * 2;
    pub const PADDING: usize = BlockArea::PADDING;
    const AXIS_RANGE: Range<i8> = -(Self::PADDING as i8)..(Chunk::DIM + Self::PADDING) as i8;

    pub fn from_fn<F: FnMut(Vector3<i8>) -> bool>(mut f: F) -> Self {
        let mut value = Self::default();
        for delta in Self::deltas() {
            value.set(delta, f(delta));
        }
        value
    }

    fn block_area(&self, coords: Point3<u8>) -> BlockArea {
        let coords = coords.coords.cast();
        BlockArea::from_fn(|delta| self.is_opaque(coords + delta))
    }

    fn is_opaque(&self, delta: Vector3<i8>) -> bool {
        unsafe { *self.0.get_unchecked(Self::index(delta)) }
    }

    fn set(&mut self, delta: Vector3<i8>, is_opaque: bool) {
        unsafe {
            self.0.set_unchecked(Self::index(delta), is_opaque);
        }
    }

    pub fn chunk_deltas() -> impl Iterator<Item = Vector3<i32>> {
        let chunk_padding = utils::div_ceil(Self::PADDING, Chunk::DIM) as i32;
        (-chunk_padding..1 + chunk_padding).flat_map(move |dx| {
            (-chunk_padding..1 + chunk_padding).flat_map(move |dy| {
                (-chunk_padding..1 + chunk_padding).map(move |dz| vector![dx, dy, dz])
            })
        })
    }

    fn deltas() -> impl Iterator<Item = Vector3<i8>> {
        Self::AXIS_RANGE.flat_map(|dx| {
            Self::AXIS_RANGE.flat_map(move |dy| Self::AXIS_RANGE.map(move |dz| vector![dx, dy, dz]))
        })
    }

    fn index(delta: Vector3<i8>) -> usize {
        assert!(
            Self::AXIS_RANGE.contains(&delta.x)
                && Self::AXIS_RANGE.contains(&delta.y)
                && Self::AXIS_RANGE.contains(&delta.z)
        );
        unsafe { Self::index_unchecked(delta) }
    }

    unsafe fn index_unchecked(delta: Vector3<i8>) -> usize {
        let idx = delta.map(|c| (c + Self::PADDING as i8) as usize);
        idx.x * Self::DIM.pow(2) + idx.y * Self::DIM + idx.z
    }
}
