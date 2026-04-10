use crate::hardware_accel::HardwareAccel;

pub enum LogLevel {
    Error,
}

impl LogLevel {
    fn as_arg(&self) -> Vec<String> {
        match self {
            LogLevel::Error => vec![String::from("-loglevel"), String::from("error")],
        }
    }
}

pub enum GlobalOption {
    Threads(u32),
    NoStdIn,
    HideBanner,
    LogLevel(LogLevel),
    HardwareAccel(Option<HardwareAccel>),
    StandardFormatFlags,
}

impl GlobalOption {
    pub(crate) fn as_arg(&self) -> Vec<String> {
        match self {
            GlobalOption::Threads(count) => vec![String::from("-threads"), count.to_string()],
            GlobalOption::NoStdIn => vec![String::from("-nostdin")],
            GlobalOption::HideBanner => vec![String::from("-hide_banner")],
            GlobalOption::LogLevel(level) => level.as_arg(),
            GlobalOption::HardwareAccel(Some(hardware_accel)) => hardware_accel.as_arg(),
            GlobalOption::HardwareAccel(None) => Vec::new(),
            GlobalOption::StandardFormatFlags => vec![
                String::from("-fflags"),
                String::from("+genpts+discardcorrupt+igndts"),
            ],
        }
    }
}
