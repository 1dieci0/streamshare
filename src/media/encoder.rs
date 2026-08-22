use std::{
    io::Write,
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::Instant,
};

use anyhow::{anyhow, Context, Result};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    media::{
        frame::RawFrame,
        state::Media,
    },
    protocol::video::{VideoCodec, VideoPacket},
};

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

pub struct FFmpegEncoder;

impl FFmpegEncoder {
    /// Starts FFmpeg + NVIDIA NVENC.
    ///
    /// Pipeline:
    ///
    /// Raw BGRA frames
    ///      ↓
    /// stdin
    ///      ↓
    /// FFmpeg
    ///      ↓
    /// h264_nvenc
    ///      ↓
    /// stdout
    ///      ↓
    /// encoded H264 stream
    ///
    /// IMPORTANT:
    /// The FFmpeg process is deliberately kept as a child process and
    /// driven through stdin/stdout. No ffmpeg Rust bindings are required.
    pub async fn start(
        media: Arc<Media>,
        width: u32,
        height: u32,
        fps: u32,
        bitrate_kbps: u32,
        video_tx: Sender<EncodedFrame>,
        mut raw_rx: Receiver<RawFrame>,
    ) -> Result<()> {
        if width == 0 || height == 0 {
            return Err(anyhow!("invalid encoder dimensions: {}x{}", width, height));
        }

        if fps == 0 {
            return Err(anyhow!("fps must be > 0"));
        }

        if bitrate_kbps == 0 {
            return Err(anyhow!("bitrate must be > 0"));
        }

        println!(
            "[FFMPEG] starting NVIDIA encoder {}x{} @ {} FPS, {} kbps",
            width,
            height,
            fps,
            bitrate_kbps
        );

        // -------------------------------------------------------------
        // Verify FFmpeg exists.
        // -------------------------------------------------------------

        let version = Command::new("ffmpeg")
            .arg("-version")
            .output()
            .context("failed to execute ffmpeg.exe; make sure FFmpeg is in PATH")?;

        if !version.status.success() {
            return Err(anyhow!(
                "ffmpeg -version failed with status {:?}",
                version.status.code()
            ));
        }

        println!("[FFMPEG] ffmpeg executable found");

        // -------------------------------------------------------------
        // Verify NVENC exists.
        // -------------------------------------------------------------

        let encoder_check = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-encoders",
            ])
            .output()
            .context("failed to query FFmpeg encoders")?;

        let encoder_list = String::from_utf8_lossy(&encoder_check.stdout);

        if !encoder_list.contains("h264_nvenc") {
            return Err(anyhow!(
                "FFmpeg does not expose h264_nvenc; NVIDIA NVENC is unavailable"
            ));
        }

        println!("[FFMPEG] h264_nvenc available");

        // -------------------------------------------------------------
        // Spawn FFmpeg.
        //
        // Input:
        //   raw BGRA
        //
        // Output:
        //   raw H264 elementary stream
        // -------------------------------------------------------------

        let bitrate = format!("{}k", bitrate_kbps);
        let maxrate = format!("{}k", bitrate_kbps);
        let bufsize = format!("{}k", bitrate_kbps * 2);

        let mut child = Command::new("ffmpeg")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args([
                "-hide_banner",

                // We consume stderr ourselves.
                "-loglevel",
                "warning",

                // -----------------------------------------------------
                // Input
                // -----------------------------------------------------

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

                // -----------------------------------------------------
                // Video encoder
                // -----------------------------------------------------

                "-an",

                "-c:v",
                "h264_nvenc",

                // Low latency is important for screen streaming.
                "-preset",
                "p1",

                "-tune",
                "ull",

                // CBR is generally preferable for a streaming demo.
                "-rc",
                "cbr",

                "-b:v",
                &bitrate,

                "-maxrate",
                &maxrate,

                "-bufsize",
                &bufsize,

                // Keep GOP predictable.
                "-g",
                &(fps * 2).to_string(),

                "-keyint_min",
                &fps.to_string(),

                // Don't insert B frames for low latency.
                "-bf",
                "0",

                // Force repeated SPS/PPS on IDR frames.
                "-forced-idr",
                "1",


                "-f",
                "h264",

                "pipe:1",
            ])
            .spawn()
            .context("failed to spawn FFmpeg")?;

        println!(
            "[FFMPEG] process started pid={:?}",
            child.id()
        );

        let mut stdin = child
            .stdin
            .take()
            .context("failed to obtain FFmpeg stdin")?;

        let stdout = child
            .stdout
            .take()
            .context("failed to obtain FFmpeg stdout")?;

        let stderr = child
            .stderr
            .take()
            .context("failed to obtain FFmpeg stderr")?;

        // -------------------------------------------------------------
        // FFmpeg stderr logger.
        //
        // VERY useful while getting NVENC working.
        // -------------------------------------------------------------

        thread::spawn(move || {
            use std::io::{BufRead, BufReader};

            let reader = BufReader::new(stderr);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        eprintln!("[FFMPEG] {}", line);
                    }

                    Err(e) => {
                        eprintln!(
                            "[FFMPEG] stderr read error: {}",
                            e
                        );
                        break;
                    }
                }
            }

            println!("[FFMPEG] stderr closed");
        });

        // -------------------------------------------------------------
        // stdout reader.
        //
        // H264 is an elementary byte stream, NOT one stdout read = one
        // encoded frame.
        //
        // We therefore parse Annex-B start codes and emit complete NAL
        // units / access units.
        // -------------------------------------------------------------

        thread::spawn(move || {
            if let Err(e) = read_h264_stdout(
                stdout,
                video_tx,
                width as usize,
                height as usize,
            ) {
                eprintln!(
                    "[FFMPEG] stdout reader stopped: {:#}",
                    e
                );
            }

            println!("[FFMPEG] stdout reader stopped");
        });

        // -------------------------------------------------------------
        // Raw frame writer.
        //
        // This is the important part:
        //
        // capture -> raw_rx -> FFmpeg stdin
        //
        // We intentionally use a dedicated thread because a blocking
        // write to a process pipe must NEVER block the Tokio runtime.
        // -------------------------------------------------------------

        thread::spawn(move || {
            let mut frames = 0u64;
            let mut dropped = 0u64;
            let report_start = Instant::now();

            while let Some(frame) = raw_rx.blocking_recv() {
                if !media.is_streaming() {
                    println!(
                        "[FFMPEG] media stopped streaming"
                    );
                    break;
                }

                if frame.width != width as usize
                    || frame.height != height as usize
                {
                    eprintln!(
                        "[FFMPEG] dropping frame={} wrong size {}x{} expected {}x{}",
                        frame.sequence,
                        frame.width,
                        frame.height,
                        width,
                        height
                    );

                    dropped += 1;
                    continue;
                }

                let expected_size =
                    width as usize *
                    height as usize *
                    4;

                if frame.data.len() != expected_size {
                    eprintln!(
                        "[FFMPEG] dropping frame={} wrong buffer size={} expected={}",
                        frame.sequence,
                        frame.data.len(),
                        expected_size
                    );

                    dropped += 1;
                    continue;
                }

                let write_start = Instant::now();

                if let Err(e) = stdin.write_all(&frame.data) {
                    eprintln!(
                        "[FFMPEG] stdin write failed at frame={}: {}",
                        frame.sequence,
                        e
                    );

                    break;
                }

                frames += 1;

                let write_time = write_start.elapsed();

                if frames <= 10 || frames % 60 == 0 {
                    println!(
                        "[FFMPEG] wrote frame={} bytes={} write={:?}",
                        frame.sequence,
                        frame.data.len(),
                        write_time
                    );
                }

                if report_start.elapsed().as_secs() >= 1 {
                    println!(
                        "[FFMPEG] input FPS={} dropped={}",
                        frames,
                        dropped
                    );

                    frames = 0;
                    dropped = 0;
                }
            }

            println!(
                "[FFMPEG] closing stdin"
            );

            // Dropping stdin sends EOF to FFmpeg.
            drop(stdin);

            println!(
                "[FFMPEG] stdin closed"
            );
        });

        // -------------------------------------------------------------
        // Reaper.
        //
        // We need to keep the Child alive and eventually report its
        // exit status.
        // -------------------------------------------------------------

        thread::spawn(move || {
            match child.wait() {
                Ok(status) => {
                    println!(
                        "[FFMPEG] process exited: {}",
                        status
                    );
                }

                Err(e) => {
                    eprintln!(
                        "[FFMPEG] failed waiting for process: {}",
                        e
                    );
                }
            }
        });

        println!("[FFMPEG] encoder pipeline ready");

        Ok(())
    }
}


