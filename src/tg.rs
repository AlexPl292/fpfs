use std::collections::HashMap;
use std::fs::File;

use fuse::FileAttr;
use grammers_client::ext::MessageMediaExt;
use grammers_client::{Client, ClientHandle, Config, InputMessage};
use grammers_session::Session;
use grammers_tl_types as tl;
use tempfile::NamedTempFile;

use crate::serialization::{from_str, to_string};
use crate::tg_tools::{edit_or_recreate, get_message, last_message, resend_message};
use crate::types::{FileLink, MetaMessage, VERSION};
use crate::utils;

const META_CONSTANT: &'static str = "[META]";

pub struct TgConnection {
    client_handler: ClientHandle,
}

impl TgConnection {
    pub async fn connect() -> (TgConnection, Client) {
        let api_id: i32 = env!("TG_ID").parse().expect("TG_ID invalid");
        let api_hash = env!("TG_HASH").to_string();

        let mut client = Client::connect(Config {
            session: Session::load_or_create("dialogs.session").unwrap(),
            api_id,
            api_hash: api_hash.clone(),
            params: Default::default(),
        })
        .await
        .unwrap();

        if !client.is_authorized().await.unwrap() {
            panic!("Panic")
        }

        let client_handler = client.handle();

        return (TgConnection { client_handler }, client);
    }

    #[tokio::main]
    pub async fn check_or_init_meta(&mut self, root_attr: &FileAttr) {
        let (_, meta) = self.get_or_create_meta_message().await;
        if meta.files.is_empty() {
            self.do_create_dir("", root_attr.ino, None, root_attr).await;
            self.edit_meta_message(&|x: &mut MetaMessage| x.next_ino = root_attr.ino + 1)
                .await;
        }
    }

    #[tokio::main]
    pub async fn create_file(&mut self, name: &str, ino: u64, parent: u64, attr: &FileAttr) {
        let mut client_handle = &mut self.client_handler;
        let peer_into = TgConnection::get_peer();

        let new_file_link = FileLink::new_file(name.to_string(), attr.clone());

        let attr_message = to_string(&new_file_link).unwrap();
        let message: InputMessage = attr_message.into();
        client_handle
            .send_message(&peer_into, message)
            .await
            .unwrap();
        let attr_message_id = last_message(&mut client_handle, &peer_into).await;

        let new_text = |text: &mut MetaMessage| {
            text.files.insert(ino.clone(), attr_message_id);
        };

        self.edit_meta_message(&new_text).await;

        self.add_child(ino, &parent).await;
    }

    async fn add_child(&mut self, child: u64, parent: &u64) {
        let peer_into = TgConnection::get_peer();

        let (_, meta) = self.get_meta_message().await.unwrap();
        let parent_id = meta.files.get(&parent).unwrap();

        let mut client_handle = &mut self.client_handler;
        let message = get_message(&mut client_handle, parent_id.clone()).await;
        let mut dir_attrs: FileLink = from_str(&message.text()).unwrap();
        dir_attrs.children.push(child);

        let first_msg = to_string(&dir_attrs).unwrap();
        let second_msg = to_string(&dir_attrs).unwrap();

        edit_or_recreate(
            message.id(),
            first_msg.into(),
            second_msg.into(),
            &mut client_handle,
            &peer_into,
        )
        .await;
    }

    async fn remove_child(&mut self, child: u64, parent: &u64) {
        let peer_into = TgConnection::get_peer();

        let (_, meta) = self.get_meta_message().await.unwrap();
        let parent_id = meta.files.get(&parent).unwrap();

        let mut client_handle = &mut self.client_handler;
        let message = get_message(&mut client_handle, parent_id.clone()).await;
        let mut dir_attrs: FileLink = from_str(&message.text()).unwrap();
        dir_attrs.children.retain(|x| x != &child);

        let first_msg = to_string(&dir_attrs).unwrap();
        let second_msg = to_string(&dir_attrs).unwrap();

        edit_or_recreate(
            message.id(),
            first_msg.into(),
            second_msg.into(),
            &mut client_handle,
            &peer_into,
        )
        .await;
    }

