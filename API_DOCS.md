# Pact API Documentation

This document describes the REST API endpoints for the Pact (Rust) server.

## Base URL
The server runs by default on `http://127.0.0.1:8002`.

---

## Index
- [System / Health](#system--health)
- [Authentication](#authentication)
- [Vault Management](#vault-management)
- [Sync Management](#sync-management)

---

## System / Health

### Check Health
`GET /health`

Check if the server is up and running.
- **Response**: `200 OK`
- **Body**: `"OK"` (text/plain)

### Get Server Status
`GET /status`

Get the current state of the server, including version and active sessions.
- **Response**: `200 OK`
- **Body**: `StatusResponse` (JSON)
```json
{
  "status": "Running",
  "version": "0.1.0",
  "engine": "Rust/Axum",
  "database": "SQLite (Connected)",
  "active_sessions": 5
}
```

### Get Machine ID
`GET /system/machine-id`

Get the unique identifier for the current device.
- **Response**: `200 OK`
- **Body**: `MachineIdResponse` (JSON)
```json
{
  "machine_id": "pact-rust-dev-device"
}
```

---

## Authentication

### Register User
`POST /auth/register`

Register a new user in the system.
- **Request Body**: `UserRegister` (JSON)
```json
{
  "username": "alice",
  "master_password": "hashed_password",
  "salt": "optional_salt",
  "color": "#ff0000"
}
```
- **Response**: `201 Created` with `AuthResponse` or `400 Bad Request` with `ErrorResponse`.
- **Note**: `username` must be 1–64 chars and cannot contain `/`, `\` or `..`.

### Login
`POST /auth/login`

Authenticate a user and create a session.
- **Request Body**: `UserLogin` (JSON)
```json
{
  "username": "alice",
  "master_password": "hashed_password"
}
```
- **Response**: `200 OK` with `AuthResponse`.
```json
{
  "status": "success",
  "username": "alice",
  "token": "session_token",
  "color": "#ff0000",
  "source": "local"
}
```

### Get Salt
`GET /auth/salt/:username`

Retrieve the salt for a specific user to perform hashing on the client.
- **Response**: `200 OK`
- **Body**: JSON
```json
{ "salt": "a1b2c3d4..." }
```

### Unlock Session
`POST /auth/unlock`

Re-authenticate a session (e.g., after a period of inactivity).
- **Request Body**: `UserLogin` (JSON)
- **Response**: `200 OK`

### Lock Session
`POST /auth/lock`

Terminate the current session.
- **Response**: `200 OK`

### List Users
`GET /auth/users`

List all users registered in the local instance.
- **Response**: `200 OK` with `UsersResponse`.

### Update User
`POST /auth/update`

Update a user's profile (username and color). Requires an authenticated session
(`Authorization: Bearer <token>`). Renaming a user also renames its `<user>.db`
inside the `.pcty` vault.
- **Request Body**: `UserUpdate` (JSON)
```json
{
  "old_name": "alice",
  "new_name": "alice_new",
  "new_color": "#00ff00"
}
```
- **Response**: `200 OK`, `401 Unauthorized` without session.

### Delete User
`DELETE /auth/:username`

Permanently delete a user account and its data. Requires an authenticated session
and master password verification. Removes the user's `<user>.db` from the vault.
- **Request Body**: `MasterPasswordOnly` (JSON)
```json
{
  "master_password": "hashed_password"
}
```
- **Response**: `200 OK`, `401 Unauthorized` or `404 Not Found`.

---

## Vault Management

> [!NOTE]
> Most vault endpoints require an `Authorization` header with a valid session token.

### Get Folders
`GET /vault/folders`

Retrieve all folders for the authenticated user.
- **Response**: `200 OK`
- **Body**: `Array<Folder>`

### Create Folder
`POST /vault/folders`

Create a new folder in the vault.
- **Request Body**: `FolderCreate` (JSON)
```json
{
  "name": "Personal",
  "color": "#3498db",
  "icon": "folder"
}
```
- **Response**: `201 Created` with the new folder `ID`.

### Update Folder
`PUT /vault/folders/:id`

Update an existing folder's metadata.
- **Request Body**: `FolderCreate` (JSON)
- **Response**: `200 OK`.

### Delete Folder
`DELETE /vault/folders/:id`

Delete a folder. Note: Accounts in this folder will have their `folder_id` set to null (soft orphan), they are not deleted.
- **Response**: `200 OK`.

### Get Accounts
`GET /vault/accounts`

Retrieve accounts for the authenticated user. Trashed accounts are hidden by
default.
- **Query params** (all optional): `include_deleted=true` (include trash),
  `is_favorite=true`, `folder_id=<id>`.
- **Response**: `200 OK`
- **Body**: `Array<Account>`

### Create Account
`POST /vault/accounts`

Create a new credential entry.
- **Request Body**: `AccountCreate` (JSON)
```json
{
  "folder_id": 1,
  "title": "GitHub",
  "url": "https://github.com",
  "username_site": "alice",
  "encrypted_pass": "...",
  "totp_seed": null,
  "is_favorite": true,
  "icon_data": "..."
}
```
- **Response**: `201 Created` with the new account `ID`.

### Update Account
`PUT /vault/accounts/:id`

Update a credential entry.
- **Request Body**: `AccountCreate` (JSON)
- **Response**: `200 OK`.

### Toggle Favorite
`PATCH /vault/accounts/:id/favorite`

Toggle the favorite status of an account.
- **Response**: `200 OK` with `{ "is_favorite": boolean }`.

### Soft Delete Account
`DELETE /vault/accounts/:id`

Move an account to the trash (soft delete).
- **Response**: `200 OK`.

### Restore Account
`POST /vault/accounts/:id/restore`

Restore a soft-deleted account from the trash.
- **Response**: `200 OK`.

### Empty Trash
`DELETE /vault/accounts/trash/empty`

Permanently delete all accounts in the trash for the authenticated user.
- **Response**: `200 OK` with `{ "deleted_count": number }`.

### Export Vault
`POST /vault/export`

Export all user data (folders and accounts) in JSON format. Requires master password verification.
- **Request Body**: `UserDelete` (JSON)
- **Response**: `200 OK` with `VaultExport` (JSON).

### Get Vault Path
`GET /vault/config/path`

Get the path to the current vault file (e.g., `pact.pcty`).
- **Response**: `200 OK`
```json
{
  "path": "/path/to/pact.pcty"
}
```

### Set Vault Path
`POST /vault/config/path`

Change or initialize the vault file path. The path must end in `.pcty`.
On first run (no users yet) it can be called without a session; once users
exist it requires authentication because it resets the local database.
- **Request Body**: `VaultPath` (JSON)
```json
{
  "path": "/new/path/to/vault.pcty",
  "force_init": false
}
```

### Get Pending Entries (Inbox)
`GET /vault/pending-entries`

Retrieve accounts shared with the user that are waiting to be accepted.
- **Response**: `200 OK`
- **Body**: `Array<PendingAccount>`

### Delete Pending Entry
`DELETE /vault/pending-entries/:id`

Discard a pending entry. Only the owner can delete it (`404` otherwise).
- **Response**: `200 OK`.

### Get Pending Count
`GET /vault/pending-count/:username`

Get the number of pending entries for a user. Available before login so the lock
screen can show the badge.
- **Response**: `200 OK` with `{ "count": number }`.

### Upload Vault
`POST /vault/upload` *(multipart/form-data, authenticated)*

Upload a `.pcty` vault file. The server keeps only the base filename, rejects
non-`.pcty` files and validates the vault signature before linking it.
- **Response**: `200 OK`, `400 Bad Request`.

### Initialize Vault
`POST /vault/initialize` *(authenticated)*

Create a new empty vault. The provided `name` is sanitized (alphanumerics,
spaces, `-`, `_`).
- **Request Body**: `{ "name": "my_vault" }`
- **Response**: `200 OK` with `{ "path": "..." }`.

### Get TOTP Code
`GET /vault/totp/:id` *(authenticated)*

Generate the current 6-digit TOTP code for an account. The stored seed is
encrypted at rest; returns `------` if it cannot be processed.
- **Response**: `200 OK` with `{ "code": "123456" }`.

### Update Account Icon
`PATCH /vault/accounts/:id/icon` *(authenticated)*
- **Request Body**: `{ "icon_data": "data:image/png;base64,..." }`
- **Response**: `200 OK`.

### Proxy Icon
`GET /vault/proxy-icon?url=<https_url>` *(authenticated)*

Download a remote icon server-side. Resolves DNS and blocks private/loopback/
link-local/CGNAT addresses (SSRF protection), does not follow redirects and caps
the response at 1 MB.
- **Response**: `200 OK` with `{ "base64": "..." }`, `400 Bad Request`.

---

## Sync Management

### Run Sync
`POST /sync/run`

Calculate the synchronization plan (diff) between local and remote.
- **Response**: `200 OK` with `SyncPlan`.

### Apply Sync
`POST /sync/apply`

Execute the synchronization plan (push/pull changes).
- **Response**: `200 OK`

### Check Sync Status
`GET /sync/check`

Check whether there are remote changes to pull.
- **Response**: `200 OK` with `{ "has_changes": boolean }`.

### Test Sync Connection
`POST /sync/test`

Test connectivity with a remote database without persisting the configuration.
- **Request Body**: `SyncConfig` (JSON)
- **Response**: `200 OK` or `400 Bad Request` (the MySQL URL is never echoed).

### Get Sync Configuration
`GET /sync/config`

Retrieve the current synchronization settings (the password is never returned).
- **Response**: `200 OK` with `SyncConfig` or `404 Not Found` if not configured.

### Set Sync Configuration
`POST /sync/config`

Update the synchronization settings.
- **Request Body**: `SyncConfig` (JSON)
