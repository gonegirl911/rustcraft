use crate::client::game::scene::world::block::BlockVertex;
use bitvec::prelude::*;
use enum_map::{enum_map, Enum, EnumMap};
use nalgebra::{point, Point2, Point3};
use once_cell::sync::Lazy;

#[repr(u8)]
#[derive(Clone, Copy, Default, Enum)]
pub enum Block {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
    Bedrock,
}

impl Block {
    pub fn vertices(
        self,
        coords: Point3<u8>,
        area: BlockArea,
    ) -> impl Iterator<Item = BlockVertex> {
        BLOCK_SIDE_ATLAS_COORDS[self]
            .iter()
            .flat_map(move |side_atlas_coords| {
                SIDE_DELTAS
                    .iter()
                    .filter_map(move |(side, delta)| {
                        (!unsafe { area.get_unchecked(*delta) }).then_some(side)
                    })
                    .flat_map(move |side| {
                        let corner_vertex_coords = &SIDE_CORNER_VERTEX_COORDS[side];
                        let atlas_coords = side_atlas_coords[side];

                        let corner_aos = enum_map! {
                            corner => Self::ambient_occlusion(side, corner, area),
                        };
                        let [a00, a10, a11, a01] = [
                            corner_aos[Corner::LowerLeft],
                            corner_aos[Corner::LowerRight],
                            corner_aos[Corner::UpperRight],
                            corner_aos[Corner::UpperLeft],
                        ];
                        let indices = if a00 + a11 > a10 + a01 {
                            FLIPPED_INDICES
                        } else {
                            INDICES
                        };

                        indices.into_iter().map(move |corner| {
                            BlockVertex::new(
                                coords + corner_vertex_coords[corner].coords,
                                CORNER_TEX_COORDS[corner],
                                atlas_coords,
                                corner_aos[corner],
                            )
                        })
                    })
            })
    }

    pub fn is_opaque(self) -> bool {
        !self.is_transparent()
    }

    fn is_transparent(self) -> bool {
        matches!(self, Block::Air)
    }

    fn ambient_occlusion(side: Side, corner: Corner, area: BlockArea) -> u8 {
        let component_deltas = SIDE_CORNER_COMPONENT_DELTAS[side][corner]
            .map(|_, delta| unsafe { area.get_unchecked(delta) });

        let [edge1, edge2, corner] = [
            component_deltas[Component::Edge1],
            component_deltas[Component::Edge2],
            component_deltas[Component::Corner],
        ];

        if edge1 && edge2 {
            0
        } else {
            3 - (edge1 as u8 + edge2 as u8 + corner as u8)
        }
    }
}

#[derive(Clone, Copy)]
pub struct BlockArea(BitArr!(for Self::DIM * Self::DIM * Self::DIM, in u32));

impl BlockArea {
    const DIM: usize = (Self::UPPER_BOUND - Self::LOWER_BOUND + 1) as usize;
    const LOWER_BOUND: i8 = -1;
    const UPPER_BOUND: i8 = 1;

    pub fn from_fn<F: FnMut(Point3<i8>) -> bool>(mut f: F) -> Self {
        let mut data = BitArray::ZERO;
        for x in Self::LOWER_BOUND..=Self::UPPER_BOUND {
            for y in Self::LOWER_BOUND..=Self::UPPER_BOUND {
                for z in Self::LOWER_BOUND..=Self::UPPER_BOUND {
                    let coords = point![x, y, z];
                    unsafe {
                        data.set_unchecked(Self::index_unchecked(coords), f(coords));
                    }
                }
            }
        }
        Self(data)
    }

    unsafe fn get_unchecked(&self, coords: Point3<i8>) -> bool {
        unsafe { *self.0.get_unchecked(Self::index_unchecked(coords)) }
    }

    unsafe fn index_unchecked(coords: Point3<i8>) -> usize {
        let coords = coords.map(|c| (c - Self::LOWER_BOUND) as usize);
        coords.x * Self::DIM.pow(2) + coords.y * Self::DIM + coords.z
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Enum)]
enum Side {
    Front,
    Right,
    Back,
    Left,
    Up,
    Down,
}

#[repr(u8)]
#[derive(Clone, Copy, Enum)]
enum Corner {
    LowerLeft,
    LowerRight,
    UpperRight,
    UpperLeft,
}

#[repr(u8)]
#[derive(Clone, Copy, Enum)]
enum Component {
    Edge1,
    Edge2,
    Corner,
}

#[allow(clippy::type_complexity)]
static BLOCK_SIDE_ATLAS_COORDS: Lazy<EnumMap<Block, Option<EnumMap<Side, Point2<u8>>>>> =
    Lazy::new(|| {
        enum_map! {
            Block::Air => None,
            Block::Grass => Some(enum_map! {
                Side::Front | Side::Right | Side::Back | Side::Left => point![3, 0],
                Side::Up => point![0, 0],
                Side::Down => point![2, 0],
            }),
            Block::Dirt => Some(enum_map! { _ => point![2, 0] }),
            Block::Stone => Some(enum_map! { _ => point![1, 0] }),
            Block::Bedrock => Some(enum_map! { _ => point![1, 1] }),
        }
    });

