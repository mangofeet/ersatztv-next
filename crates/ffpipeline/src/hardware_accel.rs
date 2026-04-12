#[derive(Debug, Clone, Copy)]
pub enum HardwareAccel {
    Cuda,
    Qsv,
    VideoToolbox,
}

impl HardwareAccel {
    pub(crate) fn as_arg(&self) -> Vec<String> {
        match self {
            HardwareAccel::Cuda => vec![
                String::from("-init_hw_device"),
                String::from("cuda"),
                String::from("-hwaccel"),
                String::from("cuda"),
            ],
            HardwareAccel::Qsv => vec![String::from("-hwaccel"), String::from("qsv")],
            HardwareAccel::VideoToolbox => {
                vec![String::from("-hwaccel"), String::from("videotoolbox")]
            }
        }
    }
}
