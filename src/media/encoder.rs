use std::time::Instant;

use openh264::{
    encoder::Encoder,
    formats::{RgbSliceU8, YUVBuffer},
};

use crate::{
    media::frame::RawFrame,
    protocol::video::{VideoCodec, VideoPacket},
};

/// Keep enough room for the QUIC datagram header.
///
/// The exact value should eventually be based on the QUIC path MTU.
/// 1150 is a conservative starting point.
pub const MAX_VIDEO_PAYLOAD: usize = 1150;

pub struct EncodedFrame {
    pub sequence: u64,
    pub timestamp: u64,
    pub keyframe: bool,
    pub codec: VideoCodec,
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

pub struct VideoEncoder {
    encoder: Encoder,
}

impl VideoEncoder {
    pub fn new() -> Result<Self, openh264::Error> {
        Ok(Self {
            encoder: Encoder::new()?,
        })
    }

    pub fn encode_frame(
        &mut self,
        frame: &RawFrame,
    ) -> Result<EncodedFrame, openh264::Error> {
        // scrap gives us BGRA.
        //
        // openh264::formats::RgbSliceU8 expects RGB.
        let rgb: Vec<u8> = frame
            .data
            .chunks_exact(4)
            .flat_map(|bgra| {
                [
                    bgra[2], // R
                    bgra[1], // G
                    bgra[0], // B
                ]
            })
            .collect();

        let rgb = RgbSliceU8::new(
            &rgb,
            (frame.width as usize, frame.height as usize),
        );

        let yuv = YUVBuffer::from_rgb8_source(rgb);

        let bitstream = self.encoder.encode(&yuv)?;

        let mut data = Vec::new();

        bitstream.write_vec(&mut data);

        Ok(EncodedFrame {
            sequence: frame.sequence,
            timestamp: frame.timestamp,
            // We will fix this properly once we handle
            // openh264's actual frame type / NAL information.
            keyframe: false,

            codec: VideoCodec::H264,

            width: frame.width,
            height: frame.height,

            data,
        })
    }

    pub fn encode_and_packetize(
        &mut self,
        uid: u64,
        frame: &RawFrame,
    ) -> Result<Vec<VideoPacket>, openh264::Error> {
        let encoded = self.encode_frame(frame)?;

        Ok(packetize(uid, &encoded))
    }
}

pub fn packetize(
    uid: u64,
    frame: &EncodedFrame,
) -> Vec<VideoPacket> {
    let packet_total = frame
        .data
        .len()
        .div_ceil(MAX_VIDEO_PAYLOAD).
        max(1);

    debug_assert!(packet_total <= u16::MAX as usize);

    frame
        .data
        .chunks(MAX_VIDEO_PAYLOAD)
        .enumerate()
        .map(|(packet_index, data)| {
            VideoPacket {
                uid,

                frame_id: frame.sequence,

                packet_index: packet_index as u16,
                packet_total: packet_total as u16,

                codec: frame.codec,

                width: frame.width as u32,
                height: frame.height as u32,

                timestamp: frame.timestamp,

                keyframe: frame.keyframe,

                data: data.to_vec(),
            }
        })
        .collect()
}