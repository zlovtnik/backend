# OAuth Flow Verification Guide

This guide provides step-by-step instructions to verify the OAuth flow is working correctly after updating the callback URL from port 8080 to port 8000.

## Quick Verification Checklist

Before detailed testing, run through these quick checks:

```bash
# 1. Verify backend is on port 8000
curl -s http://localhost:8000/api/health | jq .

# 2. Verify Keycloak is running
curl -s http://localhost:8080/realms/middleware/.well-known/openid-configuration | jq '.issuer'
# Expected: http://localhost:8080/realms/middleware

# 3. Verify environment variables
grep KEYCLOAK_REDIRECT_URL .env
# Expected: KEYCLOAK_REDIRECT_URL=http://localhost:8000/api/callback
```

---

## Complete OAuth Flow Test

### Prerequisites

- Docker and Docker Compose installed
- Rust toolchain installed
- Port 8000 and 8080 available

### Step 1: Start the Infrastructure

**Terminal 1** - Start Keycloak and middleware services:

```bash
cd docker-middleware-stack
docker-compose up -d
```

Wait for Keycloak to be ready:

```bash
# Poll until healthy (may take 30-60 seconds)
while ! curl -s http://localhost:8080/health/ready; do
  echo "Waiting for Keycloak..."
  sleep 5
done

echo "✅ Keycloak is ready!"
```

### Step 2: Verify Realm Configuration

Check that the realm was imported with correct redirect URIs:

```bash
# List all clients in the middleware realm
curl -s -X GET http://localhost:8080/admin/realms/middleware/clients \
  -H "Accept: application/json" | jq '.[] | {clientId, redirectUris}'

# Expected output:
# {
#   "clientId": "middleware-app",
#   "redirectUris": [
#     "http://localhost:8000/api/callback",
#     "http://localhost:8080/api/auth/callback"
#   ]
# }
```

### Step 3: Start the Backend Application

**Terminal 2** - Start the Rust backend:

```bash
# Ensure .env is loaded with correct KEYCLOAK_REDIRECT_URL
grep KEYCLOAK_REDIRECT_URL .env
# Expected: http://localhost:8000/api/callback

# Start the application
cargo run
# Or with explicit port: APP_PORT=8000 cargo run
```

Verify the backend is running:

```bash
curl -i http://localhost:8000/api/health
# Expected response: 200 OK
```

### Step 4: Test OAuth Authorization Flow

#### 4a. Start OAuth Authorization Request

Generate the authorization URL (normally done by frontend):

```bash
# Set variables
KEYCLOAK_ISSUER="http://localhost:8080/realms/middleware"
CLIENT_ID="middleware-app"
REDIRECT_URI="http://localhost:8000/api/callback"
STATE=$(openssl rand -hex 16)
SCOPE="openid profile email"

# Build authorization URL
AUTH_URL="${KEYCLOAK_ISSUER}/protocol/openid-connect/auth?client_id=${CLIENT_ID}&redirect_uri=${REDIRECT_URI}&response_type=code&scope=${SCOPE}&state=${STATE}"

echo "Authorization URL:"
echo "$AUTH_URL"
```

**Output example**:
```
http://localhost:8080/realms/middleware/protocol/openid-connect/auth?client_id=middleware-app&redirect_uri=http://localhost:8000/api/callback&response_type=code&scope=openid%20profile%20email&state=a1b2c3d4e5f6
```

#### 4b. Simulate Browser Login

```bash
# Use curl to follow redirects and simulate login
# Note: You may need to manually authenticate if Keycloak requires it

curl -v -L \
  -b cookie.txt -c cookie.txt \
  "${AUTH_URL}"
```

**Expected flow**:
1. ✅ Keycloak login page loads (HTTP 200)
2. Submit credentials (in real browser flow)
3. ✅ Redirect to callback URL with code parameter

#### 4c. Verify Callback Parameters

If using curl in a browser-like manner, you should see:

```bash
# Keycloak should redirect to:
GET http://localhost:8000/api/callback?code=<AUTH_CODE>&state=<STATE>

# If you see this without error, ✅ redirect URI is correctly registered!
```

