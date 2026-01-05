# Keycloak OAuth Redirect URI Configuration

This guide explains how to configure the Keycloak `middleware-app` client with proper OAuth redirect URIs following the port change from 8080 to 8000.

## Overview

The Rust Actix-web backend application has been updated to run on **port 8000** instead of 8080. The OAuth callback URL has correspondingly changed:

- **Old**: `http://localhost:8080/api/auth/callback`
- **New**: `http://localhost:8000/api/callback`

All Keycloak client configurations must be updated to recognize this new callback URL to prevent "invalid redirect_uri" errors.

## Configuration Changes Made

### 1. Backend Configuration (Already Updated)

**File**: `src/main.rs` (lines 148-149)

```rust
redirect_url: env::var("KEYCLOAK_REDIRECT_URL")
    .unwrap_or_else(|_| "http://localhost:8000/api/callback".to_string()),
```

**Environment Variable**: `.env`

```dotenv
KEYCLOAK_REDIRECT_URL=http://localhost:8000/api/callback
```

### 2. Keycloak Realm Configuration (Updated)

**File**: `docker-middleware-stack/configs/keycloak/realm-export.json`

The `middleware-app` client now uses the new redirect URI:

```json
{
  "clientId": "middleware-app",
  "redirectUris": [
    "http://localhost:8000/api/callback"
  ],
  "webOrigins": [
    "http://localhost:8000",
    "http://localhost:8080",
    "http://localhost:3000",
    "http://localhost:5173"
  ]
}
```

### 3. Keycloak Frontend URL (Already Updated)

**File**: `.env.keycloak`

```dotenv
KC_FRONTEND_URL=http://localhost:8000
```

This ensures the Keycloak frontend generates URLs pointing to the correct application base.

---

## Setup Instructions

### Local Development (Docker Compose)

#### Step 1: Start the Middleware Stack

```bash
cd docker-middleware-stack
docker-compose up -d
```

The middleware stack includes:
- Keycloak (admin console on `http://localhost:8080`)
- PostgreSQL
- Redis
- Kafka
- Prometheus
- Grafana
- OpenTelemetry Collector

#### Step 2: Verify the Realm Configuration

The `realm-export.json` with updated redirect URIs will be automatically imported on startup.

**Via Keycloak Admin Console**:

1. Navigate to `http://localhost:8080/admin`
2. Login with credentials (default: `admin` / `changeme` from `.env.keycloak`)
3. Select realm **"middleware"**
4. Go to **Clients** → **middleware-app**
5. In the **Settings** tab, verify the "Redirect URIs" field contains:
   - `http://localhost:8000/api/callback` ✅ (primary)

#### Step 3: Verify Web Origins

Same location, check **Settings** → **Web Origins**:

```
http://localhost:8000
http://localhost:8080
http://localhost:3000
http://localhost:5173
```

### Production/External Keycloak

If you're using an external Keycloak instance (e.g., Keycloak running on Railway, AWS, Azure):

#### Option A: Using Keycloak Admin Console

1. Log into your Keycloak admin console
2. Navigate to your realm
3. Go to **Clients** → Your application client (e.g., `middleware-app`)
4. In **Settings** tab, under **Redirect URIs**, add:
   - **Primary**: `https://your-app-domain:8000/api/callback`
5. Under **Web Origins**, add:
   - `https://your-app-domain:8000`
   - `https://your-app-domain:8080` (optional, for legacy support)
   - Any frontend origins (e.g., `https://your-frontend-domain`)
6. Click **Save**

#### Option B: Using Keycloak API

```bash
# Get the client by name and extract its ID
CLIENT_ID=$(curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/admin/realms/middleware/clients?clientId=middleware-app" \
  | jq -r '.[0].id')

# Update the client with new redirect URIs
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8080/admin/realms/middleware/clients/$CLIENT_ID" \
  -d '{
    "redirectUris": [
      "http://localhost:8000/api/callback"
    ],
    "webOrigins": [
      "http://localhost:8000",
      "http://localhost:8080",
      "http://localhost:3000",
      "http://localhost:5173"
    ]
  }'
```

---

## Environment Variable Reference

| Variable | Purpose | Example |
|----------|---------|---------|
| `KEYCLOAK_REDIRECT_URL` | OAuth callback URL in backend | `http://localhost:8000/api/callback` |
| `KEYCLOAK_ISSUER_URL` | Keycloak realm issuer (for token validation) | `http://localhost:8080/realms/middleware` |
| `KEYCLOAK_CLIENT_ID` | OAuth client ID registered in Keycloak | `middleware-app` |
| `KEYCLOAK_CLIENT_SECRET` | OAuth client secret (production only) | (sensitive) |
| `KC_FRONTEND_URL` | Keycloak frontend URL (Keycloak config) | `http://localhost:8000` |

---

## OAuth Flow Testing

### 1. Start the Application Stack

```bash
# Terminal 1: Start Keycloak and middleware services
cd docker-middleware-stack
docker-compose up -d

# Terminal 2: Start the Rust backend
cargo run
# Or: APP_PORT=8000 cargo run
```

### 2. Initiate OAuth Flow

Use the browser or curl to start the OAuth authorization flow:

