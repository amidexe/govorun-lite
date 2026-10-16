//! Existing pinned Sherpa/Silero C ABI; no new model or inference algorithm.
use std::{
    error::Error,
    ffi::{c_char, c_void, CString},
    path::Path,
};

#[repr(C)]
#[derive(Default)]
struct Silero {
    model: *const c_char,
    threshold: f32,
    min_silence: f32,
    min_speech: f32,
    window: i32,
    max_speech: f32,
}
#[repr(C)]
struct Config {
    silero: Silero,
    sample_rate: i32,
    threads: i32,
    provider: *const c_char,
    debug: i32,
    ten_vad: Silero,
}
#[repr(C)]
struct Segment {
    start: i32,
    samples: *const f32,
    count: i32,
}

type Create = unsafe extern "C" fn(*const Config, f32) -> *const c_void;
type Destroy = unsafe extern "C" fn(*const c_void);
type Accept = unsafe extern "C" fn(*const c_void, *const f32, i32);
type Empty = unsafe extern "C" fn(*const c_void) -> i32;
type Front = unsafe extern "C" fn(*const c_void) -> *const Segment;
type DestroySegment = unsafe extern "C" fn(*const Segment);

#[cfg(target_os = "macos")]
extern "C" {
    fn SherpaOnnxCreateVoiceActivityDetector(config: *const Config, seconds: f32) -> *const c_void;
    fn SherpaOnnxDestroyVoiceActivityDetector(detector: *const c_void);
    fn SherpaOnnxVoiceActivityDetectorAcceptWaveform(
        detector: *const c_void,
        samples: *const f32,
        count: i32,
    );
    fn SherpaOnnxVoiceActivityDetectorEmpty(detector: *const c_void) -> i32;
    fn SherpaOnnxVoiceActivityDetectorFront(detector: *const c_void) -> *const Segment;
    fn SherpaOnnxVoiceActivityDetectorPop(detector: *const c_void);
    fn SherpaOnnxVoiceActivityDetectorFlush(detector: *const c_void);
    fn SherpaOnnxVoiceActivityDetectorDetected(detector: *const c_void) -> i32;
    fn SherpaOnnxDestroySpeechSegment(segment: *const Segment);
}

struct Api {
    create: Create,
    destroy: Destroy,
    accept: Accept,
    empty: Empty,
    front: Front,
    pop: Destroy,
    flush: Destroy,
    detected: Empty,
    destroy_segment: DestroySegment,
    #[cfg(windows)]
    _library: libloading::Library,
}
impl Api {
    #[cfg(target_os = "macos")]
    fn load(_: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            create: SherpaOnnxCreateVoiceActivityDetector,
            destroy: SherpaOnnxDestroyVoiceActivityDetector,
            accept: SherpaOnnxVoiceActivityDetectorAcceptWaveform,
            empty: SherpaOnnxVoiceActivityDetectorEmpty,
            front: SherpaOnnxVoiceActivityDetectorFront,
            pop: SherpaOnnxVoiceActivityDetectorPop,
            flush: SherpaOnnxVoiceActivityDetectorFlush,
            detected: SherpaOnnxVoiceActivityDetectorDetected,
            destroy_segment: SherpaOnnxDestroySpeechSegment,
        })
    }
    #[cfg(windows)]
    fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        // Resolve dependencies next to this verified DLL, ahead of Windows'
        // own onnxruntime.dll. PATH alone allows the incompatible system ORT.
        use libloading::os::windows::{
            Library, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
        };
        let library: libloading::Library = unsafe {
            Library::load_with_flags(
                path.join("sherpa-onnx-c-api.dll"),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )?
            .into()
        };
        unsafe {
            Ok(Self {
                create: *library.get(b"SherpaOnnxCreateVoiceActivityDetector\0")?,
                destroy: *library.get(b"SherpaOnnxDestroyVoiceActivityDetector\0")?,
                accept: *library.get(b"SherpaOnnxVoiceActivityDetectorAcceptWaveform\0")?,
                empty: *library.get(b"SherpaOnnxVoiceActivityDetectorEmpty\0")?,
                front: *library.get(b"SherpaOnnxVoiceActivityDetectorFront\0")?,
                pop: *library.get(b"SherpaOnnxVoiceActivityDetectorPop\0")?,
                flush: *library.get(b"SherpaOnnxVoiceActivityDetectorFlush\0")?,
                detected: *library.get(b"SherpaOnnxVoiceActivityDetectorDetected\0")?,
                destroy_segment: *library.get(b"SherpaOnnxDestroySpeechSegment\0")?,
                _library: library,
            })
        }
    }
}

