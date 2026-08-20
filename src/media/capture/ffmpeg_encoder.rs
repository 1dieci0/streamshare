use std::{
    io,
    path::PathBuf,
    process::Stdio,
    sync::Arc,
};

use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    sync::mpsc::Sender,
};

use crate::{media::encoder::EncodedFrame, protocol::video::VideoCodec};



pub struct FFmpegEncoder {
    child: Child,
}

impl FFmpegEncoder {
    pub async fn start(
        width: u32,
        height: u32,
        fps: u32,
        bitrate_kbps: u32,
        video_tx: Sender<EncodedFrame>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if width == 0 || height == 0 || fps == 0 {
            return Err("invalid encoder dimensions/fps".into());
        }

        let ffmpeg = find_ffmpeg()?;

        println!("Starting FFmpeg: {}", ffmpeg.display());

        let filter = format!(
            "ddagrab=output_idx=0:framerate={}:video_size={}x{}:draw_mouse=1",
            fps, width, height
        );

        let mut child = Command::new(&ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "warning",

                "-f",
                "lavfi",
                "-i",
                &filter,

                "-c:v",
                "h264_nvenc",

                "-preset",
                "p4",
                "-tune",
                "ll",
                "-zerolatency",
                "1",

                "-bf",
                "0",

                "-g",
                &(fps * 2).to_string(),

                "-keyint_min",
                &(fps * 2).to_string(),

                "-forced-idr",
                "1",

                "-b:v",
                &format!("{}k", bitrate_kbps),

                // IMPORTANT:
                // Insert an Access Unit Delimiter before each H264 access unit.
                "-bsf:v",
                "h264_metadata=aud=insert",

                "-f",
                "h264",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or("failed to open FFmpeg stdout")?;
        

        tokio::spawn(async move {
            if let Err(e) =
                read_h264_stream(&mut stdout, video_tx, width, height, fps).await
            {
                eprintln!("FFmpeg H264 reader stopped: {e}");
            }
        });

        Ok(Self { child })
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.child.kill().await?;
        Ok(())
    }

    pub fn is_running(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }
}

async fn read_h264_stream(
    stdout: &mut tokio::process::ChildStdout,
    video_tx: Sender<EncodedFrame>,
    width: u32,
    height: u32,
    fps: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("H264 reader started");

    let mut buffer = Vec::<u8>::with_capacity(1024 * 1024);

    let mut temp = [0u8; 64 * 1024];

    let frame_duration_ms = 1000 / fps as u64;

    let mut sequence = 0u64;

    loop {
        let n = stdout.read(&mut temp).await?;

        // if n == 0 {
        //     println!("FFmpeg closed stdout");
        //     break;
        // }

        buffer.extend_from_slice(&temp[..n]);

        // println!(
        //     "FFmpeg gave us {} bytes, buffer = {} bytes",
        //     n,
        //     buffer.len()
        // );

        loop {
            let Some(frame_end) = find_next_frame(&buffer) else {
                break;
            };

            let data: Vec<u8> = buffer.drain(..frame_end).collect();

            if data.is_empty() {
                continue;
            }

            let keyframe = contains_idr(&data);
            if keyframe{
                println!("idr keyframe :fireemoji:");
            }

            // println!(
            //     "H264 frame {}: {} bytes, keyframe={}",
            //     sequence,
            //     data.len(),
            //     keyframe
            // );

            let encoded = EncodedFrame {
                sequence,
                timestamp: sequence * frame_duration_ms,
                keyframe,
                codec: VideoCodec::H264,
                width: width as usize,
                height: height as usize,
                data,
            };

            sequence += 1;

            match video_tx.try_send(encoded) {
                Ok(_) => {}

                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    // Drop frames if the network side is behind.
                    //
                    // This prevents latency from growing forever.
                }

                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    println!("Video channel closed");
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

/// Find a complete H264 access unit.
///
/// H264 Annex-B contains:
///
/// 00 00 01
/// or
/// 00 00 00 01
///
/// We split on AUD NAL units (NAL type 9).
fn find_next_access_unit(buffer: &[u8]) -> Option<usize> {
    let mut positions = Vec::new();

    let mut i = 0;

    while i + 3 <= buffer.len() {
        let start_len = if i + 4 <= buffer.len()
            && buffer[i] == 0
            && buffer[i + 1] == 0
            && buffer[i + 2] == 0
            && buffer[i + 3] == 1
        {
            4
        } else if buffer[i] == 0
            && buffer[i + 1] == 0
            && buffer[i + 2] == 1
        {
            3
        } else {
            i += 1;
            continue;
        };

        let nal_start = i + start_len;

        if nal_start < buffer.len() {
            let nal_type = buffer[nal_start] & 0x1f;

            if nal_type == 9 {
                positions.push(i);
            }
        }

        i += start_len;
    }

    // Need two AUDs to know where the current access unit ends.
    if positions.len() >= 2 {
        Some(positions[1])
    } else {
        None
    }
}

fn contains_idr(data: &[u8]) -> bool {
    let mut i = 0;

    while i + 3 < data.len() {
        let start_code_len;

        if data[i] == 0
            && data[i + 1] == 0
            && data[i + 2] == 0
            && data[i + 3] == 1
        {
            start_code_len = 4;
        } else if data[i] == 0
            && data[i + 1] == 0
            && data[i + 2] == 1
        {
            start_code_len = 3;
        } else {
            i += 1;
            continue;
        }

        let nal_start = i + start_code_len;

        if nal_start >= data.len() {
            break;
        }

        let nal_type = data[nal_start] & 0x1f;

        if nal_type == 5 {
            return true;
        }

        i = nal_start;
    }

    false
}

fn find_ffmpeg() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // First try ffmpeg.exe next to the application.
    let exe_dir = std::env::current_exe()?
        .parent()
        .ok_or("failed to determine executable directory")?
        .to_path_buf();

    // println!("{}", exe_dir.clone().into_os_string().into_string().unwrap());

    let local = exe_dir.join("ffmpeg.exe");

    if local.exists() {
        return Ok(local);
    }

    // Then try PATH.
    if let Ok(path) = which::which("ffmpeg") {
        return Ok(path);
    }

    Err(
        "ffmpeg.exe was not found. Put ffmpeg.exe next to the StreamShare executable."
            .into(),
    )
}

fn find_next_frame(buffer: &[u8]) -> Option<usize> {
    let mut aud_positions = Vec::new();

    let mut i = 0;

    while i + 3 < buffer.len() {
        let start_code_len = if buffer[i..].starts_with(&[0, 0, 0, 1]) {
            4
        } else if buffer[i..].starts_with(&[0, 0, 1]) {
            3
        } else {
            i += 1;
            continue;
        };

        let nal_start = i + start_code_len;

        if nal_start >= buffer.len() {
            break;
        }

        let nal_type = buffer[nal_start] & 0x1f;

        // H264 AUD
        if nal_type == 9 {
            aud_positions.push(i);

            // We have:
            //
            // AUD #1 ... frame data ... AUD #2
            //
            // Therefore everything before AUD #2 is one access unit.
            if aud_positions.len() >= 2 {
                return Some(aud_positions[1]);
            }
        }

        i = nal_start;
    }

    None
}