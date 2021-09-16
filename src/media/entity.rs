use crate::{v4l_sys::*, wrap_c_str_slice_until_nul};

// TODO: How to best represent these?
// /// Holds the `dev` union member from [`media_entity_desc`]
// /// for supported entity types.
// #[derive(Clone, Copy, Debug)]
// pub struct NodeSpecification {
//     major: u32,
//     minor: u32,
// }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityType {
    #[doc(alias = "MEDIA_ENT_F_UNKNOWN")]
    Unknown,
    #[doc(alias = "MEDIA_ENT_F_V4L2_SUBDEV_UNKNOWN")]
    Subdev,
    #[doc(alias = "MEDIA_ENT_F_IO_V4L")]
    Dev {
        major: u32,
        minor: u32,
    },
    Alsa {
        card: u32,
        device: u32,
        subdevice: u32,
    },
    Fb {
        major: u32,
        minor: u32,
    },
    Dvb(i32),

    // TODO: Add all ENT_F entity functions
    #[doc(alias = "MEDIA_ENT_F_CAM_SENSOR")]
    Camera,
}

// TODO: Make proper helpers for these integer references!
// TODO: Rename to "Entity"? Check conventions
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[doc(alias = "media_entity_desc")]
pub struct EntityDesc {
    pub id: u32,
    pub name: String,
    // TODO: This is loosely linked to devinfo
    pub type_: u32,
    // pub node_spec: Option<NodeSpecification>,
    pub entity_type: EntityType,
    pub revision: u32,
    pub flags: u32,
    pub group_id: u32,
    // TODO: Immediately query and provide the pads and links?
    pub pads: u16,
    pub links: u16,
}

impl From<media_entity_desc> for EntityDesc {
    fn from(desc: media_entity_desc) -> Self {
        Self {
            id: desc.id,
            name: wrap_c_str_slice_until_nul(&desc.name)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            type_: desc.type_,
            entity_type: match desc.type_ {
                MEDIA_ENT_F_UNKNOWN => EntityType::Unknown,
                MEDIA_ENT_F_V4L2_SUBDEV_UNKNOWN => EntityType::Subdev,
                // TODO: More flags. Add extra member in EntityType::Dev
                // to distinguish the variants?
                MEDIA_ENT_F_IO_V4L => unsafe {
                    EntityType::Dev {
                        major: desc.__bindgen_anon_1.dev.major,
                        minor: desc.__bindgen_anon_1.dev.minor,
                    }
                },
                MEDIA_ENT_F_CAM_SENSOR => EntityType::Camera,
                _ => todo!("Entity function {:x} not implemented", desc.type_),
            },
            revision: desc.revision,
            flags: desc.flags,
            group_id: desc.group_id,
            pads: desc.pads,
            links: desc.links,
        }
    }
}