pub struct Vad {
    api: Api,
    detector: *const c_void,
    #[cfg(windows)]
    beginning: Vec<f32>,
    #[cfg(windows)]
    first_pending: bool,
}
impl Vad {
    pub fn new(model: &Path, runtime: &Path) -> Result<Self, Box<dyn Error>> {
        let path = CString::new(model.to_str().ok_or("non-UTF8 model path")?)?;
        let provider = CString::new("cpu")?;
        let config = Config {
            silero: Silero {
                model: path.as_ptr(),
                threshold: 0.5,
                min_silence: 1.0,
                min_speech: if cfg!(windows) { 0.10 } else { 0.25 },
                window: 512,
                // Preserve the existing macOS pause policy on both desktops.
                // This native setting is a soft threshold, not a hard limit.
                max_speech: 30.0,
            },
            sample_rate: 16000,
            threads: 1,
            provider: provider.as_ptr(),
            debug: 0,
            ten_vad: Silero::default(),
        };
        let api = Api::load(runtime)?;
        let detector = unsafe { (api.create)(&config, 60.0) };
        if detector.is_null() {
            return Err("pinned Silero initialization failed".into());
        }
        Ok(Self {
            api,
            detector,
            #[cfg(windows)]
            beginning: Vec::new(),
            #[cfg(windows)]
            first_pending: true,
        })
    }
    pub fn segments(&self, pcm: &[f32]) -> Vec<usize> {
        let mut segments = Vec::new();
        for chunk in pcm.chunks(512) {
            unsafe {
                (self.api.accept)(self.detector, chunk.as_ptr(), chunk.len() as i32);
            }
            self.drain(&mut segments);
        }
        unsafe {
            (self.api.flush)(self.detector);
        }
        self.drain(&mut segments);
        segments
    }
    /// Called on the owning inference thread with exactly one model window.
    pub fn accept(&mut self, pcm: &[f32]) -> Result<Vec<Vec<f32>>, Box<dyn Error>> {
        if pcm.len() != 512 || pcm.iter().any(|sample| !sample.is_finite()) {
            return Err("canonical finite Silero window required".into());
        }
        #[cfg(windows)]
        {
            if self.first_pending && self.beginning.len() < crate::segmentation::SHORT_MAX {
                let count = pcm
                    .len()
                    .min(crate::segmentation::SHORT_MAX - self.beginning.len());
                self.beginning.extend_from_slice(&pcm[..count]);
            }
        }
        unsafe {
            (self.api.accept)(self.detector, pcm.as_ptr(), pcm.len() as i32);
        }
        self.drain_samples()
    }
    pub fn speech_detected(&self) -> bool {
        unsafe { (self.api.detected)(self.detector) != 0 }
    }
    pub fn flush(&mut self) -> Result<Vec<Vec<f32>>, Box<dyn Error>> {
        unsafe {
            (self.api.flush)(self.detector);
        }
        self.drain_samples()
    }
    fn drain_samples(&mut self) -> Result<Vec<Vec<f32>>, Box<dyn Error>> {
        let mut output = Vec::new();
        unsafe {
            while (self.api.empty)(self.detector) == 0 {
                let segment = (self.api.front)(self.detector);
                if segment.is_null() {
                    return Err("Silero segment missing".into());
                }
                let count = (*segment).count;
                let samples = (*segment).samples;
                let valid = count > 0 && !samples.is_null();
                if valid {
                    #[cfg(windows)]
                    let mut pcm = Vec::new();
                    #[cfg(windows)]
                    if self.first_pending {
                        // Native segment.start is an absolute input offset.
                        // Restore only the missing prefix, never duplicate the
                        // detected speech or reassemble subsequent segments.
                        if let Ok(start) = usize::try_from((*segment).start) {
                            if start <= self.beginning.len() {
                                pcm.extend_from_slice(&self.beginning[..start]);
                            }
                        }
                        self.beginning.clear();
                        self.first_pending = false;
                    }
                    #[cfg(windows)]
                    {
                        pcm.extend_from_slice(std::slice::from_raw_parts(samples, count as usize));
                    }
                    if valid {
                        #[cfg(not(windows))]
                        let pcm = std::slice::from_raw_parts(samples, count as usize).to_vec();
                        output.push(pcm);
                    }
                }
                (self.api.destroy_segment)(segment);
                (self.api.pop)(self.detector);
                if !valid {
                    return Err("Silero segment invalid".into());
                }
            }
        }
        Ok(output)
    }
    fn drain(&self, lengths: &mut Vec<usize>) {
        unsafe {
            while (self.api.empty)(self.detector) == 0 {
                let segment = (self.api.front)(self.detector);
                if !segment.is_null() {
                    if (*segment).count > 0 {
                        lengths.push((*segment).count as usize);
                    }
                    (self.api.destroy_segment)(segment);
                }
                (self.api.pop)(self.detector);
            }
        }
    }
}
impl Drop for Vad {
    fn drop(&mut self) {
        unsafe {
            (self.api.destroy)(self.detector);
        }
    }
}
