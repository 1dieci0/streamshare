use openh264::{decoder::Decoder, formats::YUVSource};

use crate::{media::frame::SharedFrame, protocol::video::VideoCodec};

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
		codec: u8,
		sequence: u64,
		width: usize,
		height: usize,
		data: &[u8],
	) -> Result<Option<SharedFrame>, openh264::Error> {
		let Some(VideoCodec::H264) = VideoCodec::from_u8(codec) else {
			return Ok(None);
		};

		let Some(decoded) = self.decoder.decode(data)? else {
			return Ok(None);
		};

		let mut rgba = vec![0u8; decoded.rgba8_len()];
		decoded.write_rgba8(&mut rgba);

		let mut bgra = vec![0u8; width * height * 4];

		for (dst, src) in bgra.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
			dst[0] = src[2];
			dst[1] = src[1];
			dst[2] = src[0];
			dst[3] = src[3];
		}

		Ok(Some(SharedFrame {
			sequence,
			width,
			height,
			data: bgra,
		}))
	}
}
