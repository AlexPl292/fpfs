use std::collections::HashMap;
use std::fs::File;

use fuse::FileAttr;
use grammers_client::ext::MessageMediaExt;
use grammers_client::{Client, ClientHandle, Config, InputMessage};
use grammers_session::Session;
use grammers_tl_types as tl;
use tempfile::NamedTempFile;
use tokio::task;

use crate::tg_tools::{edit_or_recreate, last_message, resend_message};
use crate::types::{FileLink, MetaMessage, VERSION};
use crate::utils;

const META_CONSTANT: &'static str = "[META]";

pub struct TgConnection {
    api_id: i32,
    api_hash: String,
}

impl TgConnection {
    pub fn connect(api_id: i32, api_hash: String) -> TgConnection {
        return TgConnection { api_id, api_hash };
    }

    #[tokio::main]
    pub async fn create_file(&self, name: &str, ino: u64, attr: &FileAttr) {
        let mut client_handle = self.get_connection().await;
        let peer_into = TgConnection::get_peer();

        let new_file_link = FileLink::new(name.to_string(), attr.clone());

        let attr_message = serde_json::to_string_pretty(&new_file_link).unwrap();
        let message: InputMessage = attr_message.into();
        client_handle
            .send_message(&peer_into, message)
            .await
            .unwrap();
        let attr_message_id = last_message(&mut client_handle, &peer_into).await;

        let new_text = |text: &mut MetaMessage| {
            text.files.insert(ino, attr_message_id);
        };

        self.edit_meta_message(&new_text).await;
    }

    async fn edit_meta_message(&self, f: &dyn Fn(&mut MetaMessage) -> ()) {
        let mut client_handle = self.get_connection().await;
        let peer_into = TgConnection::get_peer();

        let (id, mut meta_message) = self
            .get_or_create_meta_message(&mut client_handle, &peer_into)
            .await;

        f(&mut meta_message);

        let new_text = TgConnection::make_meta_string_message(&meta_message);

        edit_or_recreate(
            id,
            new_text.as_str().into(),
            new_text.as_str().into(),
            &mut client_handle,
            &peer_into,
        )
        .await;
    }

    // #[tokio::main]
    pub async fn read_file(&self, ino: u64) -> Option<Vec<u8>> {
        let mut client_handle = self.get_connection().await;

        let (_, message) = self.get_meta_message(&client_handle).await?;

        let meta_id = message.files.get(&ino)?;

        let file_message = client_handle
            .get_messages_by_id(None, &[meta_id.clone()])
            .await
            .ok()?
            .into_iter()
            .nth(0)??;

        let media: tl::enums::MessageMedia = file_message.media()?;
        let file_location: tl::enums::InputFileLocation = media.to_input_file()?;

        let mut download_iter = client_handle.iter_download(file_location);
        let file = download_iter.next().await.ok()??;

        Some(file)
    }

    // #[tokio::main]
    pub async fn get_files_names(&self) -> Vec<FileLink> {
        let mut client_handle = self.get_connection().await;
        let peer_into = TgConnection::get_peer();

        let (_, text) = self
            .get_or_create_meta_message(&mut client_handle, &peer_into)
            .await;

        let ids: Vec<i32> = text.files.values().map(|x| x.clone()).collect();
        let messages = client_handle
            .get_messages_by_id(None, &ids)
            .await
            .unwrap_or(vec![])
            .iter()
            .filter_map(|x| match x {
                None => None,
                Some(t) => serde_json::from_str(t.text()).unwrap(),
            })
            .collect();

        messages
    }

