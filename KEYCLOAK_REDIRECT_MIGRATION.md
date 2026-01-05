# Keycloak OAuth Redirect URL Migration Summary

## Overview

This document summarizes the changes made to migrate the OAuth callback URL from **port 8080** to **port 8000** across all Keycloak and backend configurations.

## Changes Made

### 1. ✅ Backend Configuration (src/main.rs)

**Location**: Lines 148-149  
**Status**: Already updated  
**Details**: 
- OAuth redirect URL defaults to `http://localhost:8000/api/callback`
- Can be overridden via `KEYCLOAK_REDIRECT_URL` environment variable

```rust
redirect_url: env::var("KEYCLOAK_REDIRECT_URL")
    .unwrap_or_else(|_| "http://localhost:8000/api/callback".to_string()),
```

### 2. ✅ Keycloak Realm Export

**File**: `docker-middleware-stack/configs/keycloak/realm-export.json`  
**Client**: `middleware-app`  
**Changes**:
- Updated `redirectUris` array to use the new URL:
  - `http://localhost:8000/api/callback` (primary)
- Added `webOrigins` array for CORS with frontend domains:
  - `http://localhost:8000`
  - `http://localhost:8080` (legacy)
  - `http://localhost:3000` (React frontend)
  - `http://localhost:5173` (Vite frontend)

**Verification**:
```bash
jq '.clients[0] | {redirectUris, webOrigins}' \
  docker-middleware-stack/configs/keycloak/realm-export.json
```

### 3. ✅ Keycloak Frontend URL

**File**: `.env.keycloak`  
**Variable**: `KC_FRONTEND_URL`  
**Old**: `http://localhost:8080`  
**New**: `http://localhost:8000`  
**Purpose**: Ensures Keycloak generates correct URLs for frontend redirects

### 4. ✅ Environment Variables

**File**: `.env`  
**Variable**: `KEYCLOAK_REDIRECT_URL`  
**Status**: Already set to `http://localhost:8000/api/callback`

## Implementation Checklist

- [ ] **Backend deployed** on port 8000
  ```bash
  APP_PORT=8000 cargo run
  ```

- [ ] **Keycloak realm imported** with updated redirect URIs
  ```bash
  cd docker-middleware-stack
  docker-compose up -d
  ```

- [ ] **Verify Keycloak admin console**
  - Navigate to: `http://localhost:8080/admin`
  - Realm: **middleware**
  - Client: **middleware-app**
  - Check **Settings** tab for correct redirect URIs

- [ ] **Test OAuth flow**
  - Use [OAUTH_FLOW_VERIFICATION.md](OAUTH_FLOW_VERIFICATION.md) guide
  - Verify no "invalid_redirect_uri" errors

- [ ] **Update frontend** (if needed)
  - Point to `http://localhost:8000` for API calls
  - Verify `http://localhost:3000` or `http://localhost:5173` in Keycloak Web Origins

- [ ] **Production deployment**
  - Update external Keycloak client with production redirect URI
  - Update environment variables in production environment

## Files Modified

| File | Changes |
|------|---------|
| `docker-middleware-stack/configs/keycloak/realm-export.json` | Added `redirectUris` and `webOrigins` to `middleware-app` client |
| `.env.keycloak` | Updated `KC_FRONTEND_URL` from 8080 to 8000 |
| `src/main.rs` | Already using port 8000 callback URL (no change needed) |
| `.env` | Already has `KEYCLOAK_REDIRECT_URL=http://localhost:8000/api/callback` |

## New Documentation Files

### 1. KEYCLOAK_REDIRECT_URI_SETUP.md
Complete setup guide covering:
- What changed and why
- Local development setup
- External Keycloak configuration
- Environment variable reference
- Troubleshooting guide
- Migration instructions for existing deployments

