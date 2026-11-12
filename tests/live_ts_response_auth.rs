#![cfg(feature = "experimental-auth")]
//! Negative response-authentication acceptance through a test-only fault proxy.
//! The proxy forwards to the real pinned TS handlers, never inventing business responses.
use message_box_client::{
    bsv::{primitives::private_key::PrivateKey, wallet::proto_wallet::ProtoWallet},
    transport::AuthTransport,
    *,
};
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

// Fixed test-only control endpoint, not part of the MessageBox client API.
fn counts() -> Value {
    let mut socket = TcpStream::connect("127.0.0.1:5003").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    socket
        .write_all(
            b"GET /_test/counts HTTP/1.1\r\nHost: 127.0.0.1:5003\r\nConnection: close\r\n\r\n",
        )
        .unwrap();
    let mut raw = String::new();
    socket.read_to_string(&mut raw).unwrap();
    let (headers, body) = raw.split_once("\r\n\r\n").unwrap();
    assert!(headers.starts_with("HTTP/1.1 200"));
    serde_json::from_str(body).unwrap()
}
#[tokio::test]
#[ignore = "requires the pinned real TS harness and test-only signature fault proxy documented in interop/README.md"]
async fn invalid_signed_response_rejects_success_without_replaying_write() {
    let recipient_key = PrivateKey::from_random().unwrap();
    let recipient_id = recipient_key.to_public_key().to_der_hex();
    let sender_key = PrivateKey::from_random().unwrap();
    let sender_id = sender_key.to_public_key().to_der_hex();
    let recipient = Client::new(
        "http://127.0.0.1:5002",
        AuthTransport::new(ProtoWallet::new(recipient_key)),
    )
    .unwrap();
    let sender = Client::new(
        "http://127.0.0.1:5003",
        AuthTransport::new(ProtoWallet::new(sender_key)).with_timeout(Duration::from_secs(5)),
    )
    .unwrap();
    recipient
        .set_permission(&SetPermissionRequest {
            message_box: "inbox".into(),
            sender: None,
            recipient_fee: 0,
        })
        .await
        .unwrap();
    let before = counts();
    let id = format!("signature-fault-{sender_id}");
    let result = sender
        .send(&SendRequest {
            message: Message {
                recipient: recipient_id,
                message_box: "inbox".into(),
                message_id: id.clone(),
                body: r#"{"text":"synthetic signed-response test"}"#.into(),
            },
        })
        .await;
    match result {
        Err(Error::Authentication(ref message))
            if message.to_ascii_lowercase().contains("signature") =>
        {
            println!("PASS invalid response signature rejected as Authentication, not timeout or typed success");
        }
        other => panic!("unexpected corrupted-response result: {other:?}"),
    }
    let after = counts();
    assert_eq!(after["upstreamStatus"], 200);
    assert_eq!(
        after["sends"].as_u64().unwrap(),
        before["sends"].as_u64().unwrap() + 1
    );
    assert_eq!(
        after["faults"].as_u64().unwrap(),
        before["faults"].as_u64().unwrap() + 1
    );
    let request = ListRequest {
        message_box: "inbox".into(),
        limit: Some(10),
        ..Default::default()
    };
    let page = recipient.list(&request).await.unwrap();
    assert_eq!(page.messages.len(), 1);
    let stored = &page.messages[0];
    assert_eq!(stored.message_id, id);
    assert_eq!(stored.sender, sender_id);
    assert_eq!(
        serde_json::from_str::<Value>(&stored.body).unwrap()["message"],
        r#"{"text":"synthetic signed-response test"}"#
    );
    println!("PASS exactly one actual send; real recipient read proves content/sender persisted despite response-auth failure");
    recipient
        .acknowledge(&AckRequest {
            message_ids: vec![id],
        })
        .await
        .unwrap();
    assert!(recipient.list(&request).await.unwrap().messages.is_empty());
}
