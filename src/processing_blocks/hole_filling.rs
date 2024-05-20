//! Processing block hole filling

use crate::{
    check_rs2_error,
    frame::CompositeFrame,
    processing_blocks::errors::{ProcessFrameError, ProcessingBlockConstructionError},
};
use anyhow::Result;
use realsense_sys as sys;
use std::{convert::TryFrom, ptr::NonNull, task::Poll, time::Duration};

/// Processing Block and Frame Queue for hole filling a stream
#[derive(Debug, Clone)]
pub struct HoleFilling {
    /// The processing block for the "HoleFilling" method
    processing_block: NonNull<sys::rs2_processing_block>,
    /// The frame queue upon which the processing block will deposit hole filled frames. We check this
    /// for completed block operations.
    processing_queue: NonNull<sys::rs2_frame_queue>,
}

impl Drop for HoleFilling {
    fn drop(&mut self) {
        unsafe {
            sys::rs2_delete_frame_queue(self.processing_queue.as_ptr());
            sys::rs2_delete_processing_block(self.processing_block.as_ptr());
        }
    }
}

impl HoleFilling {
    /// Create a new HoleFilling object
    pub fn new(processing_queue_size: i32) -> Result<Self, ProcessingBlockConstructionError> {
        let (processing_block, processing_queue) = unsafe {
            let mut err = std::ptr::null_mut::<sys::rs2_error>();

            let ptr = sys::rs2_create_hole_filling_filter_block(&mut err);
            check_rs2_error!(
                err,
                ProcessingBlockConstructionError::CouldNotCreateProcessingBlock
            )?;

            let queue_ptr = sys::rs2_create_frame_queue(processing_queue_size, &mut err);
            check_rs2_error!(
                err,
                ProcessingBlockConstructionError::CouldNotCreateProcessingQueue
            )?;

            sys::rs2_start_processing_queue(ptr, queue_ptr, &mut err);
            check_rs2_error!(
                err,
                ProcessingBlockConstructionError::CouldNotStartProcessingQueue
            )?;
            (NonNull::new(ptr).unwrap(), NonNull::new(queue_ptr).unwrap())
        };

        Ok(Self {
            processing_block,
            processing_queue,
        })
    }

    /// Own and process the depth frame and return the hole filled frames.
    pub fn queue(&mut self, frames: CompositeFrame) -> Result<(), ProcessFrameError> {
        unsafe {
            let mut err = std::ptr::null_mut::<sys::rs2_error>();
            sys::rs2_process_frame(
                self.processing_block.as_ptr(), // -> *mut
                frames.get_owned_raw().as_ptr(),
                &mut err,
            );
            check_rs2_error!(err, |kind, context| { ProcessFrameError { kind, context } })?;
            Ok(())
        }
    }

    /// Wait to receive the results of the processing block
    pub fn wait(&mut self, timeout: Duration) -> Result<CompositeFrame, ProcessFrameError> {
        unsafe {
            let mut err = std::ptr::null_mut::<sys::rs2_error>();
            let timeout_millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);

            let hole_filled_frame =
                sys::rs2_wait_for_frame(self.processing_queue.as_ptr(), timeout_millis, &mut err);
            check_rs2_error!(err, |kind, context| { ProcessFrameError { kind, context } })?;
            Ok(CompositeFrame::from(
                NonNull::new(hole_filled_frame).unwrap(),
            ))
        }
    }

    /// Poll to receive the results of the processing block
    pub fn poll(&mut self) -> Result<Poll<CompositeFrame>, ProcessFrameError> {
        unsafe {
            let mut err = std::ptr::null_mut::<sys::rs2_error>();
            let mut frame = std::ptr::null_mut::<sys::rs2_frame>();
            let is_ready =
                sys::rs2_poll_for_frame(self.processing_queue.as_ptr(), &mut frame, &mut err);

            // Check for errors
            check_rs2_error!(err, |kind, context| { ProcessFrameError { kind, context } })?;

            // Check for queue readiness
            if is_ready == 0 {
                Ok(Poll::Pending)
            } else {
                Ok(Poll::Ready(CompositeFrame::from(
                    NonNull::new(frame).unwrap(),
                )))
            }
        }
    }
}
