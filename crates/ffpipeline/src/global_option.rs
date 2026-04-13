use crate::hw_accel::{HardwareAccel, HwAccel};

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
    StandardFormatFlags,
    InitHwDevice(HardwareAccel),
}

impl GlobalOption {
    pub(crate) fn as_arg(&self) -> Vec<String> {
        match self {
            GlobalOption::Threads(count) => vec![String::from("-threads"), count.to_string()],
            GlobalOption::NoStdIn => vec![String::from("-nostdin")],
            GlobalOption::HideBanner => vec![String::from("-hide_banner")],
            GlobalOption::LogLevel(level) => level.as_arg(),
            GlobalOption::StandardFormatFlags => vec![
                String::from("-fflags"),
                String::from("+genpts+discardcorrupt+igndts"),
            ],
            GlobalOption::InitHwDevice(accel) => accel.init_hw_device(),
        }
    }
}
