use async_trait::async_trait;
use message_box_client::{
    transport::{Request, Response, Transport},
    *,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
const KEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
#[derive(Clone)]
struct Recording {
    requests: Arc<Mutex<Vec<Request>>>,
    status: u16,
    body: Vec<u8>,
}
#[async_trait]
impl Transport for Recording {
    async fn execute(&self, r: Request) -> Result<Response, Error> {
        self.requests.lock().unwrap().push(r);
        Ok(Response {
            status: self.status,
            body: self.body.clone(),
        })
    }
}
fn fixture(status: u16, body: Value) -> (Client<Recording>, Arc<Mutex<Vec<Request>>>) {
    let requests = Arc::new(Mutex::new(vec![]));
    let t = Recording {
        requests: requests.clone(),
        status,
        body: serde_json::to_vec(&body).unwrap(),
    };
    (Client::new("http://127.0.0.1:5001", t).unwrap(), requests)
}
fn message() -> SendRequest {
    SendRequest {
        message: Message {
            recipient: KEY.into(),
            message_box: "inbox".into(),
            message_id: "id-1".into(),
            body: r#"{"hello":"世界"}"#.into(),
        },
    }
}
#[tokio::test]
async fn send_preserves_wire_string_and_exact_endpoint() {
    let (c, r) = fixture(
        200,
        json!({"status":"success","message":"sent","results":[{"recipient":KEY,"messageId":"id-1"}]}),
    );
    c.send(&message()).await.unwrap();
    let r = r.lock().unwrap();
    assert_eq!(r[0].url.path(), "/sendMessage");
    assert_eq!(r[0].method, "POST");
    let v: Value = serde_json::from_slice(r[0].body.as_ref().unwrap()).unwrap();
    assert_eq!(
        v,
        json!({"message":{"recipient":KEY,"messageBox":"inbox","messageId":"id-1","body":r#"{"hello":"世界"}"#}})
    );
}
#[tokio::test]
async fn wrong_send_identity_is_rejected() {
    let (c, _) = fixture(
        200,
        json!({"status":"success","message":"sent","results":[]}),
    );
    assert!(matches!(c.send(&message()).await, Err(Error::Protocol(_))));
}
#[tokio::test]
async fn preserves_http_status_error_code_and_extensions() {
    let (c, _) = fixture(
        403,
        json!({"status":"error","code":"ERR_DELIVERY_BLOCKED","description":"blocked","blockedRecipients":[KEY]}),
    );
    match c.send(&message()).await.unwrap_err() {
        Error::Http {
            status,
            error: Some(e),
            ..
        } => {
            assert_eq!(status, 403);
            assert_eq!(e.code, "ERR_DELIVERY_BLOCKED");
            assert_eq!(e.details["blockedRecipients"], json!([KEY]));
        }
        other => panic!("{other:?}"),
    }
}
#[tokio::test]
async fn list_validates_pagination_without_auto_ack() {
    let (c, r) = fixture(
        200,
        json!({"status":"success","messages":[],"limit":10,"offset":3,"nextOffset":3,"hasMore":false}),
    );
    c.list(&ListRequest {
        message_box: "inbox".into(),
        limit: Some(10),
        offset: Some(3),
        message_id: None,
    })
    .await
    .unwrap();
    assert_eq!(r.lock().unwrap().len(), 1);
    let (c, _) = fixture(
        200,
        json!({"status":"success","messages":[],"limit":10,"offset":0,"nextOffset":0,"hasMore":true}),
    );
    assert!(matches!(
        c.list(&ListRequest {
            message_box: "inbox".into(),
            ..Default::default()
        })
        .await,
        Err(Error::Protocol(_))
    ));
}
#[tokio::test]
async fn rejects_invalid_inputs_before_io() {
    let (c, r) = fixture(200, json!({}));
    for name in ["", " inbox", "inbox\n", "a\u{0080}", &"中".repeat(43)] {
        let mut m = message();
        m.message.message_box = name.into();
        assert!(matches!(c.send(&m).await, Err(Error::InvalidInput(_))));
    }
    for (limit, offset) in [(0, 0), (1001, 0), (1, 100001)] {
        assert!(c
            .list(&ListRequest {
                message_box: "inbox".into(),
                limit: Some(limit),
                offset: Some(offset),
                message_id: None
            })
            .await
            .is_err());
    }
    assert!(c
        .acknowledge(&AckRequest {
            message_ids: vec![]
        })
        .await
        .is_err());
    assert!(c
        .acknowledge(&AckRequest {
            message_ids: vec!["id".into(); 1001]
        })
        .await
        .is_err());
    assert!(c
        .set_permission(&SetPermissionRequest {
            message_box: "inbox".into(),
            sender: None,
            recipient_fee: -2
        })
        .await
        .is_err());
    assert!(r.lock().unwrap().is_empty());
}
#[tokio::test]
async fn query_values_cannot_inject_parameters() {
    let (c, r) = fixture(
        200,
        json!({"status":"success","description":"none","permission":null}),
    );
    c.permission("a&sender=forged ?#", None).await.unwrap();
    let r = r.lock().unwrap();
    assert_eq!(r[0].method, "GET");
    assert!(r[0].body.is_none());
    assert_eq!(
        r[0].url.query_pairs().collect::<Vec<_>>(),
        vec![("messageBox".into(), "a&sender=forged ?#".into())]
    );
}
#[tokio::test]
async fn permissions_preserve_null_scope_and_optional_status() {
    let (c, _) = fixture(
        200,
        json!({"status":"success","permissions":[{"sender":null,"messageBox":"inbox","recipientFee":-1,"createdAt":"2026-11-12T00:00:00Z","updatedAt":"2026-11-12T00:00:00Z"}],"totalCount":1}),
    );
    let p = c
        .permissions(&ListPermissionsRequest::default())
        .await
        .unwrap();
    assert!(p.permissions[0].sender.is_none());
    assert!(p.permissions[0].status.is_none());
}
#[tokio::test]
async fn device_and_permission_routes_follow_contract() {
    let (c, r) = fixture(
        200,
        json!({"status":"success","message":"registered","deviceId":1}),
    );
    c.register_device(&RegisterDeviceRequest {
        fcm_token: "synthetic-token".into(),
        device_id: None,
        platform: Some(Platform::Web),
    })
    .await
    .unwrap();
    assert_eq!(r.lock().unwrap()[0].url.path(), "/registerDevice");
    let (c, r) = fixture(
        200,
        json!({"status":"success","limit":100,"offset":0,"devices":[]}),
    );
    c.devices(&Pagination::default()).await.unwrap();
    assert_eq!(r.lock().unwrap()[0].url.path(), "/devices");
    let (c, r) = fixture(200, json!({"status":"success","description":"set"}));
    c.set_permission(&SetPermissionRequest {
        message_box: "inbox".into(),
        sender: None,
        recipient_fee: 0,
    })
    .await
    .unwrap();
    let body: Value = serde_json::from_slice(r.lock().unwrap()[0].body.as_ref().unwrap()).unwrap();
    assert!(body.get("sender").is_none());
    let (c, r) = fixture(200, json!({"status":"success"}));
    c.acknowledge(&AckRequest {
        message_ids: vec!["id-1".into()],
    })
    .await
    .unwrap();
    assert_eq!(r.lock().unwrap()[0].url.path(), "/acknowledgeMessage");
}
#[tokio::test]
async fn success_envelope_and_fee_bounds_are_checked() {
    let (c, _) = fixture(200, json!({"status":"error"}));
    assert!(matches!(
        c.acknowledge(&AckRequest {
            message_ids: vec!["id".into()]
        })
        .await,
        Err(Error::Json(_))
    ));
    let (c, _) = fixture(
        200,
        json!({"status":"success","description":"quote","quote":{"deliveryFee":2147483648u64,"recipientFee":0}}),
    );
    assert!(matches!(
        c.quote(KEY, "inbox").await,
        Err(Error::Protocol(_))
    ));
    let (c, _) = fixture(
        200,
        json!({"status":"success","description":"quote","quote":{"deliveryFee":0,"recipientFee":0.5}}),
    );
    assert!(matches!(c.quote(KEY, "inbox").await, Err(Error::Json(_))));
}
#[tokio::test]
async fn decoder_limit_and_non_json_errors() {
    let (c, _) = fixture(200, json!({"status":"success"}));
    assert!(matches!(
        c.with_response_limit(1)
            .acknowledge(&AckRequest {
                message_ids: vec!["id".into()]
            })
            .await,
        Err(Error::ResponseTooLarge)
    ));
    let requests = Arc::new(Mutex::new(vec![]));
    let c = Client::new(
        "https://example.invalid",
        Recording {
            requests,
            status: 502,
            body: b"upstream unavailable".to_vec(),
        },
    )
    .unwrap();
    assert!(matches!(
        c.acknowledge(&AckRequest {
            message_ids: vec!["id".into()]
        })
        .await,
        Err(Error::Http {
            status: 502,
            error: None,
            ..
        })
    ));
}
#[test]
fn origins_are_explicit_and_constrained() {
    for origin in [
        "http://example.com",
        "https://user:pass@example.com",
        "https://example.com/prefix",
        "https://example.com/?x=1",
        "https://example.com/#a",
    ] {
        let t = Recording {
            requests: Default::default(),
            status: 200,
            body: vec![],
        };
        assert!(Client::new(origin, t).is_err());
    }
}
#[tokio::test]
async fn default_wallet_cannot_create_payments() {
    use bsv::wallet::{
        interfaces::{CreateActionArgs, WalletInterface},
        proto_wallet::ProtoWallet,
    };
    let wallet = ProtoWallet::new(bsv::primitives::private_key::PrivateKey::from_random().unwrap());
    assert!(wallet
        .create_action(
            serde_json::from_value::<CreateActionArgs>(
                json!({"description":"Synthetic test action"})
            )
            .unwrap(),
            None
        )
        .await
        .is_err());
}

#[tokio::test]
async fn canonical_recipient_hex_and_sql_boolean() {
    let (c, _) = fixture(
        200,
        json!({"status":"success","message":"sent","results":[{"recipient":KEY,"messageId":"id-1"}]}),
    );
    let mut request = message();
    request.message.recipient = KEY.to_uppercase();
    c.send(&request).await.unwrap();
    for active in [json!(true), json!(false), json!(0), json!(1)] {
        let value = json!({"id":1,"deviceId":null,"platform":null,"fcmToken":"...synthetic","active":active,"createdAt":"2026-11-12","updatedAt":"2026-11-12","lastUsed":null});
        assert!(serde_json::from_value::<RegisteredDevice>(value).is_ok());
    }
    let value = json!({"id":1,"deviceId":null,"platform":null,"fcmToken":"...synthetic","active":2,"createdAt":"2026-11-12","updatedAt":"2026-11-12","lastUsed":null});
    assert!(serde_json::from_value::<RegisteredDevice>(value).is_err());
}

#[tokio::test]
async fn nonstandard_error_message_is_preserved_without_normalization() {
    let payload = json!({"status":"error","code":"LEGACY_ERROR","message":"synthetic failure"});
    let (client, _) = fixture(400, payload.clone());
    let result = client
        .acknowledge(&AckRequest {
            message_ids: vec!["id".into()],
        })
        .await;
    match result.unwrap_err() {
        Error::Http {
            status: 400,
            error: None,
            body,
        } => {
            assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), payload);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
