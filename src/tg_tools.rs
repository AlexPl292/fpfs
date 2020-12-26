use grammers_client::{ClientHandle, InputMessage};
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::InvocationError;
use grammers_tl_types as tl;

pub async fn resend_message(
    old_message_id: i32,
    message: InputMessage,
    client_handler: &mut ClientHandle,
    peer: &tl::enums::InputPeer,
) -> i32 {
    client_handler
        .delete_messages(None, &[old_message_id])
        .await
        .unwrap();

    // TODO this method should return message instance
    client_handler.send_message(peer, message).await.unwrap();

    last_message(client_handler, &peer).await
}

pub async fn last_message(client_handler: &mut ClientHandle, peer: &tl::enums::InputPeer) -> i32 {
    let mut messages = client_handler.search_messages(&peer);
    messages.next().await.unwrap().unwrap().id()
}

pub async fn edit_or_recreate(
    id: i32,
    message: InputMessage,
    message_again: InputMessage,
    client_handler: &mut ClientHandle,
    peer: &tl::enums::InputPeer,
) -> Option<i32> {
    let result = client_handler.edit_message(&peer, id, message).await;

    match result {
        Ok(_) => None,
        Err(InvocationError::Rpc(RpcError { name, .. })) => {
            if name == "MESSAGE_EDIT_TIME_EXPIRED" {
                let res = resend_message(id, message_again, client_handler, peer).await;
                Some(res)
            } else {
                None
            }
        }
        Err(e) => panic!(e),
    }
}
