#[derive(Copy, Clone, PartialEq)]
pub enum AudioCodec {
    Copy,
    Aac,
    Ac3,
}

impl AudioCodec {
    pub(crate) fn as_arg(&self) -> Vec<String> {
        let codec = match self {
            AudioCodec::Copy => String::from("copy"),
            AudioCodec::Aac => String::from("aac"),
            AudioCodec::Ac3 => String::from("ac3"),
        };

        vec![String::from("-acodec"), codec]
    }
}
