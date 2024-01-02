pub mod area;
pub mod generator;

use super::{
    action::BlockAction,
    block::{Block, BlockLight},
};
use crate::shared::{
    bound::{Aabb, BoundingSphere},
    utils,
};
use nalgebra::{point, Point3, Vector3};
use std::{
    array, mem,
    ops::{BitOrAssign, Index, IndexMut},
};

#[derive(Default)]
pub struct Chunk {
    blocks: DataStore<Block>,
    non_air_count: u16,
    glowing_count: u16,
}

impl Chunk {
    pub const DIM: usize = 16;

    fn from_fn<F: FnMut(Point3<u8>) -> Block>(mut f: F) -> Self {
        let mut non_air_count = 0;
        let mut glowing_count = 0;
        Self {
            blocks: DataStore::from_fn(|coords| {
                let block = f(coords);
                non_air_count += (block != Block::Air) as u16;
                glowing_count += block.data().is_glowing() as u16;
                block
            }),
            non_air_count,
            glowing_count,
        }
    }

    pub fn blocks(&self) -> impl Iterator<Item = (Point3<u8>, &Block)> {
        self.blocks.values()
    }

    pub fn is_empty(&self) -> bool {
        self.non_air_count == 0
    }

    pub fn is_glowing(&self) -> bool {
        self.glowing_count != 0
    }

    pub fn apply(&mut self, coords: Point3<u8>, action: BlockAction) -> bool {
        let block = &mut self.blocks[coords];
        let prev = *block;
        if block.apply(action) {
            let curr = *block;
            self.adjust_counts(prev, curr);
            true
        } else {
            false
        }
    }

    pub fn apply_unchecked(&mut self, coords: Point3<u8>, action: BlockAction) {
        let block = &mut self.blocks[coords];
        let prev = *block;
        block.apply_unchecked(action);
        let curr = *block;
        self.adjust_counts(prev, curr);
    }

    fn adjust_counts(&mut self, prev: Block, curr: Block) {
        self.non_air_count -= (prev != Block::Air) as u16;
        self.non_air_count += (curr != Block::Air) as u16;
        self.glowing_count -= prev.data().is_glowing() as u16;
        self.glowing_count += curr.data().is_glowing() as u16;
    }

    pub fn points() -> impl Iterator<Item = Point3<u8>> {
        (0..Self::DIM).flat_map(|x| {
            (0..Self::DIM).flat_map(move |y| (0..Self::DIM).map(move |z| point![x, y, z].cast()))
        })
    }

    pub fn bounding_box(coords: Point3<i32>) -> Aabb {
        Aabb::new(
            utils::coords((coords, Default::default())).cast(),
            Vector3::repeat(Self::DIM).cast(),
        )
    }

    pub fn bounding_sphere(coords: Point3<i32>) -> BoundingSphere {
        Self::bounding_box(coords).into()
    }

    pub fn center(coords: Point3<i32>) -> Point3<f32> {
        Self::bounding_sphere(coords).center
    }
}

impl Index<Point3<u8>> for Chunk {
    type Output = Block;

    fn index(&self, coords: Point3<u8>) -> &Self::Output {
        &self.blocks[coords]
    }
}

#[repr(align(16))]
#[derive(Default)]
pub struct ChunkLight {
    lights: DataStore<BlockLight>,
    non_zero_count: u16,
}

impl ChunkLight {
    pub fn set(&mut self, coords: Point3<u8>, value: BlockLight) -> bool {
        let prev = mem::replace(&mut self.lights[coords], value);
        if prev == value {
            false
        } else {
            if prev == Default::default() {
                self.non_zero_count += 1;
            } else if value == Default::default() {
                self.non_zero_count -= 1;
            }
            true
        }
    }

    pub fn is_empty(&self) -> bool {
        self.non_zero_count == 0
    }
}

impl Index<Point3<u8>> for ChunkLight {
    type Output = BlockLight;

    fn index(&self, coords: Point3<u8>) -> &Self::Output {
        &self.lights[coords]
    }
}

impl BitOrAssign<BlockLight> for ChunkLight {
    fn bitor_assign(&mut self, light: BlockLight) {
        let mask = bytemuck::cast::<_, u128>([light; DataStore::CHUNK_SIZE]);

        for lights in self.lights.array_chunks_mut() {
            *bytemuck::cast_mut::<_, u128>(lights) |= mask;
        }

        if light != Default::default() {
            self.non_zero_count = Chunk::DIM.pow(3) as u16;
        }
    }
}

#[derive(Default)]
struct DataStore<T>([[[T; Chunk::DIM]; Chunk::DIM]; Chunk::DIM]);

impl<T> DataStore<T> {
    fn from_fn<F: FnMut(Point3<u8>) -> T>(mut f: F) -> Self {
        Self(array::from_fn(|x| {
            array::from_fn(|y| array::from_fn(|z| f(point![x, y, z].cast())))
        }))
    }

    fn values(&self) -> impl Iterator<Item = (Point3<u8>, &T)> {
        self.0.iter().enumerate().flat_map(move |(x, values)| {
            values.iter().enumerate().flat_map(move |(y, values)| {
                values
                    .iter()
                    .enumerate()
                    .map(move |(z, value)| (point![x, y, z].cast(), value))
            })
        })
    }
}

impl DataStore<BlockLight> {
    const CHUNK_SIZE: usize = mem::size_of::<u128>() / mem::size_of::<BlockLight>();

    fn array_chunks_mut(&mut self) -> impl Iterator<Item = &mut [BlockLight; Self::CHUNK_SIZE]> {
        self.0
            .iter_mut()
            .flatten()
            .flat_map(bytemuck::cast_mut::<_, [_; Chunk::DIM / Self::CHUNK_SIZE]>)
    }
}

impl<T> Index<Point3<u8>> for DataStore<T> {
    type Output = T;

    fn index(&self, coords: Point3<u8>) -> &Self::Output {
        &self.0[coords.x as usize][coords.y as usize][coords.z as usize]
    }
}

impl<T> IndexMut<Point3<u8>> for DataStore<T> {
    fn index_mut(&mut self, coords: Point3<u8>) -> &mut Self::Output {
        &mut self.0[coords.x as usize][coords.y as usize][coords.z as usize]
    }
}
