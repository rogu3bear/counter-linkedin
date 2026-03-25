use std::sync::{Mutex, OnceLock};

use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header,
    jwk::{JwkSet, KeyAlgorithm},
};
use serde::{Deserialize, Serialize};
use worker::{Fetch, Method, Request, RequestInit};

use super::AppState;

const ACCESS_CERTS_CACHE_TTL_SECS: u64 = 60 * 15;

static ACCESS_CERTS_CACHE: OnceLock<Mutex<Option<CachedAccessCerts>>> = OnceLock::new();

#[derive(Clone)]
struct CachedAccessCerts {
    team_domain: String,
    fetched_at: u64,
    jwks: JwkSet,
}

#[derive(Debug, Clone)]
struct AccessConfig {
    team_domain: String,
    certs_url: String,
    issuer: String,
    audience: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AccessClaims {
    aud: serde_json::Value,
    exp: u64,
    iss: String,
    #[serde(default)]
    nbf: Option<u64>,
    #[serde(default)]
    email: Option<String>,
}

pub async fn authorize(headers: &HeaderMap, state: &AppState) -> Result<(), Response> {
    let config = AccessConfig::from_state(state).ok_or_else(access_required)?;
    let token = extract_access_jwt(headers).ok_or_else(access_required)?;

    let jwks = get_access_jwks(&config).await.map_err(|_| access_required())?;
    validate_access_jwt_against_jwks(&token, &jwks, &config, now_unix_secs())
        .map(|_| ())
        .map_err(|_| access_required())
}

fn extract_access_jwt(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cf-access-jwt-assertion")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn access_required() -> Response {
    (
        axum::http::StatusCode::FORBIDDEN,
        "Cloudflare Access authentication required for metrics.",
    )
        .into_response()
}

async fn get_access_jwks(config: &AccessConfig) -> Result<JwkSet, String> {
    let cache = ACCESS_CERTS_CACHE.get_or_init(|| Mutex::new(None));
    if let Some(cached) = cache
        .lock()
        .map_err(|_| "Failed to lock Access cert cache.".to_string())?
        .as_ref()
        .filter(|cached| {
            cached.team_domain == config.team_domain
                && now_unix_secs().saturating_sub(cached.fetched_at) < ACCESS_CERTS_CACHE_TTL_SECS
        })
        .cloned()
    {
        return Ok(cached.jwks);
    }

    let jwks = fetch_access_jwks(&config.certs_url).await?;
    let mut guard = cache
        .lock()
        .map_err(|_| "Failed to lock Access cert cache.".to_string())?;
    *guard = Some(CachedAccessCerts {
        team_domain: config.team_domain.clone(),
        fetched_at: now_unix_secs(),
        jwks: jwks.clone(),
    });

    Ok(jwks)
}

async fn fetch_access_jwks(certs_url: &str) -> Result<JwkSet, String> {
    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let request = Request::new_with_init(certs_url, &init)
        .map_err(|error| format!("Access cert request failed: {error}"))?;
    let mut response = Fetch::Request(request)
        .send()
        .await
        .map_err(|error| format!("Access cert fetch failed: {error}"))?;

    if response.status_code() != 200 {
        return Err(format!(
            "Access cert fetch returned {}.",
            response.status_code()
        ));
    }

    response
        .json::<JwkSet>()
        .await
        .map_err(|error| format!("Access cert decode failed: {error}"))
}

fn validate_access_jwt_against_jwks(
    token: &str,
    jwks: &JwkSet,
    config: &AccessConfig,
    now_secs: u64,
) -> Result<AccessClaims, String> {
    let header = decode_header(token).map_err(|error| error.to_string())?;
    if header.alg != Algorithm::RS256 {
        return Err("Access token did not use RS256.".to_string());
    }

    let kid = header.kid.ok_or_else(|| "Access token missing kid.".to_string())?;
    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| "Access token key id not found in JWKS.".to_string())?;

    if jwk.common.key_algorithm != Some(KeyAlgorithm::RS256) {
        return Err("Access signing key is not RS256.".to_string());
    }

    let decoding_key = DecodingKey::from_jwk(jwk).map_err(|error| error.to_string())?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_nbf = true;
    validation.leeway = 0;
    validation.set_required_spec_claims(&["exp", "nbf", "iss", "aud"]);
    validation.set_issuer(&[config.issuer.as_str()]);
    validation.set_audience(&[config.audience.as_str()]);

