use std::{
    io::{self, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    thread,
};

use crate::{media::encoder::EncodedFrame, protocol::video::VideoCodec};


type EncoderResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct FFmpegEncoder {
    child: Child,
    stdin: ChildStdin,

    width: u32,
    height: u32,
    fps: u32,
    bitrate_kbps: u32,

    frame_count: u64,
    rx: mpsc::Receiver<Vec<u8>>,
}

impl FFmpegEncoder {
    pub fn new(
        ffmpeg_path: impl AsRef<std::ffi::OsStr>,
        width: u32,
        height: u32,
        fps: u32,
        bitrate_kbps: u32,
    ) -> EncoderResult<Self> {
        if width == 0 || height == 0 || fps == 0 {
            return Err("invalid encoder dimensions or FPS".into());
        }

        let mut child = Command::new(ffmpeg_path)
            // Raw BGRA frames come through stdin.
            .args([
                "-hide_banner",
                "-loglevel",
                "warning",

                "-f",
                "rawvideo",

                "-pix_fmt",
                "bgra",

                "-video_size",
                &format!("{}x{}", width, height),

                "-framerate",
                &fps.to_string(),

                "-i",
                "pipe:0",

                // GPU encoder.
                "-c:v",
                "h264_nvenc",

                // Low-latency settings.
                "-preset",
                "p4",

                "-tune",
                "ll",

                "-rc",
                "cbr",

                "-b:v",
                &format!("{}k", bitrate_kbps),

                "-maxrate",
                &format!("{}k", bitrate_kbps),

                "-bufsize",
                &format!("{}k", bitrate_kbps * 2),

                "-g",
                &(fps * 2).to_string(),

                "-bf",
                "0",

                "-pix_fmt",
                "yuv420p",

                // Raw H264 elementary stream.
                "-f",
                "h264",

                "pipe:1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or("failed to open FFmpeg stdin")?;

        let stdout = child
            .stdout
            .take()
            .ok_or("failed to open FFmpeg stdout")?;

        // FFmpeg writes an H264 byte stream to stdout.
        //
        // This thread collects bytes into complete H264 access units.
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut stdout = stdout;
            let mut buffer = vec![0u8; 1024 * 1024];
            let mut pending = Vec::new();

            loop {
                match std::io::Read::read(&mut stdout, &mut buffer) {
                    Ok(0) => break,

                    Ok(n) => {
                        pending.extend_from_slice(&buffer[..n]);

                        // H264 uses Annex-B start codes.
                        //
                        // We split whenever we find the next start code.
                        while let Some(end) = find_next_access_unit(&pending) {
                            let packet = pending.drain(..end).collect::<Vec<_>>();

                            if !packet.is_empty() {
                                if tx.send(packet).is_err() {
                                    return;
                                }
                            }
                        }
                    }

                    Err(_) => break,
                }
            }

            if !pending.is_empty() {
                let _ = tx.send(pending);
            }
        });

        Ok(Self {
            child,
            stdin,
            width,
            height,
            fps,
            bitrate_kbps,
            frame_count: 0,
            rx,
        })
    }

    pub fn encode_bgra_frame(
        &mut self,
        pixels: &[u8],
        timestamp: u64,
        sequence: u64,
    ) -> EncoderResult<Option<EncodedFrame>> {
        let expected = self.width as usize * self.height as usize * 4;

        if pixels.len() != expected {
            return Err(format!(
                "invalid BGRA frame size: got {}, expected {}",
                pixels.len(),
                expected
            )
            .into());
        }

        self.stdin.write_all(pixels)?;
        self.stdin.flush()?;

        self.frame_count += 1;

        match self.rx.try_recv() {
            Ok(data) => {
                let keyframe = is_keyframe(&data);

                Ok(Some(EncodedFrame {
                    sequence,
                    data,
                    timestamp,
                    codec: VideoCodec::H264,
                    keyframe,
                    width: self.width as usize,
                    height: self.height as usize,
                }))
            }

            Err(mpsc::TryRecvError::Empty) => Ok(None),

            Err(mpsc::TryRecvError::Disconnected) => {
                Err("FFmpeg encoder process exited".into())
            }
}
    }

    pub fn stats(&self) -> EncoderStats {
        EncoderStats {
            frames_encoded: self.frame_count,
            bitrate_kbps: self.bitrate_kbps,
            width: self.width,
            height: self.height,
            fps: self.fps,
        }
    }
}

impl Drop for FFmpegEncoder {
    fn drop(&mut self) {
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn find_next_access_unit(data: &[u8]) -> Option<usize> {
    if data.len() < 5 {
        return None;
    }

    // Look for the second Annex-B start code.
    for i in 4..data.len().saturating_sub(3) {
        if data[i] == 0
            && data[i + 1] == 0
            && data[i + 2] == 0
            && data[i + 3] == 1
        {
            return Some(i);
        }

        // 00 00 01
        if i >= 3
            && data[i - 2] == 0
            && data[i - 1] == 0
            && data[i] == 1
        {
            return Some(i + 1);
        }
    }

    None
}

fn is_keyframe(data: &[u8]) -> bool {
    let mut i = 0;

    while i + 4 < data.len() {
        let start_len;

        if data[i..].starts_with(&[0, 0, 0, 1]) {
            start_len = 4;
        } else if data[i..].starts_with(&[0, 0, 1]) {
            start_len = 3;
        } else {
            i += 1;
            continue;
        }

        if i + start_len >= data.len() {
            break;
        }

        let nal_type = data[i + start_len] & 0x1f;

        // IDR slice.
        if nal_type == 5 {
            return true;
        }

        i += start_len + 1;
    }

    false
}

#[derive(Debug, Clone, Copy)]
pub struct EncoderStats {
    pub frames_encoded: u64,
    pub bitrate_kbps: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}