### 2. OAUTH_FLOW_VERIFICATION.md
Step-by-step testing guide including:
- Quick verification checklist
- Complete OAuth flow test
- Browser-based testing
- Automated testing script
- Monitoring and debugging
- Common issues and solutions
- Performance considerations
- Validation checklist

## Migration Path

### For Local Development

1. Pull latest code with updated Keycloak realm export
2. Start middleware stack:
   ```bash
   cd docker-middleware-stack
   docker-compose down  # Clean slate if previous config existed
   docker-compose up -d
   ```
3. Start backend:
   ```bash
   cargo run
   ```
4. Verify OAuth flow using [OAUTH_FLOW_VERIFICATION.md](OAUTH_FLOW_VERIFICATION.md)

### For Production (External Keycloak)

1. Log into Keycloak admin console
2. Navigate to your Keycloak realm and client
3. Update **Redirect URIs** in client settings:
   ```
   https://your-domain.com:8000/api/callback
   ```
4. Update **Web Origins** for CORS:
   ```
   https://your-domain.com:8000
   https://your-frontend-domain.com
   ```
5. Update environment variable on application server:
   ```bash
   export KEYCLOAK_REDIRECT_URL=https://your-domain.com:8000/api/callback
   ```
6. Restart application

### For Backward Compatibility

The Keycloak realm export has been updated to use the new redirect URI:

```json
"redirectUris": [
  "http://localhost:8000/api/callback"
]
```

Migration is complete - the old URI has been removed.

## Testing & Verification

### Quick Verification (< 5 minutes)
```bash
# 1. Backend health
curl http://localhost:8000/api/health

# 2. Keycloak health
curl http://localhost:8080/health/ready

# 3. Verify Keycloak config
curl http://localhost:8080/realms/middleware/.well-known/openid-configuration | jq '.issuer'
```

### Full OAuth Flow Test (15-20 minutes)
Follow [OAUTH_FLOW_VERIFICATION.md](OAUTH_FLOW_VERIFICATION.md) for complete testing

## Common Issues & Solutions

### Issue: "invalid_redirect_uri" Error
**Cause**: Redirect URI not registered in Keycloak  
**Fix**: 
1. Check Keycloak admin console for correct URI
2. Verify no typos or trailing slashes
3. Ensure scheme (http vs https) matches

### Issue: CORS Errors in Browser
**Cause**: Web Origins not configured  
**Fix**: Add your frontend domain to Keycloak client Web Origins

### Issue: Token Validation Fails
**Cause**: `KEYCLOAK_ISSUER_URL` doesn't match  
**Fix**: Verify issuer URL matches Keycloak's actual issuer from metadata endpoint

See [KEYCLOAK_REDIRECT_URI_SETUP.md](KEYCLOAK_REDIRECT_URI_SETUP.md#troubleshooting) for detailed troubleshooting.

## Rollback Plan

If issues occur, rollback is straightforward:

1. **Revert Keycloak realm export** to previous version
2. **Restart Keycloak**:
   ```bash
   docker-compose restart keycloak
   ```
3. **Or update client manually** in admin console to use old URIs
4. **Restart backend** if port change needs reverting

## Next Steps

1. **Test locally** using the verification guides
2. **Deploy to staging** and verify OAuth flow
3. **Coordinate with frontend team** to ensure API base URL is updated
4. **Update production Keycloak** client configuration
5. **Deploy production backend** on new port
6. **Monitor logs** for any OAuth-related errors
7. **Coordinate with users** if authentication service is interrupted

## Support & Questions

Refer to these guides for detailed information:
- **Setup & Configuration**: [KEYCLOAK_REDIRECT_URI_SETUP.md](KEYCLOAK_REDIRECT_URI_SETUP.md)
- **Testing & Verification**: [OAUTH_FLOW_VERIFICATION.md](OAUTH_FLOW_VERIFICATION.md)
- **Original OAuth Guide**: [KEYCLOAK_OAUTH2_VERIFICATION.md](KEYCLOAK_OAUTH2_VERIFICATION.md)
