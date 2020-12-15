use grammers_client::ext::MessageMediaExt;
use grammers_client::{Client, ClientHandle, Config, InputMessage};
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::InvocationError;
use grammers_session::Session;
use grammers_tl_types as tl;
use tokio::task;

use crate::types::{FileLink, MetaMessage, VERSION};
use crate::utils;
use std::fs::File;
use tempfile::NamedTempFile;

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
    pub async fn create_file(&self, name: &str) {
        let new_text =
            |text: &mut MetaMessage| text.files.push(FileLink::new(name.to_string(), None, 0));

        self.edit_meta_message(&new_text).await
    }

    async fn edit_meta_message(&self, f: &dyn Fn(&mut MetaMessage) -> ()) {
        let mut client_handle = self.get_connection().await;
        let peer_into = TgConnection::get_peer();

        let (id, mut meta_message) = self
            .get_or_create_meta_message(&mut client_handle, &peer_into)
            .await;

        f(&mut meta_message);

        let new_text = TgConnection::make_meta_string_message(&meta_message);

        let edit_message_result = client_handle
            .edit_message(&peer_into, id, new_text.as_str().into())
            .await;

        match edit_message_result {
            Ok(_) => (),
            Err(InvocationError::Rpc(RpcError { name, .. })) => {
                if name == "MESSAGE_EDIT_TIME_EXPIRED" {
                    self.resend_meta_message(id, &new_text, &mut client_handle, &peer_into)
                        .await;
                }
            }
            Err(e) => panic!(e),
        }
    }

    // #[tokio::main]
    pub async fn read_file(&self, name: &str) -> Option<Vec<u8>> {
        let mut client_handle = self.get_connection().await;

        let (_, message) = self.get_meta_message(&client_handle).await?;

        let found_file = message.files.iter().find(|x| x.name == name)?;
        let meta_id = found_file.meta_file_link?;

        let file_meta_message = client_handle
            .get_messages_by_id(None, &[meta_id])
            .await
            .ok()?
            .into_iter()
            .nth(0)??;
        let file_id: i32 = file_meta_message.text().parse().ok()?;
        let file_message = client_handle
            .get_messages_by_id(None, &[file_id])
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

        text.files
    }

    #[tokio::main]
    pub async fn write_to_file(&self, tempfile: NamedTempFile, file_name: &str) {
        let mut client_handle = self.get_connection().await;
        let peer_into = TgConnection::get_peer();

        let path = tempfile.path().to_str().unwrap();

        let res: tl::enums::InputFile = client_handle.upload_file(path).await.unwrap();

        let message = InputMessage::text(path).file(res);
        client_handle
            .send_message(&peer_into, message)
            .await
            .unwrap();

        let (id, _) = TgConnection::find_message_by_text(&client_handle, &|msg| msg == path)
            .await
            .unwrap();

        let id_string = id.to_string();
        client_handle
            .send_message(&peer_into, id_string.as_str().into())
            .await
            .unwrap();

        let (id, _) =
            TgConnection::find_message_by_text(&client_handle, &|msg| msg == id_string.as_str())
                .await
                .unwrap();

        self.edit_meta_message(&|msg| {
            msg.files.retain(|x| x.name != file_name.to_string());
            let file = File::open(path).unwrap();
            msg.files.push(FileLink::new(
                file_name.to_string(),
                Some(id),
                file.metadata().unwrap().len(),
            ))
        })
        .await;
    }

    async fn resend_meta_message(
        &self,
        old_message_id: i32,
        message: &str,
        client_handler: &mut ClientHandle,
        peer: &tl::enums::InputPeer,
    ) -> i32 {
        client_handler
            .delete_messages(None, &[old_message_id])
            .await
            .unwrap();

        // TODO this method should return message instance
        client_handler
            .send_message(peer, message.into())
            .await
            .unwrap();
        self.get_meta_message(client_handler).await.unwrap().0
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
                    files: vec![],
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
    pub async fn remove_meta(&self) {
        let mut client_handle = self.get_connection().await;

        let meta_message = self.get_meta_message(&client_handle).await;
        if let Some((id, _)) = meta_message {
            client_handle.delete_messages(None, &[id]).await.unwrap();
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
