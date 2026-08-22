use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::Instant,
};

use anyhow::{anyhow, Context, Result};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    media::{
        encoder::EncodedFrame,
        frame::RawFrame,
        state::Media,
    },
};

pub struct FFmpegDecoder;

impl FFmpegDecoder {
    /// Starts FFmpeg + NVDEC (h264_cuvid) as a decoder.
    ///
    /// Pipeline:
    ///
    /// Encoded H264 NALs (from network)
    ///      ↓
    /// stdin (start codes re-added)
    ///      ↓
    /// FFmpeg
    ///      ↓
    /// h264_cuvid
    ///      ↓
    /// stdout
    ///      ↓
    /// raw BGRA frames
    ///
    /// IMPORTANT:
    /// width/height must match the resolution the stream was encoded at.
    /// FFmpeg auto-detects the coded resolution from the SPS, and honors
    /// the SPS conformance-window crop, so the raw frames coming out of
    /// stdout should already match the original capture dimensions.
    pub async fn start(
        width: u32,
        height: u32,
        raw_tx: Sender<RawFrame>,
        mut encoded_rx: Receiver<EncodedFrame>,
    ) -> Result<()> {
        if width == 0 || height == 0 {
            return Err(anyhow!("invalid decoder dimensions: {}x{}", width, height));
        }

        println!(
            "[FFMPEG-DEC] starting decoder {}x{}",
            width, height
        );

        // -------------------------------------------------------------
        // Verify FFmpeg exists.
        // -------------------------------------------------------------

        let version = Command::new("ffmpeg")
            .arg("-version")
            .output()
            .context("failed to execute ffmpeg; make sure FFmpeg is in PATH")?;

        if !version.status.success() {
            return Err(anyhow!(
                "ffmpeg -version failed with status {:?}",
                version.status.code()
            ));
        }

        println!("[FFMPEG-DEC] ffmpeg executable found");

        // -------------------------------------------------------------
        // Check whether NVDEC (h264_cuvid) is available; fall back to
        // the CPU decoder if not.
        // -------------------------------------------------------------

        let decoder_check = Command::new("ffmpeg")
            .args(["-hide_banner", "-decoders"])
            .output()
            .context("failed to query FFmpeg decoders")?;

        let decoder_list = String::from_utf8_lossy(&decoder_check.stdout);
        let use_cuvid = decoder_list.contains("h264_cuvid");

        if use_cuvid {
            println!("[FFMPEG-DEC] h264_cuvid available, using NVDEC");
        } else {
            println!("[FFMPEG-DEC] h264_cuvid not available, falling back to CPU h264 decoder");
        }

        // -------------------------------------------------------------
        // Spawn FFmpeg.
        //
        // Input:
        //   Annex-B H264 elementary stream
        //
        // Output:
        //   raw BGRA frames
        // -------------------------------------------------------------

        let mut args: Vec<String> = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "warning".into(),

            // Don't let ffmpeg buffer input trying to build a GOP before
            // producing output; we want frames out as soon as decodable.
            "-fflags".into(),
            "nobuffer".into(),
            "-flags".into(),
            "low_delay".into(),
        ];

        if use_cuvid {
            args.extend([
                "-c:v".into(),
                "h264_cuvid".into(),
            ]);
        } else {
            args.extend([
                "-c:v".into(),
                "h264".into(),
            ]);
        }

        args.extend([
            "-f".into(),
            "h264".into(),

            "-i".into(),
            "pipe:0".into(),

            "-an".into(),

            "-f".into(),
            "rawvideo".into(),

            "-pix_fmt".into(),
            "bgra".into(),

            "pipe:1".into(),
        ]);

        let mut child = Command::new("ffmpeg")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(&args)
            .spawn()
            .context("failed to spawn FFmpeg decoder")?;

        println!(
            "[FFMPEG-DEC] process started pid={:?}",
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
        // -------------------------------------------------------------

        thread::spawn(move || {
            use std::io::{BufRead, BufReader};

            let reader = BufReader::new(stderr);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        eprintln!("[FFMPEG-DEC] {}", line);
                    }

                    Err(e) => {
                        eprintln!(
                            "[FFMPEG-DEC] stderr read error: {}",
                            e
                        );
                        break;
                    }
                }
            }

