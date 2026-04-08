use ersatztv_playout::playout::PlayoutItem;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::config::ChannelConfig;
use crate::error::ChannelError;

pub struct PlayoutLoader {
    channel_config: ChannelConfig,
}

impl PlayoutLoader {
    pub fn new(channel_config: &ChannelConfig) -> PlayoutLoader {
        PlayoutLoader {
            channel_config: channel_config.to_owned(),
        }
    }

    pub async fn get_current_item(
        &self,
        now: &OffsetDateTime,
    ) -> Result<PlayoutItem, ChannelError> {
        // TODO: refactor selecting playout file

        log::debug!(
            "playout folder is {}",
            self.channel_config
                .expanded_playout_folder()
                .to_string_lossy()
        );

        // find first playout JSON in folder
        let mut entries = tokio::fs::read_dir(self.channel_config.expanded_playout_folder())
            .await
            .map_err(|e| {
                ChannelError::ChannelConfigFailure(format!(
                    "{}: {:?}",
                    e,
                    self.channel_config.expanded_playout_folder()
                ))
            })?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path =
                entry.path().into_os_string().into_string().map_err(|_| {
                    ChannelError::ChannelConfigFailure(String::from("os string error"))
                })?;

            if let Some(file_name_os) = entry.path().file_stem() {
                let file_name = file_name_os.to_os_string().into_string().map_err(|_| {
                    ChannelError::ChannelConfigFailure(String::from("os string error"))
                })?;

                if path.ends_with(".json") {
                    let split: Vec<&str> = file_name.split("_").collect();
                    if split.len() == 2 {
                        let maybe_start = OffsetDateTime::parse(split[0], &Rfc3339).ok();
                        let maybe_finish = OffsetDateTime::parse(split[1], &Rfc3339).ok();
                        if let (Some(start), Some(finish)) = (maybe_start, maybe_finish)
                            && now >= &start
                            && now <= &finish
                        {
                            log::debug!("playout JSON is {path}");

                            // load playout JSON
                            let playout_result =
                                ersatztv_playout::playout::from_file(&path).await?;

                            // find current item
                            return playout_result
                                .playout
                                .items
                                .into_iter()
                                .rfind(|i| now >= &i.start && now <= &i.finish())
                                .ok_or(ChannelError::PlayoutJsonNoItem);
                        }
                    }
                }
            }
        }

        Err(ChannelError::ChannelConfigFailure(String::from(
            "found no files for the current time",
        )))
    }
}
