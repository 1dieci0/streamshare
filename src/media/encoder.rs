use tokio::sync::mpsc::Receiver;
use openh264::{
    OpenH264API,
    encoder::{Encoder, EncoderConfig, IntraFramePeriod},
    formats::{RgbSliceU8, YUVBuffer},
};

use crate::{
    client::command::EncoderCommand, media::frame::RawFrame, protocol::video::{VideoCodec, VideoPacket},
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
    force_keyframe: bool,
    encoder_rx: Receiver<EncoderCommand>,
}

impl VideoEncoder {
    pub fn new(encoder_rx: Receiver<EncoderCommand>) -> Result<Self, openh264::Error> {
        let api = OpenH264API::from_source();
        let config = EncoderConfig::new().intra_frame_period(IntraFramePeriod::from_num_frames(60));
        Ok(Self {
            encoder: Encoder::with_api_config(api, config)?,
            force_keyframe: false,
            encoder_rx,
        })
    }

    pub fn encode_frame(
        &mut self,
        frame: &RawFrame,
    ) -> Result<EncodedFrame, openh264::Error> {
        // scrap gives us BGRA.
        //
        // openh264::formats::RgbSliceU8 expects RGB.

        while let Ok(command) = self.encoder_rx.try_recv(){

            println!("got something");

            match command{
                EncoderCommand::ForceKeyframe => {
                    println!("someone joined ! ");
                    self.force_keyframe = true;
                }
            }
        }

        // println!("test");

        if self.force_keyframe{
            self.encoder.force_intra_frame();
            self.force_keyframe = false;
        }


        let sequence = frame.sequence;

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

        let keyframe = contains_idr(&data);
        

        Ok(EncodedFrame {
            sequence,
            timestamp: frame.timestamp,

            keyframe,

            codec: VideoCodec::H264,

            width: frame.width,
            height: frame.height,

            data,
        })
    }


}

pub fn packetize(
    uid: u64,
    frame: &EncodedFrame,
) -> Vec<VideoPacket> {

    if frame.data.is_empty(){
        return Vec::new();
    }

    let packet_total = frame
        .data
        .len()
        .div_ceil(MAX_VIDEO_PAYLOAD);


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

fn contains_idr(data: &[u8]) -> bool {
    let mut i = 0;

    while i + 4 < data.len() {
        let start_code_len = if data[i..].starts_with(&[0, 0, 0, 1]) {
            4
        } else if data[i..].starts_with(&[0, 0, 1]) {
            3
        } else {
            i += 1;
            continue;
        };

        let nal_start = i + start_code_len;

        if nal_start < data.len() {
            let nal_type = data[nal_start] & 0x1F;

            if nal_type == 5 {
                return true;
            }
        }

        i = nal_start;
    }

    false
}

