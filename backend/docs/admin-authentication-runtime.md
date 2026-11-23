# Admin Authentication Runtime Configuration

Admin authentication uses a dedicated AES-256-GCM key purpose and an isolated HTTP session cookie. The API refuses to start when the Admin TOTP keyring is missing or invalid.

## Required environment variables

- `ADMIN_TOTP_ENCRYPTION_KEY_ACTIVE_VERSION`: active key version, formatted as `vN`, for example `v1`.
- `ADMIN_TOTP_ENCRYPTION_KEYS`: semicolon-separated key entries in the form `vN:HEX`. Each key must be exactly 32 random bytes encoded as 64 hexadecimal characters.

Example format only; replace the placeholder with a secret-manager value:

```dotenv
ADMIN_TOTP_ENCRYPTION_KEY_ACTIVE_VERSION=v1
ADMIN_TOTP_ENCRYPTION_KEYS=v1:<64-hex-character-secret>
```

Generate key material outside the repository and inject it through the deployment secret manager. Never commit, log, print, or reuse this key as `AUTH_RATE_LIMIT_HMAC_KEY`. Restrict access to the API runtime and authorized operators. Do not use all-zero or predictable keys.

## Key rotation

1. Add the new version and key to `ADMIN_TOTP_ENCRYPTION_KEYS`, retaining every version still needed to decrypt stored credentials.
2. Set `ADMIN_TOTP_ENCRYPTION_KEY_ACTIVE_VERSION` to the new version and restart the API.
3. Re-enroll/reset the Admin authenticator through the controlled operator workflow so the stored ciphertext is encrypted with the active key.
4. Verify the new credential and retain the previous key until all ciphertext using it has been re-encrypted and independently verified.
5. Remove an old key only after confirming no stored Admin TOTP credential references that version.

The application does not generate fallback keys or silently continue with an incomplete keyring.

## HTTP boundary

- Password login: `POST /api/v1/identity/admin/login`
- TOTP completion: `POST /api/v1/identity/admin/login/totp`
- Current Admin session: `GET /api/v1/identity/admin/me`
- Logout: `POST /api/v1/identity/admin/logout`

The short-lived challenge and eight-hour Admin session use separate host-only, Secure, HttpOnly, SameSite=Strict cookies. The Admin session cookie is not shared with Business Owner or Investor authentication. State-changing Admin requests require an exact same-origin `Origin` header.

Production must serve the API over HTTPS. The `__Host-` cookie prefix requires Secure, Path=/, and no Domain attribute.