    let claims = decode::<AccessClaims>(token, &decoding_key, &validation)
        .map_err(|error| error.to_string())?
        .claims;

    if claims.exp < now_secs {
        return Err("Access token is expired.".to_string());
    }
    if claims.nbf.is_some_and(|nbf| nbf > now_secs) {
        return Err("Access token is not valid yet.".to_string());
    }

    Ok(claims)
}

impl AccessConfig {
    fn from_state(state: &AppState) -> Option<Self> {
        let raw_team_domain = state.cf_access_team_domain()?;
        let audience = state.cf_access_aud()?;
        let team_domain = normalize_team_domain(&raw_team_domain)?;
        let issuer = format!("https://{team_domain}");
        let certs_url = format!("{issuer}/cdn-cgi/access/certs");

        Some(Self {
            team_domain,
            certs_url,
            issuer,
            audience,
        })
    }
}

fn normalize_team_domain(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let host = without_scheme.split('/').next()?.trim();
    if host.is_empty() {
        return None;
    }

    if host.contains('.') {
        Some(host.to_string())
    } else {
        Some(format!("{host}.cloudflareaccess.com"))
    }
}

fn now_unix_secs() -> u64 {
    (js_sys::Date::now() / 1000.0).floor() as u64
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{EncodingKey, Header, encode};
    use jsonwebtoken::jwk::Jwk;

    use super::*;

    const PRIVATE_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCzpMj2YxM1N7/D
sMru5SkDrTAqxyLK8qeKYVhPWXB/RjS882Gxsl66zhbitHjrtsaZ3HPi6+TpxYL/
F7L3O7Vtx87yyTus/BY4bpRAQdyEX4zfYR4SxgB0qgdfGROFxVKyW/RipNn/rImf
FzYOmAPAVqTeN9chmydUwQwZW+kBFNVwyO8ESDUp2/QqmJ6fBoHr6FvU6BN5Xhqo
LUc+n149Az3LnwsOELqyIbiod729hkCUcaun5zpc8WrJgqXC6/8czXIdNTHDJ8OE
9juVWOiUe778QF+OYWQ7bpExXNc4OfHTwKctyySm0Fl99cyakvvR+U5uisa1Sxvr
BnXL5x+NAgMBAAECgf82FFRJCyPwWNgMd5LB8bIi4KrSnFNUgOtENLSDLq3iTWJf
w7GlRFFiUXUaykqtvBPrTraDMysLvXrlLhsYlpyyK3RlolDdOfUyOwaqc4+qsExA
0LdwIpGaJuq72EgqY4DoDoJQQOnoDgjXdViMYXsmBVW+oO6rDRswaIazX+bE9JkX
Y6YdzSx7CdE5vO2Lp9tz/LiBNiIasHwXB12wRC4NHMnE9Uuit5MG4+2QyQdJpv3f
zyiyHztql4K81XxYLOeU5Dwa3w/5Gma8+5pTfeqKl/oF+gY0tW3L/JX5TFB1Igd5
OUk6d8wtw1pHNaJUD3pUNjM0Em+9UAvXuwK2H4ECgYEA2qXoK1/VyCY7XrIjzw8p
UYCnL58MNDbDGVPnN0ETpTdGVd6fGCATWxBFeGMJFwIvv9bPHudHqKRe1yjoK2ht
l2LCR4PH453x1Pbdt47k28ZHdxRM/xZ8WxQwThtIKK7kIVwe9yafDfHYPyvFvvdG
+BNZF86gFUJDLtCcBLfltSUCgYEA0lUbbSVjEUgEIlL1L9aP5IrGWmkEtwOjPE3q
3ACJ8f0EFUX7FUaHhwP4hoH0pn1IuwFlijBo42umFdrO4oIm/Vm5ZUvJicpO0dmp
/CJDaUM1qRFLzNEkQX6qY4y47cxHWmlIWtERpkZfhyOWNpd3nswO9FwiXexVtB3Q
k3xPGEkCgYEAq+MbhuW7SbKMn+BJeGEB6XnLdQuC65VVgRbNwUleqVav65es2Kl2
rfM3ufGZVsY4RYcYosHNOs2lZV5aTq204fsYomH+BXnIgNRl7wTd88yHqByEf1Dt
CCjx5KVb7+e1nmguS7vH9I14pAjEV2FMIIANXULp5GyIJkiHLspnQiUCgYEAnmbR
/OUHMuCVnHP1i01/mJKax0QH9PycVrInigAt4zy1cn/9lAxFzPzEkigU472+pHds
zSGgHIXZ0uOyowt56ZtE8HCfG1JtAcV3KxdyxTeElgsclud68og+MjKsowoRQpm/
kAWb0Sl2kAPRANQZllH/gTBSAYIXGUrK5gfcWWECgYAjFXwI8RZNZ7jV5FQ7LJBR
xij/uqbbDvIOhriBu6PcplWlcPZCEcajcwiRxA5S3mdEnFz58dubyp1q/K08cZzI
c/GZiu+To/AjTm4YNxc77H7FW/z46sk28X72nPbCXf4vS77hU47kbMFMCzbocEv/
r0RGGHfvyzkG457Q23BzpQ==
-----END PRIVATE KEY-----"#;

    fn test_config() -> AccessConfig {
        AccessConfig {
            team_domain: "counterlinkedin.cloudflareaccess.com".to_string(),
            certs_url: "https://counterlinkedin.cloudflareaccess.com/cdn-cgi/access/certs"
                .to_string(),
            issuer: "https://counterlinkedin.cloudflareaccess.com".to_string(),
            audience: "test-aud".to_string(),
        }
    }

    fn test_jwks() -> JwkSet {
        let encoding_key = EncodingKey::from_rsa_pem(PRIVATE_KEY_PEM.as_bytes()).unwrap();
        let mut jwk = Jwk::from_encoding_key(&encoding_key, Algorithm::RS256).unwrap();
        jwk.common.key_id = Some("test-kid".to_string());
        jwk.common.key_algorithm = Some(KeyAlgorithm::RS256);
        JwkSet { keys: vec![jwk] }
    }

    fn signed_token(iss: &str, aud: &str, exp: u64, nbf: u64) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-kid".to_string());
        let claims = serde_json::json!({
            "iss": iss,
            "aud": aud,
            "exp": exp,
            "nbf": nbf,
            "email": "user@mlnavigator.com"
        });

        encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(PRIVATE_KEY_PEM.as_bytes()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn missing_cloudflare_access_headers_are_forbidden() {
        let headers = HeaderMap::new();
        assert!(extract_access_jwt(&headers).is_none());
    }

    #[test]
    fn access_email_header_alone_does_not_authorize() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "cf-access-authenticated-user-email",
            "user@mlnavigator.com".parse().unwrap(),
        );

        assert!(extract_access_jwt(&headers).is_none());
    }

    #[test]
    fn malformed_token_is_rejected() {
        let config = test_config();

        assert!(validate_access_jwt_against_jwks("definitely-not-a-jwt", &test_jwks(), &config, 100).is_err());
    }

    #[test]
    fn wrong_audience_is_rejected() {
        let config = test_config();
        let token = signed_token(&config.issuer, "wrong-aud", 2_000_000_000, 1);

        assert!(validate_access_jwt_against_jwks(&token, &test_jwks(), &config, 100).is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        let config = test_config();
        let token = signed_token(&config.issuer, &config.audience, 99, 1);

        assert!(validate_access_jwt_against_jwks(&token, &test_jwks(), &config, 100).is_err());
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let config = test_config();
        let token = signed_token("https://wrong.cloudflareaccess.com", &config.audience, 2_000_000_000, 1);

        assert!(validate_access_jwt_against_jwks(&token, &test_jwks(), &config, 100).is_err());
    }

    #[test]
    fn valid_token_is_accepted() {
        let config = test_config();
        let token = signed_token(&config.issuer, &config.audience, 2_000_000_000, 1);

        assert!(validate_access_jwt_against_jwks(&token, &test_jwks(), &config, 100).is_ok());
    }

    #[test]
    fn normalizes_team_domain_inputs() {
        assert_eq!(
            normalize_team_domain("counterlinkedin"),
            Some("counterlinkedin.cloudflareaccess.com".to_string())
        );
        assert_eq!(
            normalize_team_domain("https://counterlinkedin.cloudflareaccess.com/"),
            Some("counterlinkedin.cloudflareaccess.com".to_string())
        );
    }
}
