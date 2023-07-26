use crate::credentials::ConstJwtRetriever;
use crate::handle::C8YHttpProxy;
use crate::C8YHttpConfig;
use crate::C8YHttpProxyBuilder;
use c8y_api::json_c8y::C8yUpdateSoftwareListResponse;
use c8y_api::json_c8y::InternalIdResponse;
use std::path::PathBuf;
use tedge_actors::Actor;
use tedge_actors::Builder;
use tedge_actors::ServerActor;
use tedge_actors::ServerMessageBoxBuilder;
use tedge_http_ext::test_helpers::FakeHttpServerBox;
use tedge_http_ext::test_helpers::HttpResponseBuilder;
use tedge_http_ext::HttpRequestBuilder;

#[tokio::test]
async fn c8y_http_proxy_requests_the_device_internal_id_on_start() {
    let c8y_host = "c8y.tenant.io";
    let device_id = "device-001";
    let token = "some JWT token";
    let external_id = "external-device-001";
    let tmp_dir = "/tmp";

    let (mut proxy, mut c8y) =
        spawn_c8y_http_proxy(c8y_host.into(), device_id.into(), tmp_dir.into(), token).await;

    // Even before any request is sent to the c8y_proxy
    // the proxy requests over HTTP the internal device id.
    let init_request = HttpRequestBuilder::get(format!(
        "https://{c8y_host}/identity/externalIds/c8y_Serial/{device_id}"
    ))
    .bearer_auth(token)
    .build()
    .unwrap();
    c8y.assert_recv(Some(init_request)).await;

    // Cumulocity returns the internal device id
    let c8y_response = HttpResponseBuilder::new()
        .status(200)
        .json(&InternalIdResponse::new(device_id, external_id))
        .build()
        .unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();

    // This internal id is then used by the proxy for subsequent requests.
    // For instance, if the proxy upload a log file
    tokio::spawn(async move {
        // NOTE: this is done in the background because this call awaits for the response.
        proxy
            .upload_log_binary("test.log", "some log content", "device-001".into())
            .await
            .unwrap();
    });

    // then the upload request received by c8y is related to the internal id
    c8y.assert_recv(Some(
        HttpRequestBuilder::post(format!("https://{c8y_host}/event/events/"))
            .bearer_auth(token)
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .build()
            .unwrap(),
    ))
    .await;
}

#[tokio::test]
async fn retry_internal_id_on_expired_jwt() {
    let c8y_host = "c8y.tenant.io";
    let device_id = "device-001";
    let token = "JWT token";
    let external_id = "external-device-001";
    let tmp_dir = "/tmp";

    let (mut proxy, mut c8y) =
        spawn_c8y_http_proxy(c8y_host.into(), device_id.into(), tmp_dir.into(), token).await;

    // Even before any request is sent to the c8y_proxy
    // the proxy requests over HTTP the internal device id.
    let init_request = HttpRequestBuilder::get(format!(
        "https://{c8y_host}/identity/externalIds/c8y_Serial/{device_id}"
    ))
    .bearer_auth(token)
    .build()
    .unwrap();
    c8y.assert_recv(Some(init_request)).await;

    // Cumulocity returns unauthorized error (401), because the jwt token has expired
    let c8y_response = HttpResponseBuilder::new().status(401).build().unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();
    c8y.assert_recv(Some(
        HttpRequestBuilder::get(format!(
            "https://{c8y_host}/identity/externalIds/c8y_Serial/{device_id}"
        ))
        .bearer_auth(token)
        .build()
        .unwrap(),
    ))
    .await;
    // Mapper retries to get the internal device id, after getting a fresh jwt token
    let c8y_response = HttpResponseBuilder::new()
        .status(200)
        .json(&InternalIdResponse::new(device_id, external_id))
        .build()
        .unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();

    // This internal id is then used by the proxy for subsequent requests.
    // For instance, if the proxy upload a log file
    tokio::spawn(async move {
        // NOTE: this is done in the background because this call awaits for the response.
        proxy
            .upload_log_binary("test.log", "some log content", "device-001".into())
            .await
            .unwrap();
    });

    // then the upload request received by c8y is related to the internal id
    c8y.assert_recv(Some(
        HttpRequestBuilder::post(format!("https://{c8y_host}/event/events/"))
            .bearer_auth(token)
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .build()
            .unwrap(),
    ))
    .await;
}

