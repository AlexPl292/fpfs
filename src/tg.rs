use grammers_client::{Client, ClientHandle, Config, InputMessage};
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::InvocationError;
use grammers_session::Session;
use grammers_tl_types as tl;
use tokio::task;

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
    pub async fn create_file(&self, name: &str) {
        let mut client_handle = self.get_connection().await;
        let peer_into = TgConnection::get_peer();

        let (id, text) = self
            .get_or_create_meta_message(&mut client_handle, &peer_into)
            .await;

        let new_text = format!("{}\n{}", text, name);

        let edit_message_result = client_handle
            .edit_message(&peer_into, id, new_text.as_str().into())
            .await;

        match edit_message_result {
            Ok(_) => (),
            Err(InvocationError::Rpc(RpcError { name, .. })) => {
                if name == "MESSAGE_EDIT_TIME_EXPIRED" {
                    self.resend_meta_message(id, &new_text, &mut client_handle, &peer_into).await;
                }
            },
            Err(e) => panic!(e),
        }
    }

    // #[tokio::main]
    pub async fn get_files_names(&self) -> Vec<String> {
        let mut client_handle = self.get_connection().await;
        let peer_into = TgConnection::get_peer();

        let (_, text) = self
            .get_or_create_meta_message(&mut client_handle, &peer_into)
            .await;

        let list = utils::crop_letters(&text, META_CONSTANT.len());
        list.split("\n").map(|x| x.to_string()).collect()
    }

    async fn resend_meta_message(
        &self,
        old_message_id: i32,
        message: &str,
        client_handler: &mut ClientHandle,
        peer: &tl::enums::InputPeer,
    ) -> i32 {
        client_handler.delete_messages(None, &[old_message_id]).await.unwrap();

        // TODO this method should return message instance
        client_handler.send_message(peer, message.into()).await.unwrap();
        self.get_meta_message(client_handler).await.unwrap().0
    }

    async fn get_or_create_meta_message(
        &self,
        client_handle: &mut ClientHandle,
        peer: &tl::enums::InputPeer,
    ) -> (i32, String) {
        let meta_message = self.get_meta_message(&client_handle).await;
        match meta_message {
            Some(data) => data,
            None => {
                client_handle
                    .send_message(peer, META_CONSTANT.into())
                    .await
                    .unwrap();
                self.get_meta_message(&client_handle).await.unwrap()
            }
        }
    }

    async fn get_meta_message(&self, client_handle: &ClientHandle) -> Option<(i32, String)> {
        let peer = TgConnection::get_peer();

        let mut messages = client_handle.search_messages(&peer);

        while let Some(message) = messages.next().await.unwrap() {
            if message.text().starts_with(META_CONSTANT) {
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
