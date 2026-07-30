use openh264::{
	encoder::Encoder, formats::{BgraSliceU8, RgbSliceU8, YUVBuffer},
};

use crate::{media::frame::SharedFrame, protocol::video::{VideoCodec, VideoPacket}};

pub const MAX_VIDEO_PAYLOAD: usize = 1150;

pub struct EncodedFrame {
	pub width: u32,
	pub height: u32,
	pub codec: VideoCodec,
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

	pub fn encode_frame(&mut self, frame: &SharedFrame) -> Result<EncodedFrame, openh264::Error> {
        let rgb: Vec<u8> = frame
            .data
            .chunks_exact(4)
            .flat_map(|bgra| [bgra[2], bgra[1], bgra[0]])
            .collect();

        let rgb = RgbSliceU8::new(&rgb, (frame.width, frame.height));
        let yuv = YUVBuffer::from_rgb8_source(rgb);
        
		let bitstream = self.encoder.encode(&yuv)?;

		let mut data = Vec::new();
		bitstream.write_vec(&mut data);

		Ok(EncodedFrame {
			width: frame.width as u32,
			height: frame.height as u32,
			codec: VideoCodec::H264,
			data,
		})
	}

	pub fn packetize(&self, uid: u64, frame_id: u64, frame: EncodedFrame) -> Vec<VideoPacket> {
		let packet_total = frame.data.len().div_ceil(MAX_VIDEO_PAYLOAD).max(1) as u16;

		frame
			.data
			.chunks(MAX_VIDEO_PAYLOAD)
			.enumerate()
			.map(|(packet_index, data)| VideoPacket {
				uid,
				frame_id,
				packet_index: packet_index as u16,
				packet_total,
				codec: frame.codec.as_u8(),
				width: frame.width,
				height: frame.height,
				data: data.to_vec(),
			})
			.collect()
	}

	pub fn encode_and_packetize(
		&mut self,
		uid: u64,
		frame_id: u64,
		frame: &SharedFrame,
	) -> Result<Vec<VideoPacket>, openh264::Error> {
		let encoded = self.encode_frame(frame)?;
		Ok(self.packetize(uid, frame_id, encoded))
	}
}
