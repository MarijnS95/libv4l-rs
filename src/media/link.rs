use std::fmt;

use bitflags::bitflags;
use v4l2_sys::*;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[doc(alias = "MEDIA_PAD_FL")]
    pub struct PadFlags: u32 {
        #[doc(alias = "MEDIA_PAD_FL_SINK")]
        const SINK = MEDIA_PAD_FL_SINK;
        #[doc(alias = "MEDIA_PAD_FL_SOURCE")]
        const SOURCE = MEDIA_PAD_FL_SOURCE;
        #[doc(alias = "MEDIA_PAD_FL_MUST_CONNECT")]
        const MUST_CONNECT = MEDIA_PAD_FL_MUST_CONNECT;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[doc(alias = "media_pad_desc")]
pub struct Pad {
    pub entity: u32, // TODO: make this an Arc?
    pub index: u16,
    pub flags: PadFlags,
}

impl From<media_pad_desc> for Pad {
    fn from(desc: media_pad_desc) -> Self {
        Self {
            entity: desc.entity,
            index: desc.index,
            flags: PadFlags::from_bits_retain(desc.flags),
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    #[doc(alias = "MEDIA_LNK_FL")]
    pub struct LinkFlags: u32 {
        #[doc(alias = "MEDIA_LNK_FL_ENABLED")]
        const ENABLED = MEDIA_LNK_FL_ENABLED;
        #[doc(alias = "MEDIA_LNK_FL_IMMUTABLE")]
        const IMMUTABLE = MEDIA_LNK_FL_IMMUTABLE;
        #[doc(alias = "MEDIA_LNK_FL_DYNAMIC")]
        const DYNAMIC = MEDIA_LNK_FL_DYNAMIC;
    }
}

impl fmt::Debug for LinkFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Does this print the "unknown" `MEDIA_LNK_FL_LINK_TYPE` bits?
        write!(f, "LinkFlags({:?}, {:?})", self.0, self.link_type())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[doc(alias = "MEDIA_LNK_FL_LINK_TYPE")]
pub enum LinkType {
    #[doc(alias = "MEDIA_LNK_FL_DATA_LINK")]
    Data,
    #[doc(alias = "MEDIA_LNK_FL_INTERFACE_LINK")]
    Interface,
    #[doc(alias = "MEDIA_LNK_FL_ANCILLARY_LINK")]
    Ancillary,
}

impl LinkFlags {
    pub fn link_type(self) -> LinkType {
        match self.bits() & MEDIA_LNK_FL_LINK_TYPE {
            MEDIA_LNK_FL_DATA_LINK => LinkType::Data,
            MEDIA_LNK_FL_INTERFACE_LINK => LinkType::Interface,
            MEDIA_LNK_FL_ANCILLARY_LINK => LinkType::Ancillary,
            x => unimplemented!("Unknown link type {:x}", x),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[doc(alias = "media_link_desc")]
pub struct Link {
    pub source: Pad,
    pub sink: Pad,
    pub flags: LinkFlags,
}

impl From<media_link_desc> for Link {
    fn from(desc: media_link_desc) -> Self {
        Self {
            source: desc.source.into(),
            sink: desc.sink.into(),
            flags: LinkFlags::from_bits_retain(desc.flags),
        }
    }
}
