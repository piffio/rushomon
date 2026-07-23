use reqwest::StatusCode;
use serde_json::{Value, json};

mod common;
use common::*;

// The integration test user is provisioned at 'unlimited' tier and owns their
// primary org, so it satisfies the owner + Business-or-above gate on these
// endpoints.

#[tokio::test]
async fn test_list_domains_requires_auth() {
    let client = test_client();
    let org_id = get_primary_test_org_id().await;
    let response = client
        .get(format!("{}/api/orgs/{}/org-domains", BASE_URL, org_id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_add_domain_requires_auth() {
    let client = test_client();
    let org_id = get_primary_test_org_id().await;
    let response = client
        .post(format!("{}/api/orgs/{}/org-domains", BASE_URL, org_id))
        .json(&json!({ "domain": "example.com" }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_add_domain_rejects_empty() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;
    let response = client
        .post(format!("{}/api/orgs/{}/org-domains", BASE_URL, org_id))
        .json(&json!({ "domain": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_domain_lifecycle() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;
    // Unique domain per run to avoid cross-run collisions on the DB.
    let domain = format!("{}.example.test", unique_short_code("jit").to_lowercase());

    // Add a domain challenge
    let res = client
        .post(format!("{}/api/orgs/{}/org-domains", BASE_URL, org_id))
        .json(&json!({ "domain": domain }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["domain"]["domain"], domain);
    assert_eq!(body["domain"]["is_verified"], false);
    assert!(
        body["verification_record"]
            .as_str()
            .unwrap()
            .starts_with("rushomon-verification="),
        "expected a verification TXT record value"
    );

    // List domains should include the new (pending) domain
    let res = client
        .get(format!("{}/api/orgs/{}/org-domains", BASE_URL, org_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    let domains = body["domains"].as_array().unwrap();
    assert!(
        domains.iter().any(|d| d["domain"] == domain),
        "listed domains should include the added domain"
    );

    // Verification should fail — no real TXT record exists for a .test domain
    let res = client
        .post(format!(
            "{}/api/orgs/{}/verify-org-domain",
            BASE_URL, org_id
        ))
        .json(&json!({ "domain": domain }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Delete the domain
    let res = client
        .delete(format!(
            "{}/api/orgs/{}/org-domains/{}",
            BASE_URL, org_id, domain
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Confirm it's gone from the list
    let res = client
        .get(format!("{}/api/orgs/{}/org-domains", BASE_URL, org_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let domains = body["domains"].as_array().unwrap();
    assert!(
        !domains.iter().any(|d| d["domain"] == domain),
        "deleted domain should not appear in the list"
    );
}
