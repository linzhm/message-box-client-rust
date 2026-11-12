#![cfg(feature = "experimental-auth")]
//! Opt-in real pinned TS routes + real mutual auth; no credentials or production endpoint.
use message_box_client::{
    bsv::{primitives::private_key::PrivateKey, wallet::proto_wallet::ProtoWallet},
    transport::AuthTransport,
    *,
};
fn client() -> (Client<AuthTransport>, String) {
    let k = PrivateKey::from_random().unwrap();
    let identity = k.to_public_key().to_der_hex();
    (
        Client::new(
            "http://127.0.0.1:5002",
            AuthTransport::new(ProtoWallet::new(k)),
        )
        .unwrap(),
        identity,
    )
}
#[tokio::test]
#[ignore = "requires the pinned loopback TS harness documented in interop/README.md"]
async fn real_authenticated_message_lifecycle() {
    let (a, a_key) = client();
    let (b, b_key) = client();
    b.set_permission(&SetPermissionRequest {
        message_box: "inbox".into(),
        sender: None,
        recipient_fee: 0,
    })
    .await
    .unwrap();
    let policy = b.permission("inbox", None).await.unwrap();
    assert_eq!(policy.permission.unwrap().recipient_fee, 0);
    assert_eq!(
        b.permissions(&ListPermissionsRequest::default())
            .await
            .unwrap()
            .total_count,
        1
    );
    assert_eq!(
        a.quote(&b_key, "inbox").await.unwrap().quote.recipient_fee,
        0
    );
    let id = format!("synthetic-{}", a_key);
    a.send(&SendRequest {
        message: Message {
            recipient: b_key.clone(),
            message_box: "inbox".into(),
            message_id: id.clone(),
            body: r#"{"hello":"世界","value":1}"#.into(),
        },
    })
    .await
    .unwrap();
    let request = ListRequest {
        message_box: "inbox".into(),
        limit: Some(10),
        ..Default::default()
    };
    assert!(a.list(&request).await.unwrap().messages.is_empty());
    let page = b.list(&request).await.unwrap();
    assert_eq!(page.messages.len(), 1);
    assert_eq!(page.messages[0].sender, a_key);
    assert_eq!(page.messages[0].message_id, id);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&page.messages[0].body).unwrap()["message"],
        r#"{"hello":"世界","value":1}"#
    );
    // The pinned SQLite route reports a hard duplicate as ERR_DUPLICATE_MESSAGE.
    let duplicate = a
        .send(&SendRequest {
            message: Message {
                recipient: b_key.clone(),
                message_box: "inbox".into(),
                message_id: id.clone(),
                body: r#"{"hello":"世界","value":1}"#.into(),
            },
        })
        .await
        .unwrap_err();
    assert!(
        matches!(duplicate, Error::Http { status: 400, error: Some(ref e), .. } if e.code == "ERR_DUPLICATE_MESSAGE")
    );
    assert_eq!(b.list(&request).await.unwrap().messages.len(), 1);
    // Wrong identity cannot acknowledge the recipient's message.
    assert!(a
        .acknowledge(&AckRequest {
            message_ids: vec![id.clone()]
        })
        .await
        .is_err());
    assert_eq!(b.list(&request).await.unwrap().messages.len(), 1);
    b.acknowledge(&AckRequest {
        message_ids: vec![id],
    })
    .await
    .unwrap();
    assert!(b.list(&request).await.unwrap().messages.is_empty());
    b.set_permission(&SetPermissionRequest {
        message_box: "inbox".into(),
        sender: Some(a_key.clone()),
        recipient_fee: -1,
    })
    .await
    .unwrap();
    let denied = a
        .send(&SendRequest {
            message: Message {
                recipient: b_key,
                message_box: "inbox".into(),
                message_id: "blocked-id".into(),
                body: "{}".into(),
            },
        })
        .await
        .unwrap_err();
    assert!(
        matches!(denied, Error::Http { status: 403, .. }),
        "{denied:?}"
    );
    let token = format!("synthetic-device-{a_key}");
    b.register_device(&RegisterDeviceRequest {
        fcm_token: token.clone(),
        device_id: Some("test-device".into()),
        platform: Some(Platform::Web),
    })
    .await
    .unwrap();
    assert_eq!(
        b.devices(&Pagination::default())
            .await
            .unwrap()
            .devices
            .len(),
        1
    );
    assert!(a
        .devices(&Pagination::default())
        .await
        .unwrap()
        .devices
        .is_empty());
    let conflict = a
        .register_device(&RegisterDeviceRequest {
            fcm_token: token,
            device_id: None,
            platform: None,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(conflict, Error::Http { status: 409, .. }),
        "{conflict:?}"
    );
}

#[tokio::test]
#[ignore = "requires the pinned loopback TS harness documented in interop/README.md"]
async fn real_authenticated_malformed_request() {
    use message_box_client::transport::{Request, Transport};
    let auth = AuthTransport::new(ProtoWallet::new(PrivateKey::from_random().unwrap()));
    let response = auth
        .execute(Request {
            url: "http://127.0.0.1:5002/listMessages".parse().unwrap(),
            method: "POST",
            body: Some(br#"{"messageBox":"inbox","limit":1.5}"#.to_vec()),
        })
        .await
        .unwrap();
    assert_eq!(response.status, 400);
    let error: ServerError = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(error.code, "ERR_INVALID_LIMIT");
}
