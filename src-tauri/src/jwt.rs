use std::rc::Rc;

use jsonwebtoken::{decode, errors::ErrorKind, Algorithm, DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};

use crate::AppError;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sid: Rc<str>,
    // csrf_token: Rc<str>,
    exp: i64,
    iat: i64,
    iss: Rc<str>,
    aud: Rc<str>,
    auth_type: Rc<str>,
    secure: bool,
    login_ua: Rc<str>,
    login_ip: Rc<str>,
    jwt_identity: Rc<str>,
}

pub fn jwt_is_valid(jwt: &str) -> Result<bool, AppError> {
    match validate_jwt(jwt, None) {
        Ok(_) => Ok(true),
        Err(err) => match *err.kind() {
            ErrorKind::InvalidToken => return Ok(false),
            ErrorKind::InvalidIssuer => return Ok(false),
            ErrorKind::InvalidAudience => return Ok(false),
            _ => return Err(AppError::new(err.to_string().into(), "validate_jwt".into())),
        }
    }
}

fn validate_jwt(token: &str, input_key: Option<&[u8]>) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.sub = None;
    validation.set_audience(&["jwt"]);
    validation.set_issuer(&["jwt"]);
    validation.set_required_spec_claims(&["exp", "iss", "aud"]);
    let key = if input_key.is_none() {
        validation.insecure_disable_signature_validation();
        DecodingKey::from_secret(&[])
    } else {
        DecodingKey::from_secret(input_key.unwrap())
    };

    let token_data = decode::<Claims>(token, &key, &validation)?;
    Ok(token_data)
}

#[cfg(test)]
mod tests {

    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use ::time::OffsetDateTime;

    use crate::jwt::{validate_jwt, Claims};

    // fn stress_test() -> io::Result<()> {
    //     let now_tot = SystemTime::now();
    //     for _ in 0..10000 {
    //         let now = SystemTime::now();
    //         test_validate_jwt()?;
    //         println!("subtime: {}s", now.elapsed().unwrap().as_secs_f32());
    //     }
    //     println!("Total time: {}s", now_tot.elapsed().unwrap().as_secs_f32());
    //     Ok(())
    // }
    #[test]
    fn test_valid_jwt() {
        let key = b"oooo sppokyu key fkjsdn";
        let mut header = Header::new(Algorithm::HS256);
        header.typ = Some("JWT".to_string());
        let sample_claims_valid = Claims {
            sid: "guhh".into(),
            exp: 100000000000,
            iat: 1719973822,
            iss: "jwt".into(),
            aud: "jwt".into(),
            auth_type: "coookie".into(),
            secure: true,
            login_ua: "Rusty Rivens".into(),
            login_ip: "numbershaha".into(),
            jwt_identity: "hi".into(),
        };
        let input_valid = encode(&header, &sample_claims_valid, &EncodingKey::from_secret(key)).unwrap();
        let jwt_valid = validate_jwt(input_valid.as_str(), Some(key)).unwrap();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        // println!("{:?}", jwt.claims);
        // println!("{:?}", jwt.header);
        assert!(jwt_valid.claims.exp > now, "JWT is no longer valid");
    }

    #[test]
    fn test_valid_jwt_no_key() {
        let key = b"oooo sppokyu key fkjsdn";
        let mut header = Header::new(Algorithm::HS256);
        header.typ = Some("JWT".to_string());
        let sample_claims_valid = Claims {
            sid: "guhh".into(),
            exp: 100000000000,
            iat: 1719973822,
            iss: "jwt".into(),
            aud: "jwt".into(),
            auth_type: "coookie".into(),
            secure: true,
            login_ua: "Rusty Rivens".into(),
            login_ip: "numbershaha".into(),
            jwt_identity: "hi".into(),
        };
        let input_valid = encode(&header, &sample_claims_valid, &EncodingKey::from_secret(key)).unwrap();
        let _jwt_no_key = validate_jwt(input_valid.as_str(), None).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_expired_jwt() {
        let key = b"oooo sppokyu key fkjsdn";
        let mut header = Header::new(Algorithm::HS256);
        header.typ = Some("JWT".to_string());
        let sample_claims_expired = Claims {
            sid: "guhh".into(),
            exp: 1719974000,
            iat: 1719973822,
            iss: "jwt".into(),
            aud: "jwt".into(),
            auth_type: "coookie".into(),
            secure: true,
            login_ua: "Rusty Rivens".into(),
            login_ip: "numbershaha".into(),
            jwt_identity: "hi".into(),
        };
        let input_expired = encode(&header, &sample_claims_expired, &EncodingKey::from_secret(key)).unwrap();
        let jwt_expired = validate_jwt(input_expired.as_str(), Some(key));
        assert!(jwt_expired.is_err());
        jwt_expired.unwrap();
    }
}
