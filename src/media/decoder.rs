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

        let Some(decoded) = self.decoder.decode(&frame.data)? else {
            return Ok(None);
        };

        let mut rgba = vec![0u8; decoded.rgba8_len()];
        decoded.write_rgba8(&mut rgba);

        let width = frame.width as usize;
        let height = frame.height as usize;

        let expected_len = width * height * 4;

        if rgba.len() != expected_len {
            eprintln!(
                "decoded dimensions don't match frame metadata: \
                 metadata={}x{}, rgba={} bytes",
                width,
                height,
                rgba.len()
            );

            return Ok(None);
        }

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