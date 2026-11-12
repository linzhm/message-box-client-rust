#![cfg(all(feature = "experimental-auth", unix))]
//! Real release binary, private disposable SQLite files, real SDK authentication.
use message_box_client::{
    bsv::{primitives::private_key::PrivateKey, wallet::proto_wallet::ProtoWallet},
    transport::AuthTransport,
    *,
};
use std::{
    fs,
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpStream},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

struct Server {
    child: Option<Child>,
    root: PathBuf,
    binary: PathBuf,
    address: Option<SocketAddr>,
    identity: Option<String>,
}
impl Server {
    fn new() -> Self {
        let binary = PathBuf::from(
            std::env::var("MESSAGEBOX_RUST_SERVER_BIN")
                .expect("set MESSAGEBOX_RUST_SERVER_BIN to the verified c43f8da0 release binary"),
        );
        assert!(binary.is_absolute());
        let key = PrivateKey::from_random().unwrap();
        let root = std::env::temp_dir().join(format!(
            "messagebox-client-rust-live-{}",
            key.to_public_key().to_der_hex()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let identity_file = root.join("identity.key");
        // Only a generated, disposable test identity; never printed or committed.
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&identity_file)
            .unwrap();
        file.write_all(key.to_hex().as_bytes()).unwrap();
        Self {
            child: None,
            root,
            binary,
            address: None,
            identity: None,
        }
    }
    fn start(&mut self) {
        assert!(self.child.is_none());
        let bind = self
            .address
            .map_or_else(|| "127.0.0.1:0".to_string(), |a| a.to_string());
        let config = serde_json::json!({"bind":bind,"dataDir":self.root.join("data"),"identityKeyFile":self.root.join("identity.key"),"freeBoxes":["inbox"],"corsMode":"disabled","enableWebSockets":false});
        let config_file = self.root.join("config.json");
        fs::write(&config_file, serde_json::to_vec(&config).unwrap()).unwrap();
        let child = Command::new(&self.binary)
            .arg(config_file)
            .env_clear()
            .env("ENABLE_FIREBASE", "false")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        self.child = Some(child);
        let output = self.child.as_mut().unwrap().stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(output).read_line(&mut line);
            let _ = tx.send((result, line));
        });
        let (result, line) = rx
            .recv_timeout(Duration::from_secs(30))
            .expect("server startup deadline");
        result.unwrap();
        let ready: serde_json::Value = serde_json::from_str(&line).expect("server ready JSON");
        assert_eq!(ready["status"], "ready");
        let address: SocketAddr = ready["address"].as_str().unwrap().parse().unwrap();
        assert!(address.ip().is_loopback());
        if let Some(previous) = self.address {
            assert_eq!(address, previous);
        }
        let identity = ready["identityKey"].as_str().unwrap().to_owned();
        if let Some(previous) = &self.identity {
            assert_eq!(&identity, previous);
        }
        self.address = Some(address);
        self.identity = Some(identity);
        println!("STARTED real Rust server on {address}; identity stable across restart");
    }
    fn stop(&mut self) {
        let child = self.child.as_mut().unwrap();
        assert!(Command::new("/bin/kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success());
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "server stop: {status}");
                break;
            }
            assert!(Instant::now() < deadline, "graceful shutdown deadline");
            std::thread::sleep(Duration::from_millis(25));
        }
        self.child = None;
        assert!(
            TcpStream::connect_timeout(&self.address.unwrap(), Duration::from_millis(200)).is_err()
        );
        println!("STOPPED gracefully exit=0; listener closed");
    }
    fn origin(&self) -> String {
        format!("http://{}", self.address.unwrap())
    }
    fn cleanup(self) {
        assert!(self.child.is_none());
        fs::remove_dir_all(&self.root).unwrap();
        assert!(!self.root.exists());
        println!("CLEANED owned temporary SQLite/config/identity directory");
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
fn client(origin: &str, key: PrivateKey) -> Client<AuthTransport> {
    Client::new(origin, AuthTransport::new(ProtoWallet::new(key))).unwrap()
}
fn assert_message(page: &ListResponse, id: &str, sender: &str) {
    assert_eq!(page.messages.len(), 1);
    let m = &page.messages[0];
    assert_eq!(m.message_id, id);
    assert_eq!(m.sender, sender);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&m.body).unwrap()["message"],
        r#"{"text":"restart 世界","n":1}"#
    );
}
#[tokio::test]
#[ignore = "requires the verified release Rust server binary; creates loopback listener and disposable SQLite files"]
async fn authenticated_rust_lifecycle_and_restart() {
    let mut server = Server::new();
    server.start();
    let origin = server.origin();
    let sender_key = PrivateKey::from_random().unwrap();
    let recipient_key = PrivateKey::from_random().unwrap();
    let sender_id = sender_key.to_public_key().to_der_hex();
    let recipient_id = recipient_key.to_public_key().to_der_hex();
    let sender = client(&origin, sender_key);
    let recipient = client(&origin, recipient_key.clone());
    recipient
        .set_permission(&SetPermissionRequest {
            message_box: "inbox".into(),
            sender: None,
            recipient_fee: 0,
        })
        .await
        .unwrap();
    assert_eq!(
        recipient
            .permission("inbox", None)
            .await
            .unwrap()
            .permission
            .unwrap()
            .recipient_fee,
        0
    );
    assert_eq!(
        recipient
            .permissions(&ListPermissionsRequest::default())
            .await
            .unwrap()
            .total_count,
        1
    );
    let quote = sender.quote(&recipient_id, "inbox").await.unwrap();
    assert_eq!(quote.quote.delivery_fee, 0);
    assert_eq!(quote.quote.recipient_fee, 0);
    let token = "synthetic-rust-test-token";
    recipient
        .register_device(&RegisterDeviceRequest {
            fcm_token: token.into(),
            device_id: Some("synthetic-device".into()),
            platform: Some(Platform::Web),
        })
        .await
        .unwrap();
    assert_eq!(
        recipient
            .devices(&Pagination::default())
            .await
            .unwrap()
            .devices
            .len(),
        1
    );
    assert!(sender
        .devices(&Pagination::default())
        .await
        .unwrap()
        .devices
        .is_empty());
    let conflict = sender
        .register_device(&RegisterDeviceRequest {
            fcm_token: token.into(),
            device_id: None,
            platform: None,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(conflict, Error::Http { status: 409, .. }),
        "{conflict:?}"
    );
    let send = SendRequest {
        message: Message {
            recipient: recipient_id.clone(),
            message_box: "inbox".into(),
            message_id: "survive-normal-restart".into(),
            body: r#"{"text":"restart 世界","n":1}"#.into(),
        },
    };
    sender.send(&send).await.unwrap();
    let list = ListRequest {
        message_box: "inbox".into(),
        limit: Some(10),
        ..Default::default()
    };
    assert_message(
        &recipient.list(&list).await.unwrap(),
        &send.message.message_id,
        &sender_id,
    );
    assert!(sender.list(&list).await.unwrap().messages.is_empty());
    assert!(sender
        .acknowledge(&AckRequest {
            message_ids: vec![send.message.message_id.clone()]
        })
        .await
        .is_err());
    println!("PASS real auth, send/content/sender, recipient isolation, foreign ack refusal, device/permission APIs");
    server.stop();
    server.start();
    // A stopped listener may invalidate the SDK's pooled connection. Record that
    // narrow failure, then reconnect explicitly; never silently retry a write.
    match recipient.list(&list).await {
        Ok(page) => {
            assert_message(&page, &send.message.message_id, &sender_id);
            println!("RETAINED AuthFetch read passed");
        }
        Err(Error::Authentication(message))
            if message.contains("HTTP request") && message.contains("error sending request") =>
        {
            println!("OBSERVED retained AuthFetch connection error after restart; reconnecting with same identity");
        }
        other => panic!("unexpected retained client result: {other:?}"),
    }
    let recovery_key = recipient_key.clone();
    let fresh = client(&origin, recipient_key);
    assert_message(
        &fresh.list(&list).await.unwrap(),
        &send.message.message_id,
        &sender_id,
    );
    assert_eq!(
        fresh
            .devices(&Pagination::default())
            .await
            .unwrap()
            .devices
            .len(),
        1
    );
    assert_eq!(
        fresh
            .permission("inbox", None)
            .await
            .unwrap()
            .permission
            .unwrap()
            .recipient_fee,
        0
    );
    fresh
        .acknowledge(&AckRequest {
            message_ids: vec![send.message.message_id.clone()],
        })
        .await
        .unwrap();
    assert!(fresh.list(&list).await.unwrap().messages.is_empty());
    println!("PASS unacked message/device/permission persistence; fresh same-identity AuthFetch; ack deletes");
    fresh
        .set_permission(&SetPermissionRequest {
            message_box: "inbox".into(),
            sender: Some(sender_id),
            recipient_fee: -1,
        })
        .await
        .unwrap();
    let mut blocked = send;
    blocked.message.message_id = "blocked-after-restart".into();
    let denied = sender.send(&blocked).await.unwrap_err();
    assert!(
        matches!(denied, Error::Http { status: 403, .. }),
        "{denied:?}"
    );
    server.stop();
    server.start();
    let recovered = client(&origin, recovery_key);
    assert!(recovered.list(&list).await.unwrap().messages.is_empty());
    println!("PASS blocked sender and acknowledged deletion persists after second restart");
    server.stop();
    server.cleanup();
}