```bash
# Navigate to the OAuth authorization endpoint
# (Backend should construct this URL for the frontend)

curl -i "http://localhost:8080/realms/middleware/protocol/openid-connect/auth?client_id=middleware-app&redirect_uri=http://localhost:8000/api/callback&response_type=code&scope=openid%20profile%20email"
```

### 3. Expect OAuth Redirect

After successful login at Keycloak, expect a redirect to:

```
GET http://localhost:8000/api/callback?code=<authorization_code>&state=<state>
```

✅ **Success**: Backend receives the authorization code  
❌ **Error**: "invalid redirect_uri" means the URL isn't registered in Keycloak

### 4. Verify Tokens

Once the backend exchanges the code for tokens, you should see:

```bash
# Backend logs should show successful token exchange
# Check for JWT tokens in the response

curl -v http://localhost:8000/api/health
# Should include auth header with JWT token
```

---

## Troubleshooting

### "invalid redirect_uri" Error

**Symptom**: After login, Keycloak shows an error: "Invalid redirect_uri"

**Causes**:
1. Redirect URI not registered in Keycloak
2. Typo in the registered URI (query parameters, trailing slash, etc.)
3. Backend and Keycloak using different schemes (http vs https)

**Fix**:
1. Verify the exact URI in your backend code matches Keycloak:
   ```bash
   # Backend
   echo $KEYCLOAK_REDIRECT_URL
   # Should print: http://localhost:8000/api/callback
   ```

2. Verify in Keycloak admin console:
   - **Clients** → **middleware-app** → **Settings**
   - Check exact spelling and scheme in "Redirect URIs" field
   - No trailing slashes unless in your code

3. If still failing, enable Keycloak debug logging:
   ```yaml
   # .env.keycloak
   KC_LOG_LEVEL=DEBUG
   ```

### "Realm issuer does not match" Error

**Symptom**: Token validation fails with "realm issuer does not match"

**Causes**:
1. `KEYCLOAK_ISSUER_URL` doesn't match Keycloak's actual issuer
2. Keycloak behind a proxy with different external/internal URLs

**Fix**:
```bash
# Verify the actual issuer from Keycloak's metadata
curl http://localhost:8080/realms/middleware/.well-known/openid-configuration | jq '.issuer'

# Should output something like: http://localhost:8080/realms/middleware
# Update KEYCLOAK_ISSUER_URL to match exactly
```

### CORS Issues Between Frontend and Keycloak

**Symptom**: Browser console shows CORS errors when frontend calls Keycloak

**Fix**:
1. Update `Web Origins` in Keycloak client:
   - **Clients** → **middleware-app** → **Settings** → **Web Origins**
   - Add your frontend origin: `http://localhost:3000` or `http://localhost:5173`

2. Or, if the issue persists, enable CORS in Keycloak:
   ```yaml
   # .env.keycloak
   KC_HTTP_CORS_ENABLED=true
   KC_HTTP_CORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:5173
   ```

---

## Verification Checklist

After configuration, verify:

- [ ] Backend is running on port **8000**
- [ ] `KEYCLOAK_REDIRECT_URL` in `.env` is set to `http://localhost:8000/api/callback`
- [ ] Keycloak realm exports include the redirect URI for `middleware-app` client
- [ ] `KC_FRONTEND_URL` in `.env.keycloak` is set to `http://localhost:8000`
- [ ] Keycloak admin console shows correct redirect URI (no typos, matching scheme)
- [ ] Web Origins includes all frontend origins (localhost:3000, localhost:5173, etc.)
- [ ] OAuth authorization request redirects correctly to `/api/callback`
- [ ] Backend successfully exchanges code for tokens
- [ ] Tokens are validated without "realm issuer" errors

---

## Migration from Old Configuration

If you were previously running on **port 8080**:

### For Local Development

1. Update `.env`:
   ```diff
   - KEYCLOAK_REDIRECT_URL=http://localhost:8080/api/auth/callback
   + KEYCLOAK_REDIRECT_URL=http://localhost:8000/api/callback
   ```

2. Rebuild and restart:
   ```bash
   # Pull latest realm config with new redirect URIs
   cd docker-middleware-stack
   docker-compose down
   docker-compose up -d
   ```

3. Verify realm import (check Keycloak logs):
   ```bash
   docker-compose logs keycloak | grep -i "realm.*import\|redirect"
   ```

### For Production (External Keycloak)

1. Log into your Keycloak admin console
2. Navigate to **Clients** → Your client
3. Update **Redirect URIs**:
   - Remove old: `https://your-old-domain:8080/api/auth/callback`
   - Add new: `https://your-new-domain:8000/api/callback`
4. Update **Web Origins** similarly
5. Click **Save**

6. Update environment variables on your application server:
   ```bash
   export KEYCLOAK_REDIRECT_URL=https://your-new-domain:8000/api/callback
   ```

7. Restart the application

---

## Additional Resources

- [Keycloak OpenID Connect Configuration](https://www.keycloak.org/docs/latest/server_admin/#_client_credentials)
- [OAuth 2.0 Redirect URI Security](https://tools.ietf.org/html/rfc6749#section-3.1.3)
- [Actix-web Integration Docs](https://actix.rs/)
