use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::anyhow;

use crate::model::{CccServerResponse, RequestData, RequestHeader};
use crate::sexpr::from_s_expr;
use crate::{model::CccClientRequest, sexpr, util};

static REQUEST_ID: AtomicU32 = AtomicU32::new(2);

pub(crate) struct SnxHttpAuthenticator {
    server_name: String,
    auth: (String, String),
}

impl SnxHttpAuthenticator {
    pub(crate) fn new(server_name: String, auth: (String, String)) -> Self {
        Self { server_name, auth }
    }

    fn new_request(&self) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst).to_string(),
                request_type: "UserPass".to_string(),
                session_id: String::new(),
            },
            data: RequestData {
                client_type: "TRAC".to_string(),
                endpoint_os: "unix".to_string(),
                username: util::encode_to_hex(&self.auth.0),
                password: util::encode_to_hex(&self.auth.1),
            },
        }
    }

    pub(crate) async fn connect(&self) -> anyhow::Result<CccServerResponse> {
        let expr = sexpr::to_s_expr(CccClientRequest::NAME, self.new_request())?;

        let client = reqwest::Client::new();

        let req = client
            .post(format!("https://{}/clients/", self.server_name))
            .body(expr)
            .build()?;

        let bytes = client
            .execute(req)
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let s_bytes = String::from_utf8_lossy(&bytes);

        let (name, server_response) = from_s_expr::<_, CccServerResponse>(&s_bytes)?;

        if name == CccServerResponse::NAME
            && server_response.data.is_authenticated == "true"
            && server_response.data.active_key.is_some()
        {
            Ok(server_response)
        } else {
            Err(anyhow!("Authentication failed!"))
        }
    }
}