    #[tokio::main]
    pub async fn write_to_file(&self, tempfile: NamedTempFile, ino: u64) {
        let mut client_handle = self.get_connection().await;
        let peer_into = TgConnection::get_peer();

        // Upload file
        let path = tempfile.path().to_str().unwrap();
        let res: tl::enums::InputFile = client_handle.upload_file(path).await.unwrap();

        // Get file message
        let (_, message) = self.get_meta_message(&mut client_handle).await.unwrap();

        let file_id = message.files.get(&ino).unwrap();

        let file_message = client_handle
            .get_messages_by_id(None, &[file_id.clone()])
            .await
            .unwrap()
            .remove(0)
            .unwrap();

        let mut result: FileLink = serde_json::from_str(file_message.text()).unwrap();
        let file = File::open(path).unwrap();
        result.attr.size = file.metadata().unwrap().len();

        // Update file message
        let message =
            InputMessage::text(serde_json::to_string_pretty(&result).unwrap()).file(res.clone());

        // TODO Actually we can just modify the existing message, but it's not supported by grammers yet
        let recreated_id =
            resend_message(file_message.id(), message, &mut client_handle, &peer_into).await;

        // Update meta message if needed
        let update = |x: &mut MetaMessage| {
            x.files.insert(ino, recreated_id);
        };
        self.edit_meta_message(&update).await;
    }

    async fn get_or_create_meta_message(
        &self,
        client_handle: &mut ClientHandle,
        peer: &tl::enums::InputPeer,
    ) -> (i32, MetaMessage) {
        let meta_message = self.get_meta_message(&client_handle).await;
        match meta_message {
            Some(data) => data,
            None => {
                let meta_message = MetaMessage {
                    version: VERSION.to_string(),
                    files: HashMap::new(),
                };
                let initial_message = TgConnection::make_meta_string_message(&meta_message);
                client_handle
                    .send_message(peer, initial_message.into())
                    .await
                    .unwrap();
                self.get_meta_message(&client_handle).await.unwrap()
            }
        }
    }

    fn make_meta_string_message(meta: &MetaMessage) -> String {
        let info = serde_json::to_string_pretty(&meta).unwrap();
        format!("{}\n{}", META_CONSTANT, info)
    }

    #[tokio::main]
    pub async fn cleanup(&self) {
        let mut client_handle = self.get_connection().await;

        let meta_message = self.get_meta_message(&client_handle).await;
        if let Some((id, message)) = meta_message {
            let mut messages_to_delete: Vec<i32> = message.files.values().cloned().collect();
            messages_to_delete.push(id);
            client_handle
                .delete_messages(None, &messages_to_delete)
                .await
                .unwrap();
        }
    }

    async fn get_meta_message(&self, client_handle: &ClientHandle) -> Option<(i32, MetaMessage)> {
        let (id, text) = TgConnection::find_message_by_text(client_handle, &|msg| {
            msg.starts_with(META_CONSTANT)
        })
        .await?;
        let info = utils::crop_letters(text.as_str(), META_CONSTANT.len());
        let info: MetaMessage = serde_json::from_str(info).ok()?;
        Some((id, info))
    }

    async fn find_message_by_text(
        client_handle: &ClientHandle,
        filter: &dyn Fn(&str) -> bool,
    ) -> Option<(i32, String)> {
        let peer = TgConnection::get_peer();

        let mut messages = client_handle.search_messages(&peer);

        while let Some(message) = messages.next().await.unwrap() {
            if filter(message.text()) {
                return Some((message.id(), message.text().to_string()));
            }
        }

        None
    }

    fn get_peer() -> tl::enums::InputPeer {
        let peer = tl::types::InputPeerUser {
            user_id: 1219179532,
            access_hash: 1901211422175373544,
        };
        let peer_into = peer.into();
        peer_into
    }

    async fn get_connection(&self) -> ClientHandle {
        let mut client = Client::connect(Config {
            session: Session::load_or_create("dialogs.session").unwrap(),
            api_id: self.api_id,
            api_hash: self.api_hash.clone(),
            params: Default::default(),
        })
        .await
        .unwrap();

        if !client.is_authorized().await.unwrap() {
            panic!("Panic")
        }

        let client_handle = client.handle();
        task::spawn(async move { client.run_until_disconnected().await });
        client_handle
    }
}