            println!("[FFMPEG-DEC] stderr closed");
        });

        // -------------------------------------------------------------
        // stdout reader.
        //
        // Unlike the encoder's stdout, this is `rawvideo`: fixed-size
        // BGRA frames, no start-code parsing needed. We just read
        // width*height*4 bytes at a time.
        // -------------------------------------------------------------

        thread::spawn(move || {
            if let Err(e) = read_raw_stdout(
                stdout,
                raw_tx,
                width as usize,
                height as usize,
            ) {
                eprintln!(
                    "[FFMPEG-DEC] stdout reader stopped: {:#}",
                    e
                );
            }

            println!("[FFMPEG-DEC] stdout reader stopped");
        });

        // -------------------------------------------------------------
        // Encoded NAL writer.
        //
        // encoded_rx -> re-add Annex-B start code -> FFmpeg stdin
        //
        // Dedicated thread for the same reason as the encoder: a
        // blocking pipe write must never block the Tokio runtime.
        // -------------------------------------------------------------

        thread::spawn(move || {
            let mut nals = 0u64;
            let report_start = Instant::now();

            const START_CODE: [u8; 4] = [0, 0, 0, 1];

            while let Some(frame) = encoded_rx.blocking_recv() {

                if let Err(e) = stdin.write_all(&START_CODE) {
                    eprintln!(
                        "[FFMPEG-DEC] stdin start-code write failed at frame={}: {}",
                        frame.sequence,
                        e
                    );
                    break;
                }

                if let Err(e) = stdin.write_all(&frame.data) {
                    eprintln!(
                        "[FFMPEG-DEC] stdin write failed at frame={}: {}",
                        frame.sequence,
                        e
                    );
                    break;
                }

                nals += 1;

                if report_start.elapsed().as_secs() >= 1 {
                    println!(
                        "[FFMPEG-DEC] input NAL rate={}/s",
                        nals
                    );

                    nals = 0;
                }
            }

            println!("[FFMPEG-DEC] closing stdin");

            // Dropping stdin sends EOF to FFmpeg.
            drop(stdin);

            println!("[FFMPEG-DEC] stdin closed");
        });

        // -------------------------------------------------------------
        // Reaper.
        // -------------------------------------------------------------

        thread::spawn(move || {
            match child.wait() {
                Ok(status) => {
                    println!(
                        "[FFMPEG-DEC] process exited: {}",
                        status
                    );
                }

                Err(e) => {
                    eprintln!(
                        "[FFMPEG-DEC] failed waiting for process: {}",
                        e
                    );
                }
            }
        });

        println!("[FFMPEG-DEC] decoder pipeline ready");

        Ok(())
    }
}


// =====================================================================
// Raw BGRA stdout reader
// =====================================================================

fn read_raw_stdout(
    mut stdout: impl Read,
    raw_tx: Sender<RawFrame>,
    width: usize,
    height: usize,
) -> Result<()> {
    let frame_size = width * height * 4;
    let mut buffer = vec![0u8; frame_size];

    let mut sequence = 0u64;
    let start = Instant::now();

    loop {
        if let Err(e) = stdout.read_exact(&mut buffer) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                println!("[FFMPEG-DEC] stdout EOF");
                return Ok(());
            }

            return Err(e).context("failed reading FFmpeg decoder stdout");
        }

        let frame = RawFrame {
            sequence,
            timestamp: start.elapsed().as_micros() as u64,
            width,
            height,
            data: buffer.clone(),
        };

        if sequence <= 10 || sequence % 60 == 0 {
            println!(
                "[FFMPEG-DEC] decoded frame={} bytes={}",
                frame.sequence,
                frame.data.len()
            );
        }

        sequence += 1;

        match raw_tx.blocking_send(frame) {
            Ok(()) => {}

            Err(_) => {
                println!(
                    "[FFMPEG-DEC] raw output channel closed"
                );
                return Ok(());
            }
        }
    }
}