    async fn update_file(&mut self, inode: u64, updater: &dyn Fn(&mut FileLink) -> ()) {
        let peer_into = TgConnection::get_peer();

        let (_, meta) = self.get_meta_message().await.unwrap();
        let parent_id = meta.files.get(&inode).unwrap();

        let mut client_handle = &mut self.client_handler;
        let message = get_message(&mut client_handle, parent_id.clone()).await;
        let mut dir_attrs: FileLink = from_str(&message.text()).unwrap();

        updater(&mut dir_attrs);

        let first_msg = to_string(&dir_attrs).unwrap();
        let second_msg = to_string(&dir_attrs).unwrap();

        if let Some(data) = dir_attrs.file {
            let first_full_msg = InputMessage::from(first_msg).file(data.clone().into());
            let second_msg_full = InputMessage::from(second_msg).file(data.into());

            edit_or_recreate(
                message.id(),
                first_full_msg,
                second_msg_full,
                &mut client_handle,
                &peer_into,
            )
            .await;
        } else {
            edit_or_recreate(
                message.id(),
                first_msg.into(),
                second_msg.into(),
                &mut client_handle,
                &peer_into,
            )
            .await;
        }
    }

    #[tokio::main]
    pub async fn create_dir(&mut self, name: &str, ino: u64, parent: Option<u64>, attr: &FileAttr) {
        self.do_create_dir(name, ino, parent, attr).await
    }

    #[tokio::main]
    pub async fn set_attr(&mut self, ino: u64, attr: FileAttr) {
        self.update_file(ino, &|file: &mut FileLink| file.attr = attr).await;
    }


    #[tokio::main]
    pub async fn rename(&mut self, ino: u64, new_name: &str, parent: u64, new_parent: u64) {
        let updater = |file: &mut FileLink| file.name = new_name.to_string();
        self.update_file(ino, &updater).await;

        self.remove_child(ino, &parent).await;
        self.add_child(ino, &new_parent).await;
    }

    async fn do_create_dir(&mut self, name: &str, ino: u64, parent: Option<u64>, attr: &FileAttr) {
        let mut client_handle = &mut self.client_handler;
        let peer_into = TgConnection::get_peer();

        let new_file_link = FileLink::new_dir(name.to_string(), vec![], attr.clone());

        let attr_message = to_string(&new_file_link).unwrap();
        let message: InputMessage = attr_message.into();
        client_handle
            .send_message(&peer_into, message)
            .await
            .unwrap();
        let attr_message_id = last_message(&mut client_handle, &peer_into).await;

        let new_text = |text: &mut MetaMessage| {
            text.files.insert(ino, attr_message_id);
        };

        if let Some(parent_ino) = parent {
            self.add_child(ino, &parent_ino).await;
        }

        self.edit_meta_message(&new_text).await;
    }

    async fn edit_meta_message<F>(&mut self, f: &dyn Fn(&mut MetaMessage) -> F) -> F {
        let (id, mut meta_message) = self.get_or_create_meta_message().await;

        let res = f(&mut meta_message);

        let new_text = TgConnection::make_meta_string_message(&meta_message);

        let mut client_handle = &mut self.client_handler;
        let peer_into = TgConnection::get_peer();
        edit_or_recreate(
            id,
            new_text.as_str().into(),
            new_text.as_str().into(),
            &mut client_handle,
            &peer_into,
        )
        .await;
        res
    }

