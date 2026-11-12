use message_box_client::{
    bsv::{primitives::private_key::PrivateKey, wallet::proto_wallet::ProtoWallet},
    transport::AuthTransport,
    Client, ListRequest,
};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ephemeral synthetic identity. No production URL or persisted secret.
    let wallet = ProtoWallet::new(PrivateKey::from_random()?);
    let client = Client::new("http://127.0.0.1:5001", AuthTransport::new(wallet))?;
    let page = client
        .list(&ListRequest {
            message_box: "inbox".into(),
            limit: Some(10),
            ..Default::default()
        })
        .await?;
    println!(
        "{} messages; next offset {}",
        page.messages.len(),
        page.next_offset
    );
    Ok(())
}