static SIDE_CORNER_SIDES: Lazy<EnumMap<Side, EnumMap<Corner, [Side; 2]>>> = Lazy::new(|| {
    enum_map! {
        Side::Front => enum_map! {
            Corner::LowerLeft => [Side::Left, Side::Down],
            Corner::LowerRight => [Side::Right, Side::Down],
            Corner::UpperRight => [Side::Right, Side::Up],
            Corner::UpperLeft => [Side::Left, Side::Up],
        },
        Side::Right => enum_map! {
            Corner::LowerLeft => [Side::Front, Side::Down],
            Corner::LowerRight => [Side::Back, Side::Down],
            Corner::UpperRight => [Side::Back, Side::Up],
            Corner::UpperLeft => [Side::Front, Side::Up],
        },
        Side::Back => enum_map! {
            Corner::LowerLeft => [Side::Right, Side::Down],
            Corner::LowerRight => [Side::Left, Side::Down],
            Corner::UpperRight => [Side::Left, Side::Up],
            Corner::UpperLeft => [Side::Right, Side::Up],
        },
        Side::Left => enum_map! {
            Corner::LowerLeft => [Side::Back, Side::Down],
            Corner::LowerRight => [Side::Front, Side::Down],
            Corner::UpperRight => [Side::Front, Side::Up],
            Corner::UpperLeft => [Side::Back, Side::Up],
        },
        Side::Up => enum_map! {
            Corner::LowerLeft => [Side::Left, Side::Front],
            Corner::LowerRight => [Side::Right, Side::Front],
            Corner::UpperRight => [Side::Right, Side::Back],
            Corner::UpperLeft => [Side::Left, Side::Back],
        },
        Side::Down => enum_map! {
            Corner::LowerLeft => [Side::Left, Side::Back],
            Corner::LowerRight => [Side::Right, Side::Back],
            Corner::UpperRight => [Side::Right, Side::Front],
            Corner::UpperLeft => [Side::Left, Side::Front],
        },
    }
});

static SIDE_DELTAS: Lazy<EnumMap<Side, Point3<i8>>> = Lazy::new(|| {
    enum_map! {
        Side::Front => point![0, 0, -1],
        Side::Right => point![1, 0, 0],
        Side::Back => point![0, 0, 1],
        Side::Left => point![-1, 0, 0],
        Side::Up => point![0, 1, 0],
        Side::Down => point![0, -1, 0],
    }
});

static SIDE_CORNER_VERTEX_COORDS: Lazy<EnumMap<Side, EnumMap<Corner, Point3<u8>>>> =
    Lazy::new(|| {
        SIDE_CORNER_SIDES.map(|s1, corner_sides| {
            corner_sides.map(|_, [s2, s3]| {
                (SIDE_DELTAS[s1] + SIDE_DELTAS[s2].coords + SIDE_DELTAS[s3].coords)
                    .map(|c| (c + 1) as u8 / 2)
            })
        })
    });

#[allow(clippy::type_complexity)]
static SIDE_CORNER_COMPONENT_DELTAS: Lazy<
    EnumMap<Side, EnumMap<Corner, EnumMap<Component, Point3<i8>>>>,
> = Lazy::new(|| {
    SIDE_CORNER_SIDES.map(|s1, corner_sides| {
        corner_sides.map(|_, [s2, s3]| {
            let corner = SIDE_DELTAS[s1] + SIDE_DELTAS[s2].coords + SIDE_DELTAS[s3].coords;
            enum_map! {
                Component::Edge1 => corner - SIDE_DELTAS[s3].coords,
                Component::Edge2 => corner - SIDE_DELTAS[s2].coords,
                Component::Corner => corner,
            }
        })
    })
});

static CORNER_TEX_COORDS: Lazy<EnumMap<Corner, Point2<u8>>> = Lazy::new(|| {
    enum_map! {
        Corner::LowerLeft => point![0, 1],
        Corner::LowerRight => point![1, 1],
        Corner::UpperRight => point![1, 0],
        Corner::UpperLeft => point![0, 0],
    }
});

const INDICES: [Corner; 6] = [
    Corner::LowerLeft,
    Corner::LowerRight,
    Corner::UpperLeft,
    Corner::LowerRight,
    Corner::UpperRight,
    Corner::UpperLeft,
];

const FLIPPED_INDICES: [Corner; 6] = [
    Corner::LowerLeft,
    Corner::LowerRight,
    Corner::UpperRight,
    Corner::LowerLeft,
    Corner::UpperRight,
    Corner::UpperLeft,
];