    // #[tokio::main]
    pub async fn read_file(&mut self, ino: u64) -> Option<Vec<u8>> {
        let (_, message) = self.get_meta_message().await?;

        let meta_id = message.files.get(&ino)?;

        let client_handle = &mut self.client_handler;

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

    pub async fn get_directory_files(&mut self, parent: &u64) -> Vec<FileLink> {
        let (_, text) = self.get_or_create_meta_message().await;

        let mut client_handle = &mut self.client_handler;

        let directory_msg_id = text.files.get(parent).unwrap();
        let directory_msg = get_message(&mut client_handle, directory_msg_id.clone()).await;
        let directory: FileLink = from_str(directory_msg.text()).unwrap();
        let file_ids: Vec<i32> = directory
            .children
            .iter()
            .map(|x| text.files.get(x).unwrap().clone())
            .collect();

        client_handle
            .get_messages_by_id(None, &file_ids)
            .await
            .unwrap_or(vec![])
            .iter()
            .filter_map(|x| match x {
                None => None,
                Some(t) => from_str(t.text()).unwrap(),
            })
            .collect()
    }

    pub async fn get_file_attr(&mut self, ino: &u64) -> Option<FileLink> {
        let (_, text) = self.get_or_create_meta_message().await;

        let mut client_handle = &mut self.client_handler;

        let file_msg_id = text.files.get(ino)?;
        let message = get_message(&mut client_handle, file_msg_id.clone()).await;
        from_str(message.text()).ok()
    }

    #[tokio::main]
    pub async fn write_to_file(&mut self, tempfile: NamedTempFile, ino: u64) {
        let client_handle = &mut self.client_handler;
        let peer_into = TgConnection::get_peer();

        // Upload file
        let path = tempfile.path().to_str().unwrap();
        let res: tl::enums::InputFile = client_handle.upload_file(path).await.unwrap();

        // Get file message
        let (_, message) = self.get_meta_message().await.unwrap();

        let file_id = message.files.get(&ino).unwrap();

        let mut client_handle = &mut self.client_handler;
        let file_message = get_message(&mut client_handle, file_id.clone()).await;

        let mut result: FileLink = from_str(file_message.text()).unwrap();
        let file = File::open(path).unwrap();
        result.attr.size = file.metadata().unwrap().len();
        result.file = Some(res.clone().into());

        // Update file message
        let message = InputMessage::text(to_string(&result).unwrap()).file(res.clone());

        // TODO Actually we can just modify the existing message, but it's not supported by grammers yet
        let recreated_id =
            resend_message(file_message.id(), message, &mut client_handle, &peer_into).await;

        // Update meta message if needed
        let update = |x: &mut MetaMessage| {
            x.files.insert(ino, recreated_id);
        };
        self.edit_meta_message(&update).await;
    }

    async fn get_or_create_meta_message(&mut self) -> (i32, MetaMessage) {
        let meta_message = self.get_meta_message().await;

        let client_handle = &mut self.client_handler;
        let peer = TgConnection::get_peer();

        match meta_message {
            Some(data) => data,
            None => {
                let meta_message = MetaMessage {
                    version: VERSION.to_string(),
                    files: HashMap::new(),
                    next_ino: 0u64,
                };
                let initial_message = TgConnection::make_meta_string_message(&meta_message);
                client_handle
                    .send_message(&peer, initial_message.into())
                    .await
                    .unwrap();
                self.get_meta_message().await.unwrap()
            }
        }
    }

    fn make_meta_string_message(meta: &MetaMessage) -> String {
        let info = to_string(&meta).unwrap();
        format!("{}\n{}", META_CONSTANT, info)
    }

    pub async fn cleanup(&mut self) {
        let meta_message = self.get_meta_message().await;
        let client_handle = &mut self.client_handler;
        if let Some((id, message)) = meta_message {
            let mut messages_to_delete: Vec<i32> = message.files.values().cloned().collect();
            messages_to_delete.push(id);
            client_handle
                .delete_messages(None, &messages_to_delete)
                .await
                .unwrap();
        }
    }

    pub async fn get_and_inc_ino(&mut self) -> u64 {
        let editor = |msg: &mut MetaMessage| {
            let next_ino = msg.next_ino;
            msg.next_ino = next_ino + 1;
            next_ino
        };
        self.edit_meta_message(&editor).await
    }

    #[tokio::main]
    pub async fn remove_inode(&mut self, file_ino: u64, parent_ino: u64) {
        let (_, message) = self.get_or_create_meta_message().await;

        let file_message_id = message.files.get(&file_ino).unwrap();

        let client_handle = &mut self.client_handler;
        client_handle
            .delete_messages(None, &vec![*file_message_id])
            .await
            .unwrap();

        self.remove_child(file_ino, &parent_ino).await;

        self.edit_meta_message(&|x: &mut MetaMessage| x.files.remove(&file_ino))
            .await;
    }

    async fn get_meta_message(&mut self) -> Option<(i32, MetaMessage)> {
        let client_handle = &mut self.client_handler;
        let (id, text) = TgConnection::find_message_by_text(client_handle, &|msg| {
            msg.starts_with(META_CONSTANT)
        })
        .await?;
        let info = utils::crop_letters(text.as_str(), META_CONSTANT.len());
        let info: MetaMessage = from_str(info).ok()?;
        Some((id, info))
    }

    async fn find_message_by_text(
        client_handle: &mut ClientHandle,
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
}