#[tokio::test]
async fn retry_software_list_once_with_fresh_internal_id() {
    let c8y_host = "c8y.tenant.io";
    let device_id = "device-001";
    let token = "JWT token";
    let external_id = "external-device-001";
    let tmp_dir = "/tmp";

    let (mut proxy, mut c8y) =
        spawn_c8y_http_proxy(c8y_host.into(), device_id.into(), tmp_dir.into(), token).await;

    // Even before any request is sent to the c8y_proxy
    // the proxy requests over HTTP the internal device id.
    let _init_request = HttpRequestBuilder::get(format!(
        "https://{c8y_host}/identity/externalIds/c8y_Serial/{device_id}"
    ))
    .bearer_auth(token)
    .build()
    .unwrap();
    // skip the message
    c8y.recv().await;

    // Cumulocity returns the internal device id
    let c8y_response = HttpResponseBuilder::new()
        .status(200)
        .json(&InternalIdResponse::new(device_id, external_id))
        .build()
        .unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();

    // This internal id is then used by the proxy for subsequent requests.
    // Create  the  software list and publish
    let c8y_software_list = C8yUpdateSoftwareListResponse::default();

    tokio::spawn(async move {
        // NOTE: this is done in the background because this call awaits for the response.
        proxy
            .send_software_list_http(c8y_software_list, device_id.into())
            .await
    });

    let c8y_software_list = C8yUpdateSoftwareListResponse::default();
    // then the upload request received by c8y is related to the internal id
    c8y.assert_recv(Some(
        HttpRequestBuilder::put(format!(
            "https://{c8y_host}/inventory/managedObjects/{device_id}"
        ))
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .bearer_auth(token)
        .json(&c8y_software_list)
        .build()
        .unwrap(),
    ))
    .await;

    // The software list upload fails because the device identified with internal id not found
    let c8y_response = HttpResponseBuilder::new()
        .status(404)
        .json(&InternalIdResponse::new(device_id, external_id))
        .build()
        .unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();

    // Now the mapper gets a new internal id for the specific device id
    c8y.assert_recv(Some(
        HttpRequestBuilder::get(format!(
            "https://{c8y_host}/identity/externalIds/c8y_Serial/{device_id}"
        ))
        .bearer_auth(token)
        .build()
        .unwrap(),
    ))
    .await;

    // Cumulocity returns the internal device id, after retrying with the fresh jwt token
    let c8y_response = HttpResponseBuilder::new()
        .status(200)
        .json(&InternalIdResponse::new(device_id, external_id))
        .build()
        .unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();

    let c8y_software_list = C8yUpdateSoftwareListResponse::default();
    // then the upload request received by c8y is related to the internal id
    c8y.assert_recv(Some(
        HttpRequestBuilder::put(format!(
            "https://{c8y_host}/inventory/managedObjects/{device_id}"
        ))
        .bearer_auth(token)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .json(&c8y_software_list)
        .build()
        .unwrap(),
    ))
    .await;
}

#[tokio::test]
async fn auto_retry_upload_log_binary_when_internal_id_expires() {
    let c8y_host = "c8y.tenant.io";
    let device_id = "device-001";
    let token = "JWT token";
    let external_id = "external-device-001";
    let tmp_dir = "/tmp";

    let (mut proxy, mut c8y) =
        spawn_c8y_http_proxy(c8y_host.into(), device_id.into(), tmp_dir.into(), token).await;

    // Even before any request is sent to the c8y_proxy
    // the proxy requests over HTTP the internal device id.
    let init_request = HttpRequestBuilder::get(format!(
        "https://{c8y_host}/identity/externalIds/c8y_Serial/{device_id}"
    ))
    .bearer_auth(token)
    .build()
    .unwrap();
    c8y.assert_recv(Some(init_request)).await;
    // Cumulocity returns the internal device id
    let c8y_response = HttpResponseBuilder::new()
        .status(200)
        .json(&InternalIdResponse::new(device_id, external_id))
        .build()
        .unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();
    // This internal id is then used by the proxy for subsequent requests.
    // For instance, if the proxy upload a log file
    tokio::spawn(async move {
        // NOTE: this is done in the background because this call awaits for the response.
        proxy
            .upload_log_binary("test.log", "some log content", "device-001".into())
            .await
            .unwrap();
    });
    // then the upload request received by c8y is related to the internal id
    c8y.assert_recv(Some(
        HttpRequestBuilder::post(format!("https://{c8y_host}/event/events/"))
            .bearer_auth(token)
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .build()
            .unwrap(),
    ))
    .await;

    // Creating the event over http failed due to the device is NOT_FOUND
    let c8y_response = HttpResponseBuilder::new()
        .status(404)
        .json(&InternalIdResponse::new(device_id, external_id))
        .build()
        .unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();

    // Mapper retries the call with internal id
    let c8y_response = HttpResponseBuilder::new()
        .status(200)
        .json(&InternalIdResponse::new(device_id, external_id))
        .build()
        .unwrap();
    c8y.send(Ok(c8y_response)).await.unwrap();

    // Now there is a request to get the internal id
    c8y.assert_recv(Some(
        HttpRequestBuilder::get(format!(
            "https://{c8y_host}/identity/externalIds/c8y_Serial/{device_id}"
        ))
        .bearer_auth(token)
        .build()
        .unwrap(),
    ))
    .await;

    // then the upload request received by c8y is related to the internal id
    c8y.assert_recv(Some(
        HttpRequestBuilder::post(format!("https://{c8y_host}/event/events/"))
            .bearer_auth(token)
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .build()
            .unwrap(),
    ))
    .await;
}

/// Spawn an `C8YHttpProxyActor` instance
/// Return two handles:
/// - one `C8YHttpProxy` to send requests to the actor
/// - one `ServerMessageBoxBuilder<HttpRequest,HttpResponse> to fake the behavior of C8Y REST.
///
/// This also spawns an actor to generate fake JWT tokens.
/// The tests will only check that the http requests include this token.
async fn spawn_c8y_http_proxy(
    c8y_host: String,
    device_id: String,
    tmp_dir: PathBuf,
    token: &str,
) -> (C8YHttpProxy, FakeHttpServerBox) {
    let mut jwt = ServerMessageBoxBuilder::new("JWT Actor", 16);

    let mut http = FakeHttpServerBox::builder();

    let config = C8YHttpConfig {
        c8y_host,
        device_id,
        tmp_dir,
    };
    let mut c8y_proxy_actor = C8YHttpProxyBuilder::new(config, &mut http, &mut jwt);
    let proxy = C8YHttpProxy::new("C8Y", &mut c8y_proxy_actor);

    let mut jwt_actor = ServerActor::new(
        ConstJwtRetriever {
            token: token.to_string(),
        },
        jwt.build(),
    );

    tokio::spawn(async move { jwt_actor.run().await });
    tokio::spawn(async move {
        let mut actor = c8y_proxy_actor.build();
        let _ = actor.run().await;
    });

    (proxy, http.build())
}
