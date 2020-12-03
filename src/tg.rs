use std::env;
use std::fmt::Error;
use std::fs::read_to_string;

use grammers_client::{Client, ClientHandle, Config, InputMessage};
use grammers_client::types::Entity;
use grammers_session::Session;
use grammers_tl_types as tl;
use tokio::task;

pub struct TgConnection {
    api_id: i32,
    api_hash: String,
}

impl TgConnection {
    pub fn connect(api_id: i32, api_hash: String) -> TgConnection {
        return TgConnection { api_id, api_hash };
    }


    #[tokio::main]
    pub async fn create_file(&self, name: &str) -> Result<(), Error> {
        let mut client = Client::connect(Config {
            session: Session::load_or_create("dialogs.session").unwrap(),
            api_id: self.api_id,
            api_hash: self.api_hash.clone(),
            params: Default::default(),
        }).await.unwrap();

        // Fetch new updates via long poll method


        if !client.is_authorized().await.unwrap() {
            panic!("Panic")
        }

        let mut client_handle = client.handle();
        let network_handle = task::spawn(async move { client.run_until_disconnected().await });

        let peer = tl::types::InputPeerUser { user_id: 1219179532, access_hash: 1901211422175373544 };
        let peer_into = peer.into();

        let meta_message = self.get_meta_message(&client_handle).await;
        let (id, text) = match meta_message {
            Some(data) => data,
            None => {
                client_handle.send_message(&peer_into, InputMessage::text("[META]")).await;
                self.get_meta_message(&client_handle).await.unwrap()
            }
        };

        let new_text = format!("{}\n{}", text, name);

        client_handle.edit_message(&peer_into, id, InputMessage::text(new_text)).await;

        Ok(())
    }

    // #[tokio::main]
    pub async fn get_list(&self) -> Vec<String> {
        let mut client = Client::connect(Config {
            session: Session::load_or_create("dialogs.session").unwrap(),
            api_id: self.api_id,
            api_hash: self.api_hash.clone(),
            params: Default::default(),
        }).await.unwrap();

        // Fetch new updates via long poll method


        if !client.is_authorized().await.unwrap() {
            panic!("Panic")
        }

        let mut client_handle = client.handle();
        let network_handle = task::spawn(async move { client.run_until_disconnected().await });

        let peer = tl::types::InputPeerUser { user_id: 1219179532, access_hash: 1901211422175373544 };
        let peer_into = peer.into();

        let meta_message = self.get_meta_message(&client_handle).await;
        let (id, text) = match meta_message {
            Some(data) => data,
            None => {
                client_handle.send_message(&peer_into, InputMessage::text("[META]")).await;
                self.get_meta_message(&client_handle).await.unwrap()
            }
        };

        let list = TgConnection::crop_letters(&text, 6);
        list.split("\n").map(|x| x.to_string()).collect()
    }

    fn crop_letters(s: &str, pos: usize) -> &str {
        match s.char_indices().skip(pos).next() {
            Some((pos, _)) => &s[pos..],
            None => "",
        }
    }

    async fn get_meta_message(&self, client_handle: &ClientHandle) -> Option<(i32, String)> {
        let peer = tl::types::InputPeerUser { user_id: 1219179532, access_hash: 1901211422175373544 };

        let mut messages = client_handle.search_messages(&peer.into());
        // let result = client_handle.send_message(&peer.into(), "xxx".into()).await.unwrap();

        while let Some(dialog) = messages.next().await.unwrap() {
            if dialog.text().starts_with("[META]") {
                return Some((dialog.id(), dialog.text().to_string()));
            }
        }

        None
    }

    pub async fn get_last_message(self) -> Result<(), Error> {
        Ok(())
    }
}
