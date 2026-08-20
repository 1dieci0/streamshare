use openh264::{
    decoder::Decoder,
    formats::YUVSource,
};

use crate::{
    media::{
        encoder::EncodedFrame,
        frame::RawFrame,
    },
    protocol::video::VideoCodec,
};

pub struct VideoDecoder {
    decoder: Decoder,
}

impl VideoDecoder {
    pub fn new() -> Result<Self, openh264::Error> {
        Ok(Self {
            decoder: Decoder::new()?,
        })
    }

    pub fn decode_frame(
        &mut self,
        frame: &EncodedFrame,
    ) -> Result<Option<RawFrame>, openh264::Error> {
        if frame.codec != VideoCodec::H264 {
            return Ok(None);
        }

        // println!(
        //     "Decoding H264: seq={}, {} bytes, keyframe={}",
        //     frame.sequence,
        //     frame.data.len(),
        //     frame.keyframe
        // );

        let Some(decoded) = self.decoder.decode(&frame.data)? else {
            return Ok(None);
        };

        let width = decoded.dimensions().0 as usize;
        let height = decoded.dimensions().1 as usize;

        let expected_len = width * height * 4;

        let mut rgba = vec![0u8; expected_len];
        decoded.write_rgba8(&mut rgba);

        let mut bgra = vec![0u8; expected_len];

        for (dst, src) in bgra
            .chunks_exact_mut(4)
            .zip(rgba.chunks_exact(4))
        {
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = src[3];
        }

        Ok(Some(RawFrame {
            sequence: frame.sequence,
            timestamp: frame.timestamp,
            width,
            height,
            data: bgra,
        }))
    }
}