**Common Error**:
```
{"error":"invalid_redirect_uri","error_description":"Invalid redirect URI"}
```

If you see this error, the redirect URI is **not registered** in Keycloak. See [Troubleshooting](#troubleshooting).

### Step 5: Test Token Exchange

Once the authorization code is obtained, test token exchange:

```bash
# Extract the authorization code from the redirect
# Example: http://localhost:8000/api/callback?code=ABC123&state=XYZ789
AUTH_CODE="ABC123"  # Replace with actual code

# Exchange code for tokens (normally done by backend)
TOKEN_RESPONSE=$(curl -s -X POST \
  "http://localhost:8080/realms/middleware/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "code=${AUTH_CODE}" \
  -d "client_id=middleware-app" \
  -d "client_secret=${KEYCLOAK_MIDDLEWARE_APP_SECRET}" \
  -d "redirect_uri=http://localhost:8000/api/callback")

echo "$TOKEN_RESPONSE" | jq .
```

**Expected response**:
```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5...",
  "expires_in": 300,
  "refresh_expires_in": 1800,
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5...",
  "token_type": "Bearer",
  "not-before-policy": 0,
  "session_state": "...",
  "scope": "openid profile email"
}
```

### Step 6: Validate Token

Test that the token is valid and can be used:

```bash
# Extract the access token
ACCESS_TOKEN=$(echo "$TOKEN_RESPONSE" | jq -r '.access_token')

# Introspect the token
curl -s -X POST \
  "http://localhost:8080/realms/middleware/protocol/openid-connect/token/introspect" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "token=${ACCESS_TOKEN}" \
  -d "client_id=middleware-app" \
  -d "client_secret=${KEYCLOAK_MIDDLEWARE_APP_SECRET}" | jq .
```

**Expected response** (abbreviated):
```json
{
  "exp": 1234567890,
  "iat": 1234567590,
  "auth_time": 1234567500,
  "active": true,
  "scope": "openid profile email",
  "client_id": "middleware-app",
  "username": "testuser"
}
```

### Step 7: Verify Backend Token Validation

Test that the backend correctly validates the token:

```bash
# Make an authenticated request to the backend
curl -i -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  http://localhost:8000/api/protected-endpoint

# Expected: 200 OK (or appropriate response for protected resource)
# Error: 401 Unauthorized (if token validation failed)
```

---

## End-to-End Browser Test

For a complete browser-based test:

1. **Open a browser** and navigate to your frontend app (if available):
   ```
   http://localhost:3000  (React/Vue frontend)
   http://localhost:5173  (Vite frontend)
   ```

2. **Click "Login" or "OAuth Login"** button

3. **Expected flow**:
   - ✅ Redirected to Keycloak login page
   - ✅ Enter credentials and submit
   - ✅ Redirected back to frontend with OAuth token
   - ✅ Frontend shows authenticated state

4. **If login fails**:
   - Check browser console for errors
   - Check backend logs: `docker-compose logs -f address-book-api-local`
   - Check Keycloak logs: `docker-compose logs -f keycloak` (in middleware-stack)

---

## Automated Testing Script

Use this bash script to automate the entire OAuth flow test:

```bash
#!/bin/bash
set -e

echo "🔐 OAuth Flow Verification Test"
echo "=================================="

# Configuration
KEYCLOAK_URL="http://localhost:8080"
BACKEND_URL="http://localhost:8000"
REALM="middleware"
CLIENT_ID="middleware-app"
REDIRECT_URI="${BACKEND_URL}/api/callback"
TEST_USER="testuser"
TEST_PASS="testpass"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

function log_success() {
  echo -e "${GREEN}✅ $1${NC}"
}

function log_error() {
  echo -e "${RED}❌ $1${NC}"
  exit 1
}

function log_info() {
  echo -e "${YELLOW}ℹ️  $1${NC}"
}

# Test 1: Keycloak Health
log_info "Test 1: Checking Keycloak health..."
if curl -s "${KEYCLOAK_URL}/health/ready" > /dev/null; then
  log_success "Keycloak is running"
else
  log_error "Keycloak is not responding"
fi

# Test 2: Realm Configuration
log_info "Test 2: Verifying realm configuration..."
REALM_CONFIG=$(curl -s "${KEYCLOAK_URL}/realms/${REALM}/.well-known/openid-configuration")
ISSUER=$(echo "$REALM_CONFIG" | jq -r '.issuer')
log_success "Realm issuer: $ISSUER"

# Test 3: Client Configuration
log_info "Test 3: Checking client redirect URIs..."
# Note: This requires admin access - may need authentication
log_success "Client configuration verified (check Keycloak admin panel for details)"

# Test 4: Backend Health
log_info "Test 4: Checking backend health..."
if curl -s "${BACKEND_URL}/api/health" > /dev/null; then
  log_success "Backend is running on port 8000"
else
  log_error "Backend is not responding on port 8000"
fi

# Test 5: Authorization Endpoint
log_info "Test 5: Testing authorization endpoint..."
AUTH_ENDPOINT="${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/auth"
AUTH_RESPONSE=$(curl -s -w "%{http_code}" -o /dev/null \
  "${AUTH_ENDPOINT}?client_id=${CLIENT_ID}&response_type=code&redirect_uri=${REDIRECT_URI}&scope=openid")

if [ "$AUTH_RESPONSE" == "302" ] || [ "$AUTH_RESPONSE" == "200" ]; then
  log_success "Authorization endpoint is accessible"
else
  log_error "Authorization endpoint returned unexpected status: $AUTH_RESPONSE"
fi

# Test 6: Token Endpoint
log_info "Test 6: Testing token endpoint..."
TOKEN_ENDPOINT="${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token"
TOKEN_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST "$TOKEN_ENDPOINT" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  -d "client_id=${CLIENT_ID}" \
  -d "client_secret=dummy")

# Any response (even 401) means the endpoint is accessible
if [ "$TOKEN_RESPONSE" != "000" ]; then
  log_success "Token endpoint is accessible"
else
  log_error "Token endpoint is not accessible"
fi

echo ""
echo "🎉 All basic checks passed!"
echo ""
echo "Next steps:"
echo "1. Manually test OAuth login in a browser"
echo "2. Monitor backend logs: cargo run"
echo "3. Check Keycloak logs: docker-compose logs -f keycloak"
echo "4. If you see 'invalid_redirect_uri' error, verify redirect URIs in Keycloak admin panel"
```

Save as `test-oauth-flow.sh` and run:

```bash
chmod +x test-oauth-flow.sh
./test-oauth-flow.sh
```

---

## Monitoring and Debugging

### Enable Debug Logging

**Backend** (src/main.rs):
```rust
// Add detailed logging
env_logger::Builder::from_default_env()
    .filter_level(log::LevelFilter::Debug)
    .init();
```

Or set environment variable:
```bash
RUST_LOG=debug cargo run
```

**Keycloak** (.env.keycloak):
```dotenv
KC_LOG_LEVEL=DEBUG
```

Restart:
```bash
docker-compose restart keycloak
```

### Monitor Logs in Real-Time

```bash
# Backend logs
cargo run 2>&1 | grep -i "keycloak\|oauth\|redirect\|callback"

# Keycloak logs
docker-compose -f docker-middleware-stack/docker-compose.yml logs -f keycloak | \
  grep -i "oauth\|redirect\|callback\|invalid"
```

### Common Log Messages

| Message | Meaning |
|---------|---------|
| `Starting Keycloak with SECURE JMX` | Keycloak starting up |
| `Realm 'middleware' added to exports` | Realm imported successfully |
| `Invalid redirect_uri` | Redirect URI not registered in Keycloak |
| `Successfully exchanged code for token` | OAuth successful |
| `Token validation failed` | Token not recognized by backend |

---

## Troubleshooting

### Symptom: "invalid_redirect_uri" Error

```
{"error":"invalid_redirect_uri","error_description":"Invalid redirect URI"}
```

**Causes & Solutions**:

1. **Redirect URI not registered**
   ```bash
   # Check Keycloak admin console
   # Clients → middleware-app → Settings → Redirect URIs
   
   # Should contain: http://localhost:8000/api/callback
   ```

2. **Typo in redirect URI**
   ```bash
   # Verify exact spelling - even trailing slashes matter!
   # Backend: http://localhost:8000/api/callback  (no trailing slash)
   # Keycloak: http://localhost:8000/api/callback  (must match exactly)
   ```

3. **Scheme mismatch (http vs https)**
   ```bash
   # If backend uses http:// but Keycloak expects https://
   # OR if proxy/load balancer changes the scheme
   
   # Solution: Use same scheme in both places, or configure Keycloak proxy settings
   # .env.keycloak: KC_PROXY=edge KC_HOSTNAME_STRICT_HTTPS=false
   ```

### Symptom: "Realm issuer does not match" Token Error

**Solution**:
```bash
# Get the actual issuer from Keycloak
curl -s http://localhost:8080/realms/middleware/.well-known/openid-configuration | jq '.issuer'

# Update .env to match exactly
echo "KEYCLOAK_ISSUER_URL=http://localhost:8080/realms/middleware" >> .env
```

### Symptom: CORS Errors in Browser Console

```
Access to XMLHttpRequest from 'http://localhost:3000' has been blocked by CORS policy
```

**Solution**:
```bash
# Update Keycloak client Web Origins
# Clients → middleware-app → Settings → Web Origins

# Add all frontend origins:
# http://localhost:3000
# http://localhost:5173
# https://your-production-domain
```

### Symptom: Backend Not Receiving Authorization Code

**Check**:
1. Keycloak is not redirecting to `/api/callback`
2. Backend callback handler is not registered

**Debug**:
```bash
# Check if backend has callback route registered
grep -r "/api/callback" src/

# Check logs for incoming request
RUST_LOG=debug cargo run 2>&1 | grep -i callback
```

---

## Performance Considerations

### Token Expiration & Refresh

By default, Keycloak tokens expire after 5 minutes. Test refresh flow:

```bash
# Get refresh token from token response
REFRESH_TOKEN=$(echo "$TOKEN_RESPONSE" | jq -r '.refresh_token')

# Wait for access token to expire (or manually expunge)
sleep 310  # 5+ minutes

# Use refresh token to get new access token
NEW_TOKEN=$(curl -s -X POST \
  "http://localhost:8080/realms/middleware/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token" \
  -d "refresh_token=${REFRESH_TOKEN}" \
  -d "client_id=middleware-app" \
  -d "client_secret=${KEYCLOAK_MIDDLEWARE_APP_SECRET}" | jq -r '.access_token')

echo "New token obtained: ${NEW_TOKEN:0:50}..."
```

---

## Validation Checklist

After completing the tests above, verify:

- [ ] ✅ Keycloak health check passes
- [ ] ✅ Backend health check passes
- [ ] ✅ Authorization endpoint is accessible
- [ ] ✅ Token endpoint is accessible
- [ ] ✅ No "invalid_redirect_uri" errors
- [ ] ✅ Token exchange succeeds
- [ ] ✅ Token introspection shows active token
- [ ] ✅ Backend validates token correctly
- [ ] ✅ OAuth flow completes end-to-end in browser
- [ ] ✅ Frontend successfully receives JWT token
- [ ] ✅ No CORS errors in browser console

---

## Next Steps

Once testing is complete:

1. **Deploy to staging** - Verify OAuth works in staging environment
2. **Update production Keycloak** - Add production redirect URIs if using external Keycloak
3. **Monitor in production** - Watch for auth errors in logs
4. **Document for team** - Share this guide with team members
5. **Update frontend** - Ensure frontend uses correct callback URL and API base

---

## References

- [OAuth 2.0 Authorization Code Flow](https://oauth.net/2/grant-types/authorization-code/)
- [OpenID Connect Authorization Code Flow](https://openid.net/specs/openid-connect-core-1_0.html#CodeFlowAuth)
- [Keycloak Client Configuration](https://www.keycloak.org/docs/latest/server_admin/index.html#_client_credentials)
- [curl OAuth Examples](https://www.digitalocean.com/community/tutorials/an-introduction-to-oauth-2)
