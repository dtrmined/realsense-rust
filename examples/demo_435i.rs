use anyhow::{ensure, Result};
use realsense_rust::{
    config::Config,
    context::Context,
    frame::{DepthFrame, MotionFrame},
    kind::{Rs2Format, Rs2ProductLine, Rs2StreamKind},
    pipeline::InactivePipeline,
};
use std::{
    collections::HashSet,
    convert::TryFrom,
    io::{self, Write},
    time::Duration,
};
pub fn main() -> Result<()> {
    // Check for depth or color-compatible devices.
    let mut queried_devices = HashSet::new();
    queried_devices.insert(Rs2ProductLine::Any);
    let context = Context::new()?;
    let devices = context.query_devices(queried_devices);
    ensure!(!devices.is_empty(), "No devices found");

    // create pipeline
    let pipeline = InactivePipeline::try_from(&context)?;
    let mut config = Config::new();
    config
        .enable_device_from_serial(devices[0].info(Rs2CameraInfo::SerialNumber).unwrap())?
        .disable_all_streams()?
        .enable_stream(Rs2StreamKind::Depth, 0, 640, 0, Rs2Format::Z16, 30)?
        .enable_stream(Rs2StreamKind::Color, 0, 640, 0, Rs2Format::Rgb8, 30)?
        .enable_stream(Rs2StreamKind::Gyro, 0, 0, 0, Rs2Format::Any, 0)?;
    // Change pipeline's type from InactivePipeline -> ActivePipeline
    let mut pipeline = pipeline.start(Some(&config))?;
    let mut distance = 0.0;
    let mut motion = [0.0, 0.0, 0.0];

    // process frames
    for i in 0..1000 {
        let timeout = Duration::from_millis(5000);
        let frames = pipeline.wait(Some(timeout))?;

        // Get depth
        let mut depth_frames = frames.frames_of_extension::<DepthFrame>();
        if !depth_frames.is_empty() {
            let depth_frame = depth_frames.pop().unwrap();
            let tmp_distance =
                depth_frame.distance(depth_frame.width() / 2, depth_frame.height() / 2)?;
            if tmp_distance != 0.0 {
                distance = tmp_distance;
            }
        }

        if i % 10 == 0 {
            // Get gyro
            let motion_frames = frames.frames_of_extension::<MotionFrame>();
            if !motion_frames.is_empty() {
                motion = *motion_frames[0].motion();
            }
        }

        // Print our results
        println!(
            "Distance of center pixel: {:<10} m | Gyro reading: {:>15}, {:>15}, {:>15}",
            distance, motion[0], motion[1], motion[2]
        );
        //io::stdout().flush().unwrap();
    }

    Ok(())
}
