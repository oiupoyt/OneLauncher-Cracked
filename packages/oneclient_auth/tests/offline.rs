use chrono::{TimeDelta, Utc};
use oneclient_auth::{AccountKind, CredentialsStore, offline_account, offline_uuid};
use uuid::Uuid;

fn isolate_launcher_dir() {
    oneclient_common::paths::set_launcher_dir(
        std::env::temp_dir().join(format!("oneclient-auth-test-{}", Uuid::new_v4())),
    );
}

fn fake_microsoft_account(username: &str) -> oneclient_auth::MinecraftAccount {
    let mut account = offline_account(username.to_string());
    account.kind = AccountKind::Microsoft;
    account.access_token = "access".into();
    account.refresh_token = "refresh".into();
    account
}

#[test]
fn offline_uuid_matches_vanilla_algorithm() {
    let id = offline_uuid("Notch");

    assert_eq!(
        id,
        Uuid::parse_str("b50ad385-829d-3141-a216-7e7d7539ba7f").unwrap()
    );
    assert_ne!(id, offline_uuid("Steve"));
}

#[tokio::test]
async fn offline_account_allowed_without_microsoft() {
    isolate_launcher_dir();

    let mut store = CredentialsStore::default();
    let offline = store.add_offline_account("Steve".into()).unwrap();

    assert_eq!(offline.kind, AccountKind::Offline);
    assert_eq!(offline.username, "Steve");
}

#[tokio::test]
async fn offline_account_allowed_when_microsoft_exists() {
    isolate_launcher_dir();

    let mut store = CredentialsStore::default();
    let msa = fake_microsoft_account("MsaUser");
    store.users.insert(msa.id, msa);

    let offline = store.add_offline_account("Steve".into()).unwrap();
    assert_eq!(offline.kind, AccountKind::Offline);
    assert_eq!(offline.username, "Steve");
}

#[tokio::test]
async fn default_offline_works_when_last_microsoft_removed() {
    isolate_launcher_dir();

    let mut store = CredentialsStore::default();
    let msa = fake_microsoft_account("MsaUser");
    let msa_id = msa.id;
    store.users.insert(msa_id, msa);

    let offline = store.add_offline_account("Steve".into()).unwrap();
    let offline_id = offline.id;
    store.default_user = Some(offline_id);
    store.users.remove(&msa_id);

    let account = store.default_account().await.unwrap().unwrap();
    assert_eq!(account.id, offline_id);
    assert_eq!(account.kind, AccountKind::Offline);
}

#[test]
fn token_expiring_imminently_counts_as_expired() {
    let mut account = fake_microsoft_account("MsaUser");

    account.expires = Utc::now() + TimeDelta::hours(1);
    assert!(!account.is_expired(), "a token with an hour left is usable");

    account.expires = Utc::now() + TimeDelta::seconds(5);
    assert!(
        account.is_expired(),
        "a token about to lapse must be renewed before launching"
    );

    account.expires = Utc::now() - TimeDelta::hours(1);
    assert!(account.is_expired());
}

#[tokio::test]
async fn default_account_returns_expired_token_without_refreshing() {
    isolate_launcher_dir();

    let mut store = CredentialsStore::default();
    let mut msa = fake_microsoft_account("MsaUser");
    msa.expires = Utc::now() - TimeDelta::hours(5);
    let msa_id = msa.id;
    store.users.insert(msa_id, msa);

    store.default_user = Some(msa_id);

    let account = store
        .default_account()
        .await
        .expect("an expired token must not fail the read path")
        .expect("the only account should be the default");

    assert_eq!(account.id, msa_id);
    assert!(account.is_expired());
}

#[tokio::test]
async fn offline_account_for_launch_succeeds_without_microsoft() {
    isolate_launcher_dir();

    let (events, _rx) = oneclient_events::EventBus::channel();
    let net = oneclient_net::RequestClient::new(oneclient_net::NetConfig::default()).expect("net client");
    let mut store = CredentialsStore::default();
    let offline = store.add_offline_account("Steve".into()).unwrap();
    let service = oneclient_auth::AuthService::with_store(store, net, events);

    let launched = service
        .account_for_launch(offline.id)
        .await
        .expect("offline account must be allowed for launch without Microsoft account");
    assert_eq!(launched.id, offline.id);
    assert_eq!(launched.username, "Steve");
    assert_eq!(launched.kind, AccountKind::Offline);

    let default_launched = service
        .default_account_for_launch()
        .await
        .expect("default offline account must be allowed for launch")
        .expect("default account should exist");
    assert_eq!(default_launched.id, offline.id);
}
