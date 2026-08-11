use std::collections::HashMap;

use crate::{
    media::encoder::EncodedFrame,
    protocol::video::VideoPacket,
};

struct FrameAssembly {
    timestamp: u64,
    keyframe: bool,
    codec: crate::protocol::video::VideoCodec,
    width: u32,
    height: u32,
    packet_total: u16,
    packets: Vec<Option<Vec<u8>>>,
}

pub struct VideoReassembler {
    frames: HashMap<u64, FrameAssembly>,
}

impl VideoReassembler {
    pub fn new() -> Self {
        Self {
            frames: HashMap::new(),
        }
    }

    pub fn push(&mut self, packet: VideoPacket) -> Option<EncodedFrame> {
        let frame_id = packet.frame_id;

        // Create the assembly for this frame if we haven't
        // received any packets for it yet.
        let assembly = self.frames.entry(frame_id).or_insert_with(|| {
            FrameAssembly {
                timestamp: packet.timestamp,
                keyframe: packet.keyframe,
                codec: packet.codec,
                width: packet.width,
                height: packet.height,
                packet_total: packet.packet_total,
                packets: vec![None; packet.packet_total as usize],
            }
        });

        // Make sure this packet belongs to the same frame layout.
        if assembly.packet_total != packet.packet_total
            || assembly.width != packet.width
            || assembly.height != packet.height
            || assembly.codec != packet.codec
        {
            self.frames.remove(&frame_id);
            return None;
        }

        let index = packet.packet_index as usize;

        if index >= assembly.packets.len() {
            self.frames.remove(&frame_id);
            return None;
        }

        // Ignore duplicate fragments.
        if assembly.packets[index].is_some() {
            return None;
        }

        assembly.packets[index] = Some(packet.data);

        // Don't assemble until every fragment arrived.
        if assembly.packets.iter().any(|packet| packet.is_none()) {
            return None;
        }

        // Remove the completed frame from the map.
        let assembly = self.frames.remove(&frame_id)?;

        let mut data = Vec::new();

        for packet in assembly.packets {
            data.extend(packet?);
        }

        Some(EncodedFrame {
            sequence: frame_id,
            timestamp: assembly.timestamp,
            keyframe: assembly.keyframe,
            codec: assembly.codec,
            width: assembly.width as usize,
            height: assembly.height as usize,
            data,
        })
    }
}