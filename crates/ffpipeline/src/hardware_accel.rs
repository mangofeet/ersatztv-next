#[derive(Debug, Clone, Copy)]
pub enum HardwareAccel {
    Cuda,
    Qsv,
    VideoToolbox,
}

impl HardwareAccel {
    pub(crate) fn as_arg(&self) -> String {
        match self {
            HardwareAccel::Cuda => String::from("cuda"),
            HardwareAccel::Qsv => String::from("qsv"),
            HardwareAccel::VideoToolbox => String::from("videotoolbox"),
        }
    }
}