// =====================================================================
// H264 stdout parser
// =====================================================================

fn read_h264_stdout(
    mut stdout: impl std::io::Read,
    video_tx: Sender<EncodedFrame>,
    width: usize,
    height: usize,
) -> Result<()> {
    use std::io::Read;

    let mut buffer = Vec::<u8>::with_capacity(1024 * 1024);

    let mut temp = [0u8; 64 * 1024];

    let mut sequence = 0u64;
    let mut timestamp = Instant::now();

    loop {
        let n = stdout
            .read(&mut temp)
            .context("failed reading FFmpeg stdout")?;

        if n == 0 {
            break;
        }

        buffer.extend_from_slice(&temp[..n]);

        // -------------------------------------------------------------
        // Extract complete Annex-B NAL units.
        // -------------------------------------------------------------

        loop {
            let Some(first) = find_start_code(&buffer, 0) else {
                // Keep a few bytes in case the start code is split
                // across stdout reads.
                if buffer.len() > 4 {
                    let keep = buffer.split_off(buffer.len() - 4);
                    buffer = keep;
                }

                break;
            };

            if first > 0 {
                buffer.drain(..first);
            }

            let Some(start_len) = start_code_length(&buffer) else {
                break;
            };

            let Some(next) =
                find_start_code(&buffer, start_len)
            else {
                // We have an incomplete NAL.
                break;
            };

            let nal = buffer[start_len..next].to_vec();

            buffer.drain(..next);

            if nal.is_empty() {
                continue;
            }

            let nal_type = nal[0] & 0x1f;

            println!(
                "[FFMPEG] NAL type={} size={} bytes",
                nal_type,
                nal.len()
            );

            // H264 NAL types:
            //
            // 1 = non-IDR slice
            // 5 = IDR slice
            // 7 = SPS
            // 8 = PPS
            //
            // This implementation emits each NAL as an EncodedFrame.
            //
            // For a production decoder, I recommend upgrading this to
            // proper access-unit grouping once the basic pipeline works.
            let keyframe = nal_type == 5;

            let now = timestamp.elapsed();
            timestamp = Instant::now();

            let encoded = EncodedFrame {
                sequence,
                timestamp: now.as_micros() as u64,
                keyframe,
                codec: VideoCodec::H264,
                width,
                height,
                data: nal,
            };

            sequence += 1;

            match video_tx.blocking_send(encoded) {
                Ok(()) => {}

                Err(_) => {
                    println!(
                        "[FFMPEG] video output channel closed"
                    );

                    return Ok(());
                }
            }
        }
    }

    Ok(())
}


// =====================================================================
// Annex-B helpers
// =====================================================================

fn start_code_length(data: &[u8]) -> Option<usize> {
    if data.starts_with(&[0, 0, 0, 1]) {
        Some(4)
    } else if data.starts_with(&[0, 0, 1]) {
        Some(3)
    } else {
        None
    }
}

fn find_start_code(
    data: &[u8],
    from: usize,
) -> Option<usize> {
    if data.len() < 3 {
        return None;
    }

    let mut i = from;

    while i + 3 <= data.len() {
        if i + 4 <= data.len()
            && data[i..i + 4] == [0, 0, 0, 1]
        {
            return Some(i);
        }

        if data[i..i + 3] == [0, 0, 1] {
            return Some(i);
        }

        i += 1;
    }

    None
}


// =====================================================================
// QUIC packetization
// =====================================================================

pub fn packetize(
    uid: u64,
    frame: &EncodedFrame,
) -> Vec<VideoPacket> {
    if frame.data.is_empty() {
        return Vec::new();
    }

    let packet_total =
        frame.data.len().div_ceil(MAX_VIDEO_PAYLOAD);

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