//! Vision's `VNRecognizeTextRequest` behind [`TextRecognizer`], Low Power Mode behind
//! [`PowerMode`], and the thread's QoS class.

use std::ptr;

use objc2::AnyThread;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::AnyObject;
use objc2_core_foundation::{CFData, CFRetained};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo,
};
use objc2_foundation::{NSArray, NSDictionary, NSProcessInfo, NSString};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel,
};

use crate::text::{OcrError, PowerMode, RgbaImage, TextRecognizer};

/// Larger images are scaled down to this first. Text in a 5K screenshot still reads well at it.
const MAX_DIMENSION: u32 = 4096;

/// Lines Vision is less sure of than this are mostly noise, such as icons read as letters.
const MIN_CONFIDENCE: f32 = 0.3;

/// Vision's accurate text recognition, with language correction, in the languages it detects
/// in each image. It doesn't need the main thread.
#[derive(Debug, Clone, Default)]
pub struct VisionRecognizer;

impl TextRecognizer for VisionRecognizer {
    fn available(&self) -> bool {
        true
    }

    fn max_dimension(&self) -> u32 {
        MAX_DIMENSION
    }

    fn recognize(&self, image: &RgbaImage) -> Result<Vec<String>, OcrError> {
        let expected = image.width as usize * image.height as usize * 4;
        if image.width == 0 || image.height == 0 || image.pixels.len() != expected {
            return Err(OcrError::BadImage(image.width, image.height));
        }
        // Vision autoreleases its results. This thread has no pool of its own.
        autoreleasepool(|_| {
            let cg_image = cg_image(image)
                .ok_or_else(|| OcrError::Failed("couldn't make a CGImage".into()))?;
            let options = NSDictionary::<NSString, AnyObject>::new();
            // SAFETY: the options dictionary is empty, so its values have the right types.
            let handler = unsafe {
                VNImageRequestHandler::initWithCGImage_options(
                    VNImageRequestHandler::alloc(),
                    &cg_image,
                    &options,
                )
            };
            let request = VNRecognizeTextRequest::new();
            request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
            request.setUsesLanguageCorrection(true);
            request.setAutomaticallyDetectsLanguage(true);
            let base: Retained<VNRequest> =
                Retained::into_super(Retained::into_super(request.clone()));
            handler
                .performRequests_error(&NSArray::from_retained_slice(&[base]))
                .map_err(|e| OcrError::Failed(e.localizedDescription().to_string()))?;

            let Some(observations) = request.results() else {
                return Ok(Vec::new());
            };
            Ok(observations
                .iter()
                .filter_map(|observation| observation.topCandidates(1).firstObject())
                .filter(|text| text.confidence() >= MIN_CONFIDENCE)
                .map(|text| text.string().to_string())
                .collect())
        })
    }
}

/// The pixels as an sRGB `CGImage`. Alpha is ignored: the caller has put the image on white.
fn cg_image(image: &RgbaImage) -> Option<CFRetained<CGImage>> {
    let data = CFData::from_bytes(&image.pixels);
    let provider = CGDataProvider::with_cf_data(Some(&data))?;
    let space = CGColorSpace::new_device_rgb()?;
    let width = image.width as usize;
    // SAFETY: the provider holds `width * height * 4` bytes, as the sizes given say, and a
    // null decode array means none.
    unsafe {
        CGImage::new(
            width,
            image.height as usize,
            8,
            32,
            width * 4,
            Some(&space),
            CGBitmapInfo(CGImageAlphaInfo::NoneSkipLast.0),
            Some(&provider),
            ptr::null(),
            false,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
}

/// Low Power Mode, from `NSProcessInfo`. It can be read from any thread.
#[derive(Debug, Clone, Default)]
pub struct MacPowerMode;

impl PowerMode for MacPowerMode {
    fn saving_power(&self) -> bool {
        NSProcessInfo::processInfo().isLowPowerModeEnabled()
    }
}

/// Runs the calling thread at the utility QoS class: long work the user isn't waiting on.
pub(crate) fn lower_thread_priority() {
    // SAFETY: changes only the calling thread's QoS, and takes no pointers.
    unsafe {
        pthread_set_qos_class_self_np(QOS_CLASS_UTILITY, 0);
    }
}

/// `QOS_CLASS_UTILITY` from `<sys/qos.h>`.
const QOS_CLASS_UTILITY: u32 = 0x11;

unsafe extern "C" {
    fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
}
